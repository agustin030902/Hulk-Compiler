use crate::parser::expression::DestructiveAssignExpr;

use super::super::{backend::LlvmBackend, helper::state::ValueRef};

impl LlvmBackend {
    pub(super) fn emit_destructive_assign(
        &mut self,
        assign: &DestructiveAssignExpr,
    ) -> Option<ValueRef> {
        let Some(existing) = self.lookup_var(&assign.name) else {
            self.semantic_error(format!(
                "Variable '{}' is assigned before declaration. Declare it with 'let' first.",
                assign.name
            ));
            return None;
        };

        let value_ref = self.emit_expr(&assign.value)?;

        if value_ref.value_type != existing.value_type {
            self.semantic_error(format!(
                "Destructive assignment ':=' requires the same type. Variable '{}' is {:?} but expression is {:?}.",
                assign.name, existing.value_type, value_ref.value_type
            ));
            return None;
        }

        self.store_value_at(&existing.ptr_name, &value_ref);
        Some(value_ref)
    }
}
