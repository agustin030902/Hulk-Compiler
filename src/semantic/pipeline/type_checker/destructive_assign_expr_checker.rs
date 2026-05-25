use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_destructive_assign(
        &mut self,
        assign: &DestructiveAssignExpr,
        source: &str,
    ) -> Option<SemanticType> {
        match &assign.target {
            AssignTarget::Variable { name, name_span } => {
                let Some((scope_index, existing)) = self.analyzer.lookup_with_scope_index(name)
                else {
                    self.analyzer.push_semantic_error(
                        *name_span,
                        source,
                        format!(
                            "Variable '{}' is assigned before declaration. Declare it with 'let' first.",
                            name
                        ),
                    );
                    return None;
                };

                if self.is_self_binding(name, scope_index) {
                    self.analyzer.push_semantic_error(
                        *name_span,
                        source,
                        "`self` is not a valid assignment target.".to_string(),
                    );
                    return None;
                }

                let value_type = self.check_expr(&assign.value, source)?;

                if existing != SemanticType::Unknown
                    && !self.types_compatible(existing, value_type)
                {
                    self.analyzer.push_type_error(
                        assign.span,
                        source,
                        format!(
                            "Destructive assignment ':=' requires the same type. Variable '{}' is {:?} but expression is {:?}.",
                            name, existing, value_type
                        ),
                    );
                    return None;
                }

                let assigned_type = match (existing, value_type) {
                    (SemanticType::Unknown, inferred) => inferred,
                    (SemanticType::Null, inferred) if inferred.is_nullable() => inferred,
                    (known, SemanticType::Null) if known.is_nullable() => known,
                    (known, _) => known,
                };

                self.analyzer
                    .assign_in_scope(scope_index, name.clone(), assigned_type);
                Some(assigned_type)
            }
            AssignTarget::Member {
                object,
                member,
                member_span,
                ..
            } => {
                let receiver_type = self.check_expr(object, source)?;
                let SemanticType::Struct(receiver_raw) = receiver_type else {
                    self.analyzer.push_type_error(
                        assign.span,
                        source,
                        format!(
                            "Member assignment expects a struct instance, but got {}.",
                            receiver_type.display_name()
                        ),
                    );
                    return None;
                };

                let receiver_id = TypeId(receiver_raw);
                if !self.can_access_private_field(receiver_id) {
                    self.analyzer.push_semantic_error(
                        *member_span,
                        source,
                        format!(
                            "Attribute '{}' is private and cannot be assigned from this context.",
                            member
                        ),
                    );
                    return None;
                }

                let Some(field_type_id) = self.lookup_field_type_id(receiver_id, member) else {
                    self.analyzer.push_semantic_error(
                        *member_span,
                        source,
                        format!("Attribute '{}' is not declared in this type.", member),
                    );
                    return None;
                };

                let expected_type =
                    TypeResolver::type_id_to_semantic_type(self.analyzer, field_type_id);
                let mut value_type = self.check_expr(&assign.value, source)?;

                if value_type == SemanticType::Unknown && expected_type != SemanticType::Unknown {
                    value_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &assign.value,
                        expected_type,
                        source,
                    );
                }

                if expected_type != SemanticType::Unknown
                    && value_type != SemanticType::Unknown
                    && !self.types_compatible(expected_type, value_type)
                {
                    self.analyzer.push_type_error(
                        assign.span,
                        source,
                        format!(
                            "Destructive assignment ':=' requires type {}, but expression is {}.",
                            expected_type.display_name(),
                            value_type.display_name()
                        ),
                    );
                    return None;
                }

                if expected_type != SemanticType::Unknown {
                    Some(expected_type)
                } else {
                    Some(value_type)
                }
            }
        }
    }
}
