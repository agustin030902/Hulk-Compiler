use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_method_call(
        &mut self,
        call: &MethodCallExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let receiver_type = self.check_expr(&call.receiver, source)?;

        // Los arreglos exponen un único método intrínseco: size(): Number.
        if let SemanticType::Array(_) = receiver_type {
            if call.method_name == "size" {
                if !call.args.is_empty() {
                    self.analyzer.push_semantic_error(
                        call.span,
                        source,
                        format!(
                            "Method 'size' expects 0 argument(s), but got {}.",
                            call.args.len()
                        ),
                    );
                    return None;
                }
                return Some(SemanticType::Number);
            }
            self.analyzer.push_semantic_error(
                call.method_name_span,
                source,
                format!(
                    "Method '{}' is not declared for arrays. Only 'size' is available.",
                    call.method_name
                ),
            );
            return None;
        }

        let SemanticType::Struct(receiver_raw) = receiver_type else {
            self.analyzer.push_type_error(
                call.span,
                source,
                format!(
                    "Method call expects a struct instance receiver, but got {}.",
                    receiver_type.display_name_with_table(&self.analyzer.type_table)
                ),
            );
            return None;
        };

        let receiver_id = TypeId(receiver_raw);

        if SymbolCollector::is_interface(self.analyzer, receiver_id) {
            let interface_methods = InterfaceChecker::collect_inherited_interface_methods(self.analyzer, receiver_id, receiver_id);
            let Some(interface_signature) = interface_methods
                .iter()
                .find(|m| m.name == call.method_name)
                .map(|m| FunctionSignature {
                    type_id: m.type_id.0,
                    param_names: vec![],
                    param_types: m.param_types.clone(),
                    return_type: m.return_type,
                })
            else {
                self.analyzer.push_semantic_error(
                    call.method_name_span,
                    source,
                    format!(
                        "Method '{}' is not declared in interface '{}'.",
                        call.method_name,
                        self.analyzer
                            .type_table
                            .get_struct(receiver_id)
                            .map(|info| info.name.clone())
                            .unwrap_or_default()
                    ),
                );
                return None;
            };

            if interface_signature.arity() != call.args.len() {
                self.analyzer.push_semantic_error(
                    call.span,
                    source,
                    format!(
                        "Method '{}' expects {} argument(s), but got {}.",
                        call.method_name,
                        interface_signature.arity(),
                        call.args.len()
                    ),
                );
                return None;
            }

            for (_index, arg) in call.args.iter().enumerate() {
                let _ = self.check_expr(arg, source);
            }

            if let Expr::Variable { name, .. } = call.receiver.as_ref() {
                let real_id = self
                    .analyzer
                    .interface_real_types
                    .get(name)
                    .copied()
                    .or_else(|| self.analyzer.lookup_param_real_type(name));
                if let Some(real_id) = real_id {
                    if let Some(real_signature) = self
                        .resolve_method_symbol_key(real_id, &call.method_name)
                        .or_else(|| self.resolve_method_symbol_key_in_structs(real_id, &call.method_name))
                        .and_then(|key| self.analyzer.functions.get(&key).cloned())
                    {
                        return Some(real_signature.return_type);
                    }
                }
            }

            return Some(interface_signature.return_type);
        }

        let Some(method_key) = self.resolve_method_symbol_key(receiver_id, &call.method_name)
        else {
            self.analyzer.push_semantic_error(
                call.method_name_span,
                source,
                format!(
                    "Method '{}' is not declared for this type.",
                    call.method_name
                ),
            );
            return None;
        };

        let Some(signature) = self.analyzer.functions.get(&method_key).cloned() else {
            self.analyzer.push_semantic_error(
                call.method_name_span,
                source,
                format!(
                    "Method '{}' is not declared for this type.",
                    call.method_name
                ),
            );
            return None;
        };

        if signature.arity() != call.args.len() {
            self.analyzer.push_semantic_error(
                call.span,
                source,
                format!(
                    "Method '{}' expects {} argument(s), but got {}.",
                    call.method_name,
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
                .get(&method_key)
                .and_then(|entry| entry.param_types.get(index).copied())
                .unwrap_or(SemanticType::Unknown);

            if arg_type == SemanticType::Unknown && expected_type != SemanticType::Unknown {
                let _ = TypeConstraintEngine::constrain_expr_type(self, arg, expected_type, source);
                continue;
            }

            if expected_type == SemanticType::Unknown && arg_type != SemanticType::Unknown {
                let _ = TypeConstraintEngine::constrain_function_param_type(
                    self,
                    &method_key,
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
                        "Method '{}' argument #{} expects {}, but got {}.",
                        call.method_name,
                        index + 1,
                        expected_type.display_name_with_table(&self.analyzer.type_table),
                        arg_type.display_name_with_table(&self.analyzer.type_table)
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
            .get(&method_key)
            .map(|entry| entry.return_type)
            .or(Some(SemanticType::Unknown))
    }

    fn resolve_method_symbol_key_in_structs(
        &self,
        receiver: TypeId,
        method_name: &str,
    ) -> Option<String> {
        let mut cursor = Some(receiver);
        while let Some(current) = cursor {
            if let Some(info) = self.analyzer.type_table.get_struct(current) {
                if info.methods.iter().any(|(name, _)| name == method_name) {
                    let key = SymbolCollector::method_symbol_key(current, method_name);
                    if self.analyzer.function_symbols.contains_key(&key) {
                        return Some(key);
                    }
                }
                cursor = info.parent;
            } else {
                return None;
            }
        }
        None
    }
}
