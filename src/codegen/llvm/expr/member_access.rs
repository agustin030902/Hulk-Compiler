use crate::parser::expression::MemberAccessExpr;

use super::super::{backend::LlvmBackend, helper::state::ValueRef};

impl LlvmBackend {
    pub(super) fn emit_member_access(&mut self, access: &MemberAccessExpr) -> Option<ValueRef> {
        let _ = self.emit_expr(&access.instance)?;
        self.semantic_error(format!(
            "Member access '.{}' is not supported by this LLVM backend yet.",
            access.member
        ));
        None
    }
}
