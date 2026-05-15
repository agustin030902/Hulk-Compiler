use crate::parser::expression::NewExpr;

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_new_expr(
        &mut self,
        new_expr: &NewExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let Some(type_signature) = self.types.get(&new_expr.type_name).cloned() else {
            self.push_semantic_error(
                new_expr.type_name_span,
                source,
                format!(
                    "Type '{}' is instantiated before declaration.",
                    new_expr.type_name
                ),
            );
            return None;
        };

        if type_signature.params.len() != new_expr.args.len() {
            self.push_semantic_error(
                new_expr.span,
                source,
                format!(
                    "Type '{}' expects {} constructor argument(s), but got {}.",
                    new_expr.type_name,
                    type_signature.params.len(),
                    new_expr.args.len()
                ),
            );
            return None;
        }

        let mut valid_call = true;
        for (index, arg) in new_expr.args.iter().enumerate() {
            let arg_type = self
                .check_expr(arg, source)
                .unwrap_or(SemanticType::Unknown);
            let expected_type = self
                .types
                .get(&new_expr.type_name)
                .and_then(|entry| entry.params.get(index))
                .map(|entry| entry.value_type)
                .unwrap_or(SemanticType::Unknown);

            if arg_type == SemanticType::Unknown && expected_type != SemanticType::Unknown {
                let _ = self.constrain_expr_type(arg, expected_type, source);
                continue;
            }

            if expected_type == SemanticType::Unknown && arg_type != SemanticType::Unknown {
                if let Some(entry) = self.types.get_mut(&new_expr.type_name)
                    && let Some(param) = entry.params.get_mut(index)
                {
                    param.value_type = arg_type;
                }
                continue;
            }

            if expected_type != SemanticType::Unknown
                && arg_type != SemanticType::Unknown
                && expected_type != arg_type
            {
                self.push_type_error(
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
                valid_call = false;
            }
        }

        if !valid_call {
            return None;
        }

        Some(SemanticType::Struct(type_signature.type_id))
    }
}
