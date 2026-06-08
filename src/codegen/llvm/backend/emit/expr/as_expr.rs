use crate::parser::expression::AsExpr;

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_as_expr(
        &mut self,
        as_expr: &AsExpr,
    ) -> Option<ValueRef> {
        let expr_value = self.emit_expr(&as_expr.expr)?;

        let Some(target_type_id) = self.type_ids.get(&as_expr.target_type).copied() else {
            self.semantic_error(format!(
                "Type '{}' is not declared for 'as' cast.",
                as_expr.target_type
            ));
            return None;
        };

        match expr_value.value_type {
            ValueType::Null => Some(ValueRef {
                value_type: ValueType::Null,
                repr: "null".to_string(),
            }),
            ValueType::Struct(actual_type_id) => {
                if actual_type_id == target_type_id {
                    return Some(ValueRef {
                        value_type: ValueType::Struct(target_type_id),
                        repr: expr_value.repr,
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

                let is_subtype = self.next_temp();
                self.emit_body(format!(
                    "{is_subtype} = call i1 @hulk_is_subtype(i64 {tag}, i64 {target_type_id})"
                ));

                let ok_label = self.next_label("as.ok");
                let end_label = self.next_label("as.end");

                self.emit_body(format!(
                    "br i1 {is_subtype}, label %{ok_label}, label %{ok_label}"
                ));

                self.emit_body(format!("{ok_label}:"));
                self.emit_body(format!("br label %{end_label}"));

                self.emit_body(format!("{end_label}:"));

                Some(ValueRef {
                    value_type: ValueType::Struct(target_type_id),
                    repr: expr_value.repr,
                })
            }
            _ => Some(ValueRef {
                value_type: ValueType::Struct(target_type_id),
                repr: expr_value.repr,
            }),
        }
    }
}
