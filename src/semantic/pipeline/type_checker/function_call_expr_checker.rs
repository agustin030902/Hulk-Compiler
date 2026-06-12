use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_function_call(
        &mut self,
        call: &FunctionCallExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let Some(symbol) = self.analyzer.function_symbols.get(&call.name).cloned() else {
            self.analyzer.push_semantic_error(
                call.name_span,
                source,
                format!("Function '{}' is called before declaration.", call.name),
            );
            return None;
        };

        if symbol.is_method() {
            self.analyzer.push_semantic_error(
                call.name_span,
                source,
                format!(
                    "Method '{}' requires a receiver and cannot be called as a global function.",
                    call.name
                ),
            );
            return None;
        }

        let Some(signature) = self.analyzer.functions.get(&call.name).cloned() else {
            self.analyzer.push_semantic_error(
                call.name_span,
                source,
                format!("Function '{}' is called before declaration.", call.name),
            );
            return None;
        };

        if signature.arity() != call.args.len() {
            self.analyzer.push_semantic_error(
                call.span,
                source,
                format!(
                    "Function '{}' expects {} argument(s), but got {}.",
                    call.name,
                    signature.arity(),
                    call.args.len()
                ),
            );
            return None;
        }

        let mut valid_call = true;
        let mut has_protocol_args = false;
        let mut arg_types: Vec<SemanticType> = Vec::with_capacity(call.args.len());

        for (index, arg) in call.args.iter().enumerate() {
            let arg_type = self
                .check_expr(arg, source)
                .unwrap_or(SemanticType::Unknown);
            arg_types.push(arg_type);
            let expected_type = self
                .analyzer
                .functions
                .get(&call.name)
                .and_then(|entry| entry.param_types.get(index).copied())
                .unwrap_or(SemanticType::Unknown);

            if arg_type == SemanticType::Unknown && expected_type != SemanticType::Unknown {
                let _ = TypeConstraintEngine::constrain_expr_type(self, arg, expected_type, source);
                continue;
            }

            if expected_type == SemanticType::Unknown && arg_type != SemanticType::Unknown {
                let _ = TypeConstraintEngine::constrain_function_param_type(
                    self,
                    &call.name,
                    index,
                    arg_type,
                    arg.span(),
                    source,
                );
                continue;
            }

            if expected_type != SemanticType::Unknown
                && arg_type != SemanticType::Unknown
                && !self.types_compatible(expected_type, arg_type)
            {
                self.analyzer.push_type_error(
                    arg.span(),
                    source,
                    format!(
                        "Function '{}' argument #{} expects {}, but got {}.",
                        call.name,
                        index + 1,
                        expected_type.display_name_with_table(&self.analyzer.type_table),
                        arg_type.display_name_with_table(&self.analyzer.type_table)
                    ),
                );
                valid_call = false;
            }

            if let (
                SemanticType::Struct(exp_raw),
                SemanticType::Struct(arg_raw),
            ) = (expected_type, arg_type) {
                let exp_id = TypeId(exp_raw);
                if SymbolCollector::is_protocol(self.analyzer, exp_id)
                    && exp_raw != arg_raw
                {
                    has_protocol_args = true;
                    if let Some(param_name) = signature.param_names.get(index) {
                        self.analyzer
                            .bind_param_real_type(param_name.clone(), TypeId(arg_raw));
                    }
                }
            }
        }

        if !valid_call {
            return None;
        }

        if has_protocol_args {
            self.analyzer.push_param_real_types();

            for (index, param_name) in signature.param_names.iter().enumerate() {
                if let Some(arg_type) = arg_types.get(index) {
                    if let (
                        SemanticType::Struct(exp_raw),
                        SemanticType::Struct(arg_raw),
                    ) = (signature.param_types.get(index).copied().unwrap_or(SemanticType::Unknown), *arg_type) {
                        let exp_id = TypeId(exp_raw);
                        if SymbolCollector::is_protocol(self.analyzer, exp_id)
                            && exp_raw != arg_raw
                        {
                            self.analyzer
                                .bind_param_real_type(param_name.clone(), TypeId(arg_raw));
                        }
                    }
                }
            }

            if let Some(function_decl) = self
                .analyzer
                .function_decls
                .get(&call.name)
                .cloned()
            {
                self.analyzer.push_scope();
                for (index, param) in function_decl.params.iter().enumerate() {
                    let param_type = if let Some(param_name) = signature.param_names.get(index) {
                        self.analyzer
                            .lookup_param_real_type(param_name)
                            .map(|tid| SemanticType::Struct(tid.0))
                            .unwrap_or_else(|| signature.param_types.get(index).copied().unwrap_or(SemanticType::Unknown))
                    } else {
                        signature.param_types.get(index).copied().unwrap_or(SemanticType::Unknown)
                    };
                    self.analyzer
                        .bind_current_scope(param.name.clone(), param_type);
                }
                let _ = self.check_expr(&function_decl.body, source);
                self.analyzer.pop_scope();
            }

            self.analyzer.pop_param_real_types();
        }

        Some(
            self.analyzer
                .functions
                .get(&call.name)
                .map(|entry| entry.return_type)
                .unwrap_or(SemanticType::Unknown),
        )
    }
}
