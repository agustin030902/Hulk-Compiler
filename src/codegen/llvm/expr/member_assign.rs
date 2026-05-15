use crate::parser::expression::MemberAssignExpr;

use super::super::{backend::LlvmBackend, helper::state::ValueRef};

impl LlvmBackend {
    pub(super) fn emit_member_assign(&mut self, assign: &MemberAssignExpr) -> Option<ValueRef> {
        let _ = self.emit_expr(&assign.instance)?;
        let _ = self.emit_expr(&assign.value)?;
        self.semantic_error(format!(
            "Member assignment '.{} :=' is not supported by this LLVM backend yet.",
            assign.member
        ));
        None
    }
}
