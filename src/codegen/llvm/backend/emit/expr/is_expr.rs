use crate::parser::expression::IsExpr;

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_is_expr(
        &mut self,
        is_expr: &IsExpr,
    ) -> Option<ValueRef> {
        let expr_value = self.emit_expr(&is_expr.expr)?;

        let Some(target_type_id) = self.type_ids.get(&is_expr.target_type).copied() else {
            self.semantic_error(format!(
                "Type '{}' is not declared for 'is' check.",
                is_expr.target_type
            ));
            return None;
        };

        match expr_value.value_type {
            ValueType::Null => {
                let result = self.next_temp();
                self.emit_body(format!("{result} = icmp eq i8* null, null"));
                Some(ValueRef {
                    value_type: ValueType::Bool,
                    repr: result,
                })
            }
            ValueType::Struct(actual_type_id) => {
                if actual_type_id == target_type_id {
                    let result = self.next_temp();
                    self.emit_body(format!(
                        "{result} = icmp eq i64 {actual_type_id}, {target_type_id}"
                    ));
                    return Some(ValueRef {
                        value_type: ValueType::Bool,
                        repr: result,
                    });
                }

                let tag_ptr = self.next_temp();
                self.emit_body(format!(
                    "{tag_ptr} = getelementptr i8, i8* {}, i64 0",
                    expr_value.repr
                ));
                let tag_ptr_i64 = self.next_temp();
                self.emit_body(format!(
                    "{tag_ptr_i64} = bitcast i8* {tag_ptr} to i64*"
                ));
                let tag = self.next_temp();
                self.emit_body(format!("{tag} = load i64, i64* {tag_ptr_i64}"));

                let result = self.next_temp();
                self.emit_body(format!(
                    "{result} = call i1 @hulk_is_subtype(i64 {tag}, i64 {target_type_id})"
                ));
                let bool_result = self.next_temp();
                self.emit_body(format!("{bool_result} = zext i1 {result} to i8"));

                Some(ValueRef {
                    value_type: ValueType::Bool,
                    repr: bool_result,
                })
            }
            _ => {
                let result = self.next_temp();
                self.emit_body(format!("{result} = add i8 0, 0"));
                Some(ValueRef {
                    value_type: ValueType::Bool,
                    repr: result,
                })
            }
        }
    }
}
