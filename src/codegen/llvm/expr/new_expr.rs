use crate::parser::expression::NewExpr;

use super::super::{backend::LlvmBackend, helper::state::ValueRef};

impl LlvmBackend {
    pub(super) fn emit_new_expr(&mut self, new_expr: &NewExpr) -> Option<ValueRef> {
        for arg in &new_expr.args {
            let _ = self.emit_expr(arg)?;
        }

        self.semantic_error(format!(
            "Object instantiation 'new {}(...)' is not supported by this LLVM backend yet.",
            new_expr.type_name
        ));
        None
    }
}
