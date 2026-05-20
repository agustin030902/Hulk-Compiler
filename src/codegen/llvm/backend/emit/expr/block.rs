use crate::parser::expression::BlockExpr;

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_block_expr(
        &mut self,
        block: &BlockExpr,
    ) -> Option<ValueRef> {
        self.push_scope();

        let mut last_value: Option<ValueRef> = None;
        for statement in &block.statements {
            if let Some(value) = self.emit_statement(statement) {
                last_value = Some(value);
            }
        }

        self.pop_scope();

        if block.statements.is_empty() {
            Some(ValueRef {
                value_type: ValueType::Unit,
                repr: "0".to_string(),
            })
        } else {
            last_value
        }
    }
}
