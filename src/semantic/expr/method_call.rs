use crate::parser::expression::MethodCallExpr;

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_method_call(
        &mut self,
        call: &MethodCallExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let instance_type = self
            .check_expr(&call.instance, source)
            .unwrap_or(SemanticType::Unknown);

        let SemanticType::Struct(owner_type_id) = instance_type else {
            if instance_type != SemanticType::Unknown {
                self.push_type_error(
                    call.span,
                    source,
                    format!(
                        "Method call expects an object instance, but got {}.",
                        instance_type.display_name()
                    ),
                );
            }
            return None;
        };

        let Some((owner_type_name, signature)) =
            self.lookup_type_by_id(owner_type_id)
                .and_then(|(name, type_signature)| {
                    type_signature
                        .methods
                        .get(&call.method)
                        .cloned()
                        .map(|method_signature| (name.clone(), method_signature))
                })
        else {
            self.push_semantic_error(
                call.method_span,
                source,
                format!(
                    "Type '{}' has no method named '{}'.",
                    self.lookup_type_by_id(owner_type_id)
                        .map(|(name, _)| name.as_str())
                        .unwrap_or("<?>"),
                    call.method
                ),
            );
            return None;
        };

        if signature.arity() != call.args.len() {
            self.push_semantic_error(
                call.span,
                source,
                format!(
                    "Method '{}.{}' expects {} argument(s), but got {}.",
                    owner_type_name,
                    call.method,
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
                .lookup_method_signature(owner_type_id, &call.method)
                .and_then(|entry| entry.param_types.get(index).copied())
                .unwrap_or(SemanticType::Unknown);

            if arg_type == SemanticType::Unknown && expected_type != SemanticType::Unknown {
                let _ = self.constrain_expr_type(arg, expected_type, source);
                continue;
            }

            if expected_type == SemanticType::Unknown && arg_type != SemanticType::Unknown {
                let owner_name = self
                    .lookup_type_by_id(owner_type_id)
                    .map(|(name, _)| name.clone());
                if let Some(owner_name) = owner_name {
                    if let Some(type_entry) = self.types.get_mut(&owner_name)
                        && let Some(method_entry) = type_entry.methods.get_mut(&call.method)
                    {
                        method_entry.param_types[index] = arg_type;
                    }
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
                        "Method '{}.{}' argument #{} expects {}, but got {}.",
                        owner_type_name,
                        call.method,
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

        self.lookup_method_signature(owner_type_id, &call.method)
            .map(|entry| entry.return_type)
            .or(Some(SemanticType::Unknown))
    }
}
