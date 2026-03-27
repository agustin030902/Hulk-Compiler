use crate::parser::expression::BlockExpr;

use super::super::{backend::LlvmBackend, helper::state::ValueRef};

impl LlvmBackend {
    pub(super) fn emit_block_expr(&mut self, block: &BlockExpr) -> Option<ValueRef> {
        self.push_scope();

        let mut last_value: Option<ValueRef> = None;
        for statement in &block.statements {
            if let Some(value) = self.emit_statement(statement) {
                last_value = Some(value);
            }
        }

        self.pop_scope();

        if let Some(value) = last_value {
            Some(value)
        } else {
            self.semantic_error("Block expression must produce a value");
            None
        }
    }
}
