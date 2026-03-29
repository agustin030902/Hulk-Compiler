use crate::parser::expression::WhileExpr;

use super::super::{
    backend::LlvmBackend,
    helper::state::{ValueRef, ValueType},
};

impl LlvmBackend {
    pub(super) fn emit_while_expr(&mut self, while_expr: &WhileExpr) -> Option<ValueRef> {
        let cond_label = self.next_label("while.cond");
        let body_label = self.next_label("while.body");
        let end_label = self.next_label("while.end");

        self.emit_body(format!("br label %{cond_label}"));
        self.emit_body(format!("{cond_label}:"));

        let condition = self.emit_expr(&while_expr.condition)?;
        if condition.value_type != ValueType::Bool {
            self.semantic_error("While condition must be Boolean");
            return None;
        }

        self.emit_body(format!(
            "br i1 {}, label %{body_label}, label %{end_label}",
            condition.repr
        ));
        self.emit_body(format!("{body_label}:"));
        let _ = self.emit_block_expr(&while_expr.body)?;
        self.emit_body(format!("br label %{cond_label}"));
        self.emit_body(format!("{end_label}:"));

        Some(ValueRef {
            value_type: ValueType::Unit,
            repr: "0".to_string(),
        })
    }
}
