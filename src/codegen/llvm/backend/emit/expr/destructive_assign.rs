use crate::parser::expression::{AssignTarget, DestructiveAssignExpr};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_destructive_assign(
        &mut self,
        assign: &DestructiveAssignExpr,
    ) -> Option<ValueRef> {
        match &assign.target {
            AssignTarget::Variable { name, .. } => {
                let Some(existing) = self.lookup_var(name) else {
                    self.semantic_error(format!(
                        "Variable '{}' is assigned before declaration. Declare it with 'let' first.",
                        name
                    ));
                    return None;
                };

                let value_ref = self.emit_expr(&assign.value)?;

                if value_ref.value_type != existing.value_type {
                    self.semantic_error(format!(
                        "Destructive assignment ':=' requires the same type. Variable '{}' is {:?} but expression is {:?}.",
                        name, existing.value_type, value_ref.value_type
                    ));
                    return None;
                }

                self.store_value_at(&existing.ptr_name, &value_ref);
                Some(value_ref)
            }
            AssignTarget::Member { object, member, .. } => {
                let object_ref = self.emit_expr(object)?;
                let ValueType::Struct(type_id) = object_ref.value_type else {
                    self.semantic_error(format!(
                        "Member assignment expects a struct instance, but got {}.",
                        object_ref.value_type.display_name()
                    ));
                    return None;
                };

                let Some(field_layout) = self.field_layout(type_id, member).cloned() else {
                    self.semantic_error(format!(
                        "Attribute '{}' is not declared in this type.",
                        member
                    ));
                    return None;
                };

                let value_ref = self.emit_expr(&assign.value)?;
                if value_ref.value_type != field_layout.value_type {
                    self.semantic_error(format!(
                        "Destructive assignment ':=' requires type {}, but expression is {}.",
                        field_layout.value_type.display_name(),
                        value_ref.value_type.display_name()
                    ));
                    return None;
                }

                let field_ptr = self.emit_field_ptr(
                    &object_ref.repr,
                    field_layout.offset,
                    field_layout.value_type,
                );
                let llvm_type = field_layout.value_type.llvm_type();
                self.emit_body(format!(
                    "store {llvm_type} {}, {llvm_type}* {field_ptr}",
                    value_ref.repr
                ));

                Some(value_ref)
            }
        }
    }
}
