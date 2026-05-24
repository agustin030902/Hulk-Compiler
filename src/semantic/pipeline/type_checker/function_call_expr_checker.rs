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

        for (index, arg) in call.args.iter().enumerate() {
            let arg_type = self
                .check_expr(arg, source)
                .unwrap_or(SemanticType::Unknown);
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
                && !Self::types_compatible(expected_type, arg_type)
            {
                self.analyzer.push_type_error(
                    arg.span(),
                    source,
                    format!(
                        "Function '{}' argument #{} expects {}, but got {}.",
                        call.name,
                        index + 1,
                        expected_type.display_name(),
                        arg_type.display_name()
                    ),
                );
                valid_call = false;
            }
        }

        if !valid_call {
            return None;
        }

        self.analyzer
            .functions
            .get(&call.name)
            .map(|entry| entry.return_type)
            .or(Some(SemanticType::Unknown))
    }
}
