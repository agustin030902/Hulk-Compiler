use crate::parser::expression::MemberAssignExpr;

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_member_assign(
        &mut self,
        assign: &MemberAssignExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let instance_type = self
            .check_expr(&assign.instance, source)
            .unwrap_or(SemanticType::Unknown);

        let SemanticType::Struct(owner_type_id) = instance_type else {
            if instance_type != SemanticType::Unknown {
                self.push_type_error(
                    assign.span,
                    source,
                    format!(
                        "Member assignment expects an object instance, but got {}.",
                        instance_type.display_name()
                    ),
                );
            }
            return None;
        };

        let owner_type_name = self
            .lookup_type_by_id(owner_type_id)
            .map(|(name, _)| name.clone());
        let Some(owner_type_name) = owner_type_name else {
            self.push_semantic_error(
                assign.span,
                source,
                "Unknown object type in member assignment.".to_string(),
            );
            return None;
        };

        let expected_type = self
            .types
            .get(&owner_type_name)
            .and_then(|signature| signature.attributes.get(&assign.member))
            .map(|attribute| attribute.value_type);
        let Some(expected_type) = expected_type else {
            self.push_semantic_error(
                assign.member_span,
                source,
                format!(
                    "Type '{}' has no attribute named '{}'.",
                    owner_type_name, assign.member
                ),
            );
            return None;
        };

        let current_type_id = self.current_type_id();
        if current_type_id != Some(owner_type_id) {
            self.push_semantic_error(
                assign.member_span,
                source,
                format!(
                    "Attribute '{}.{}' is private and cannot be modified outside its type.",
                    owner_type_name, assign.member
                ),
            );
            return None;
        }

        let mut value_type = self
            .check_expr(&assign.value, source)
            .unwrap_or(SemanticType::Unknown);

        if value_type == SemanticType::Unknown && expected_type != SemanticType::Unknown {
            value_type = self.constrain_expr_type(&assign.value, expected_type, source);
        }

        if expected_type == SemanticType::Unknown && value_type != SemanticType::Unknown {
            if let Some(type_entry) = self.types.get_mut(&owner_type_name)
                && let Some(attribute_entry) = type_entry.attributes.get_mut(&assign.member)
            {
                attribute_entry.value_type = value_type;
            }
        }

        if expected_type != SemanticType::Unknown
            && value_type != SemanticType::Unknown
            && expected_type != value_type
        {
            self.push_type_error(
                assign.span,
                source,
                format!(
                    "Attribute '{}.{}' expects {}, but assignment is {}.",
                    owner_type_name,
                    assign.member,
                    expected_type.display_name(),
                    value_type.display_name()
                ),
            );
            return None;
        }

        Some(if value_type == SemanticType::Unknown {
            expected_type
        } else {
            value_type
        })
    }
}
