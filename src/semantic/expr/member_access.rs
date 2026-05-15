use crate::parser::expression::MemberAccessExpr;

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_member_access(
        &mut self,
        access: &MemberAccessExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let instance_type = self
            .check_expr(&access.instance, source)
            .unwrap_or(SemanticType::Unknown);

        let SemanticType::Struct(owner_type_id) = instance_type else {
            if instance_type != SemanticType::Unknown {
                self.push_type_error(
                    access.span,
                    source,
                    format!(
                        "Member access expects an object instance, but got {}.",
                        instance_type.display_name()
                    ),
                );
            }
            return None;
        };

        let Some((owner_type_name, owner_type_signature)) = self.lookup_type_by_id(owner_type_id)
        else {
            self.push_semantic_error(
                access.span,
                source,
                "Unknown object type in member access.".to_string(),
            );
            return None;
        };

        let Some(attribute) = owner_type_signature.attributes.get(&access.member) else {
            self.push_semantic_error(
                access.member_span,
                source,
                format!(
                    "Type '{}' has no attribute named '{}'.",
                    owner_type_name, access.member
                ),
            );
            return None;
        };

        let current_type_id = self.current_type_id();
        if current_type_id != Some(owner_type_id) {
            self.push_semantic_error(
                access.member_span,
                source,
                format!(
                    "Attribute '{}.{}' is private and cannot be accessed outside its type.",
                    owner_type_name, access.member
                ),
            );
            return None;
        }

        Some(attribute.value_type)
    }
}
