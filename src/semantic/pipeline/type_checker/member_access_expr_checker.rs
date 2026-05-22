use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_member_access(
        &mut self,
        access: &crate::parser::expression::MemberAccessExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let object_type = self.check_expr(&access.object, source)?;
        let SemanticType::Struct(type_raw_id) = object_type else {
            self.analyzer.push_type_error(
                access.span,
                source,
                format!(
                    "Member access expects a struct instance, but got {}.",
                    object_type.display_name()
                ),
            );
            return None;
        };

        let receiver = TypeId(type_raw_id);
        let Some(field_type_id) = self.lookup_field_type_id(receiver, &access.member) else {
            self.analyzer.push_semantic_error(
                access.member_span,
                source,
                format!(
                    "Attribute '{}' is not declared in this type.",
                    access.member
                ),
            );
            return None;
        };

        if !self.can_access_private_field(receiver) {
            self.analyzer.push_semantic_error(
                access.member_span,
                source,
                format!(
                    "Attribute '{}' is private and cannot be accessed from this context.",
                    access.member
                ),
            );
            return None;
        }

        Some(TypeResolver::type_id_to_semantic_type(
            self.analyzer,
            field_type_id,
        ))
    }
}
