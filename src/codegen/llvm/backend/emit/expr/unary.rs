use crate::parser::expression::{UnaryExpr, UnaryOp};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_unary_expr(
        &mut self,
        unary: &UnaryExpr,
    ) -> Option<ValueRef> {
        let value = self.emit_expr(&unary.expr)?;

        match unary.op {
            UnaryOp::Neg => {
                if value.value_type != ValueType::Double {
                    self.semantic_error("Unary '-' only supports numeric values");
                    return None;
                }

                let result = self.next_temp();
                self.emit_body(format!("{result} = fneg double {}", value.repr));
                Some(ValueRef {
                    value_type: ValueType::Double,
                    repr: result,
                })
            }
            UnaryOp::Not => {
                if value.value_type != ValueType::Bool {
                    self.semantic_error("Unary '!' only supports boolean values");
                    return None;
                }

                let result = self.next_temp();
                self.emit_body(format!("{result} = xor i1 {}, true", value.repr));
                Some(ValueRef {
                    value_type: ValueType::Bool,
                    repr: result,
                })
            }
        }
    }
}
