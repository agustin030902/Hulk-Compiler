use crate::parser::expression::IfExpr;
use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_if_expr(
        &mut self,
        if_expr: &IfExpr,
    ) -> Option<ValueRef> {
        let condition = self.emit_expr(&if_expr.condition)?;

        if condition.value_type != ValueType::Bool {
            self.semantic_error("If condition must be Boolean");
            return None;
        }

        let then_label = self.next_label("if.then");
        let end_label = self.next_label("if.end");

        let next_label = if if_expr.elif_branches.is_empty() {
            self.next_label("if.else")
        } else {
            self.next_label("if.elif")
        };

        self.emit_body(format!(
            "br i1 {}, label %{then_label}, label %{next_label}",
            condition.repr
        ));

        // =========================
        // THEN
        // =========================

        self.emit_body(format!("{then_label}:"));

        let then_value = self.emit_expr(&if_expr.then_branch)?;
        let result_type = then_value.value_type;

        if result_type != ValueType::Unit {
            let then_result_repr = then_value.repr.clone();

            // IMPORTANT:
            // capture the REAL terminal block
            let then_terminal_label = self.current_block.clone();

            self.emit_body(format!("br label %{end_label}"));

            let mut branch_results =
                vec![(then_result_repr, then_terminal_label)];

            // =========================
            // ELIFS
            // =========================

            let mut current_next_label = next_label;

            for (idx, elif_branch) in if_expr.elif_branches.iter().enumerate() {
                self.emit_body(format!("{current_next_label}:"));

                let elif_condition =
                    self.emit_expr(&elif_branch.condition)?;

                if elif_condition.value_type != ValueType::Bool {
                    self.semantic_error("Elif condition must be Boolean");
                    return None;
                }

                let elif_then_label =
                    self.next_label("if.elif.then");

                let elif_next_label =
                    if idx == if_expr.elif_branches.len() - 1 {
                        self.next_label("if.else")
                    } else {
                        self.next_label("if.elif")
                    };

                self.emit_body(format!(
                    "br i1 {}, label %{elif_then_label}, label %{elif_next_label}",
                    elif_condition.repr
                ));

                self.emit_body(format!("{elif_then_label}:"));

                let elif_value =
                    self.emit_expr(&elif_branch.body)?;

                if elif_value.value_type != result_type {
                    self.semantic_error(format!(
                        "Elif branch returns {} but expected {}",
                        self.type_name_for_value_type(elif_value.value_type),
                        self.type_name_for_value_type(result_type)
                    ));
                    return None;
                }

                let elif_result_repr = elif_value.repr.clone();

                // IMPORTANT:
                // capture REAL terminal block
                let elif_terminal_label =
                    self.current_block.clone();

                self.emit_body(format!("br label %{end_label}"));

                branch_results.push((
                    elif_result_repr,
                    elif_terminal_label,
                ));

                current_next_label = elif_next_label;
            }

            // =========================
            // ELSE
            // =========================

            self.emit_body(format!("{current_next_label}:"));

            let else_value =
                self.emit_expr(&if_expr.else_branch)?;

            if else_value.value_type != result_type {
                self.semantic_error(format!(
                    "Else branch returns {} but expected {}",
                    self.type_name_for_value_type(else_value.value_type),
                    self.type_name_for_value_type(result_type)
                ));
                return None;
            }

            let else_result_repr = else_value.repr.clone();

            // IMPORTANT:
            // capture REAL terminal block
            let else_terminal_label =
                self.current_block.clone();

            self.emit_body(format!("br label %{end_label}"));

            branch_results.push((
                else_result_repr,
                else_terminal_label,
            ));

            // =========================
            // MERGE
            // =========================

            self.emit_body(format!("{end_label}:"));

            let result = self.next_temp();
            let llvm_type = result_type.llvm_type();

            let phi_args = branch_results
                .iter()
                .map(|(val_repr, label_repr)| {
                    format!("[ {}, %{} ]", val_repr, label_repr)
                })
                .collect::<Vec<_>>()
                .join(", ");

            self.emit_body(format!(
                "{} = phi {} {}",
                result,
                llvm_type,
                phi_args
            ));

            Some(ValueRef {
                value_type: result_type,
                repr: result,
            })
        } else {
            self.emit_body(format!("br label %{end_label}"));

            let mut current_next_label = next_label;

            for (idx, elif_branch) in if_expr.elif_branches.iter().enumerate() {
                self.emit_body(format!("{current_next_label}:"));

                let elif_condition =
                    self.emit_expr(&elif_branch.condition)?;

                if elif_condition.value_type != ValueType::Bool {
                    self.semantic_error("Elif condition must be Boolean");
                    return None;
                }

                let elif_then_label =
                    self.next_label("if.elif.then");

                let elif_next_label =
                    if idx == if_expr.elif_branches.len() - 1 {
                        self.next_label("if.else")
                    } else {
                        self.next_label("if.elif")
                    };

                self.emit_body(format!(
                    "br i1 {}, label %{elif_then_label}, label %{elif_next_label}",
                    elif_condition.repr
                ));

                self.emit_body(format!("{elif_then_label}:"));

                self.emit_expr(&elif_branch.body)?;

                self.emit_body(format!("br label %{end_label}"));

                current_next_label = elif_next_label;
            }

            self.emit_body(format!("{current_next_label}:"));

            self.emit_expr(&if_expr.else_branch)?;

            self.emit_body(format!("br label %{end_label}"));

            self.emit_body(format!("{end_label}:"));

            Some(then_value)
        }
    }
}