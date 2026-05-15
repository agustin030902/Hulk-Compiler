use crate::parser::expression::MethodCallExpr;

use super::super::{backend::LlvmBackend, helper::state::ValueRef};

impl LlvmBackend {
    pub(super) fn emit_method_call(&mut self, call: &MethodCallExpr) -> Option<ValueRef> {
        let _ = self.emit_expr(&call.instance)?;
        for arg in &call.args {
            let _ = self.emit_expr(arg)?;
        }

        self.semantic_error(format!(
            "Method call '{}(...)' is not supported by this LLVM backend yet.",
            call.method
        ));
        None
    }
}
