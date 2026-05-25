use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_new_expr(
        &mut self,
        new_expr: &NewExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let Some(type_id) = self.analyzer.type_symbols.get(&new_expr.type_name).copied() else {
            self.analyzer.push_semantic_error(
                new_expr.type_name_span,
                source,
                format!("Type '{}' is not declared.", new_expr.type_name),
            );
            return None;
        };

        let constructor_params = self
            .analyzer
            .type_table
            .get_struct(type_id)
            .map(|info| info.constructor_params.clone())
            .unwrap_or_default();

        if constructor_params.len() != new_expr.args.len() {
            self.analyzer.push_semantic_error(
                new_expr.span,
                source,
                format!(
                    "Type '{}' constructor expects {} argument(s), but got {}.",
                    new_expr.type_name,
                    constructor_params.len(),
                    new_expr.args.len()
                ),
            );
            return None;
        }

        for (index, arg) in new_expr.args.iter().enumerate() {
            let arg_type = self
                .check_expr(arg, source)
                .unwrap_or(SemanticType::Unknown);
            let expected_type = constructor_params
                .get(index)
                .map(|(_, param_type_id)| {
                    TypeResolver::type_id_to_semantic_type(self.analyzer, *param_type_id)
                })
                .unwrap_or(SemanticType::Unknown);

            if arg_type == SemanticType::Unknown && expected_type != SemanticType::Unknown {
                let _ = TypeConstraintEngine::constrain_expr_type(self, arg, expected_type, source);
                continue;
            }

            if expected_type == SemanticType::Unknown && arg_type != SemanticType::Unknown {
                let inferred_type_id =
                    TypeResolver::semantic_type_to_type_id(self.analyzer, arg_type);
                if let Some(info) = self.analyzer.type_table.get_struct_mut(type_id)
                    && let Some((_, entry_type_id)) = info.constructor_params.get_mut(index)
                {
                    *entry_type_id = inferred_type_id;
                }
                continue;
            }

            if expected_type != SemanticType::Unknown
                && arg_type != SemanticType::Unknown
                && !self.is_assignable(expected_type, arg_type)
            {
                self.analyzer.push_type_error(
                    arg.span(),
                    source,
                    format!(
                        "Type '{}' constructor argument #{} expects {}, but got {}.",
                        new_expr.type_name,
                        index + 1,
                        expected_type.display_name(),
                        arg_type.display_name()
                    ),
                );
                return None;
            }
        }

        Some(SemanticType::Struct(type_id.0))
    }
}
