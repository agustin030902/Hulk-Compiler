use crate::parser::expression::MethodCallExpr;

use super::super::{SemanticType, analyzer::SemanticAnalyzer, helper::TypeId};

impl SemanticAnalyzer {
    pub(super) fn check_method_call(
        &mut self,
        call: &MethodCallExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let receiver_type = self.check_expr(&call.receiver, source)?;
        let SemanticType::Struct(receiver_raw) = receiver_type else {
            self.push_type_error(
                call.span,
                source,
                format!(
                    "Method call expects a struct instance receiver, but got {}.",
                    receiver_type.display_name()
                ),
            );
            return None;
        };

        let receiver_id = TypeId(receiver_raw);
        let Some(method_key) = self.resolve_method_symbol_key(receiver_id, &call.method_name)
        else {
            self.push_semantic_error(
                call.method_name_span,
                source,
                format!(
                    "Method '{}' is not declared for this type.",
                    call.method_name
                ),
            );
            return None;
        };

        let Some(signature) = self.functions.get(&method_key).cloned() else {
            self.push_semantic_error(
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
            self.push_semantic_error(
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
                .functions
                .get(&method_key)
                .and_then(|entry| entry.param_types.get(index).copied())
                .unwrap_or(SemanticType::Unknown);

            if arg_type == SemanticType::Unknown && expected_type != SemanticType::Unknown {
                let _ = self.constrain_expr_type(arg, expected_type, source);
                continue;
            }

            if expected_type == SemanticType::Unknown && arg_type != SemanticType::Unknown {
                let _ = self.constrain_function_param_type(
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
                && expected_type != arg_type
            {
                self.push_type_error(
                    arg.span(),
                    source,
                    format!(
                        "Method '{}' argument #{} expects {}, but got {}.",
                        call.method_name,
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

        self.functions
            .get(&method_key)
            .map(|entry| entry.return_type)
            .or(Some(SemanticType::Unknown))
    }
}
