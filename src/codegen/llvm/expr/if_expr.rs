use crate::parser::expression::IfExpr;

use super::super::{
    backend::LlvmBackend,
    helper::state::{ValueRef, ValueType},
};

impl LlvmBackend {
    pub(super) fn emit_if_expr(&mut self, if_expr: &IfExpr) -> Option<ValueRef> {
        let condition = self.emit_expr(&if_expr.condition)?;
        if condition.value_type != ValueType::Bool {
            self.semantic_error("If condition must be Boolean");
            return None;
        }

        let then_label = self.next_label("if.then");
        let end_label = self.next_label("if.end");

        // First elif or else?
        let next_label = if if_expr.elif_branches.is_empty() {
            self.next_label("if.else")
        } else {
            self.next_label("if.elif")
        };

        self.emit_body(format!(
            "br i1 {}, label %{then_label}, label %{next_label}",
            condition.repr
        ));

        // Emit then branch
        self.emit_body(format!("{then_label}:"));
        let then_value = self.emit_expr(&if_expr.then_branch)?;
        let result_type = then_value.value_type;
        self.emit_body(format!("br label %{end_label}"));

        // Emit elif branches
        let mut current_next_label = next_label;
        for (idx, elif_branch) in if_expr.elif_branches.iter().enumerate() {
            self.emit_body(format!("{current_next_label}:"));

            let elif_condition = self.emit_expr(&elif_branch.condition)?;
            if elif_condition.value_type != ValueType::Bool {
                self.semantic_error("Elif condition must be Boolean");
                return None;
            }

            let elif_then_label = self.next_label("if.elif.then");
            let elif_next_label = if idx == if_expr.elif_branches.len() - 1 {
                self.next_label("if.else")
            } else {
                self.next_label("if.elif")
            };

            self.emit_body(format!(
                "br i1 {}, label %{elif_then_label}, label %{elif_next_label}",
                elif_condition.repr
            ));

            self.emit_body(format!("{elif_then_label}:"));
            let elif_value = self.emit_expr(&elif_branch.body)?;
            if elif_value.value_type != result_type {
                self.semantic_error(format!(
                    "Elif branch returns {} but expected {}",
                    elif_value.value_type.display_name(),
                    result_type.display_name()
                ));
                return None;
            }
            self.emit_body(format!("br label %{end_label}"));
            current_next_label = elif_next_label;
        }

        // Emit else branch
        self.emit_body(format!("{current_next_label}:"));
        let else_value = self.emit_expr(&if_expr.else_branch)?;
        if else_value.value_type != result_type {
            self.semantic_error(format!(
                "Else branch returns {} but expected {}",
                else_value.value_type.display_name(),
                result_type.display_name()
            ));
            return None;
        }
        self.emit_body(format!("br label %{end_label}"));

        // End label
        self.emit_body(format!("{end_label}:"));

        Some(then_value)
    }
}
