use crate::parser::expression::MemberAccessExpr;

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_member_access(
        &mut self,
        access: &MemberAccessExpr,
    ) -> Option<ValueRef> {
        let object = self.emit_expr(&access.object)?;
        let ValueType::Struct(type_id) = object.value_type else {
            self.semantic_error(format!(
                "Member access expects a struct instance, but got {}.",
                object.value_type.display_name()
            ));
            return None;
        };

        let Some(field_layout) = self.field_layout(type_id, &access.member).cloned() else {
            self.semantic_error(format!(
                "Attribute '{}' is not declared in this type.",
                access.member
            ));
            return None;
        };

        let field_ptr =
            self.emit_field_ptr(&object.repr, field_layout.offset, field_layout.value_type);
        let loaded = self.next_temp();
        let llvm_type = field_layout.value_type.llvm_type();
        self.emit_body(format!(
            "{loaded} = load {llvm_type}, {llvm_type}* {field_ptr}"
        ));

        Some(ValueRef {
            value_type: field_layout.value_type,
            repr: loaded,
        })
    }
}
