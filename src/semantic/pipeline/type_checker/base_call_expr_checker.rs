use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_base_call(
        &mut self,
        call: &BaseCallExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let Some(receiver_id) = self.analyzer.current_method_receiver else {
            self.analyzer.push_semantic_error(
                call.span,
                source,
                "Base call 'base()' can only be used inside a type method.".to_string(),
            );
            return None;
        };

        let Some(method_name) = self.analyzer.current_method_name.clone() else {
            self.analyzer.push_semantic_error(
                call.span,
                source,
                "Base call 'base()' can only be used inside a type method.".to_string(),
            );
            return None;
        };

        let Some(parent_signature) =
            SymbolCollector::find_method_in_parent(self.analyzer, receiver_id, &method_name)
        else {
            let type_name = self
                .analyzer
                .type_table
                .get_struct(receiver_id)
                .map(|info| info.name.clone())
                .unwrap_or_default();
            self.analyzer.push_semantic_error(
                call.span,
                source,
                format!(
                    "Base call 'base()' failed: type '{}' has no parent type with method '{}'.",
                    type_name, method_name
                ),
            );
            return None;
        };

        if parent_signature.arity() != call.args.len() {
            self.analyzer.push_semantic_error(
                call.span,
                source,
                format!(
                    "Base call 'base()' expects {} argument(s), but got {}.",
                    parent_signature.arity(),
                    call.args.len()
                ),
            );
            return None;
        }

        for (index, arg) in call.args.iter().enumerate() {
            let arg_type = self.check_expr(arg, source).unwrap_or(SemanticType::Unknown);
            let expected_type = parent_signature.param_types.get(index).copied().unwrap_or(SemanticType::Unknown);

            if expected_type != SemanticType::Unknown
                && arg_type != SemanticType::Unknown
                && !self.types_compatible(expected_type, arg_type)
            {
                self.analyzer.push_type_error(
                    arg.span(),
                    source,
                    format!(
                        "Base call argument #{} expects {}, but got {}.",
                        index + 1,
                        expected_type.display_name_with_table(&self.analyzer.type_table),
                        arg_type.display_name_with_table(&self.analyzer.type_table)
                    ),
                );
            }
        }

        Some(parent_signature.return_type)
    }
}
