use crate::parser::expression::{ArrayIndexExpr, Expr};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    fn infer_array_index_result_type(&self, object: &Expr) -> ValueType {
        if let Expr::ArrayIndex(inner) = object {
            match &inner.object.as_ref() {
                Expr::Variable { name, .. } => {
                    if let Some(info) = self.lookup_var(name) {
                        return match info.value_type {
                            ValueType::ArrayPtrOf(crate::codegen::llvm::helper::state::ElementTag::Array) => {
                                ValueType::Double
                            }
                            ValueType::ArrayPtr => ValueType::Double,
                            _ => info.value_type,
                        };
                    }
                }
                Expr::ArrayIndex(_) => return self.infer_array_index_result_type(&inner.object),
                _ => {}
            }
        }
        ValueType::Double
    }

    pub(in crate::codegen::llvm) fn emit_array_index(
        &mut self,
        expr: &ArrayIndexExpr,
    ) -> Option<ValueRef> {
        let object = self.emit_expr(&expr.object)?;
        if !object.value_type.is_array() {
            self.semantic_error(format!(
                "Array index expects an Array, but got {}.",
                self.type_name_for_value_type(object.value_type)
            ));
            return None;
        }

        let index = self.emit_expr(&expr.index)?;
        if index.value_type != ValueType::Double {
            self.semantic_error("Array index must be a Number.");
            return None;
        }

        let index_as_i64 = self.next_temp();
        self.emit_body(format!(
            "{index_as_i64} = fptosi double {0} to i64",
            index.repr
        ));

        let length_ptr = self.next_temp();
        self.emit_body(format!(
            "{length_ptr} = getelementptr i8, i8* {0}, i64 8",
            object.repr
        ));
        let length_ptr_i64 = self.next_temp();
        self.emit_body(format!(
            "{length_ptr_i64} = bitcast i8* {length_ptr} to i64*"
        ));
        let length_i64 = self.next_temp();
        self.emit_body(format!(
            "{length_i64} = load i64, i64* {length_ptr_i64}"
        ));

        let bounds_ok = self.next_temp();
        self.emit_body(format!(
            "{bounds_ok} = icmp slt i64 {index_as_i64}, {length_i64}"
        ));
        let bounds_label = self.next_label("array.bounds");
        let error_label = self.next_label("array.oob");
        let cont_label = self.next_label("array.cont");
        self.emit_body(format!(
            "br i1 {bounds_ok}, label %{bounds_label}, label %{error_label}"
        ));

        self.emit_body(format!("{error_label}:"));
        let oob_msg_ptr = self.next_string_name();
        let oob_msg_bytes = 29;
        self.emit_global(format!(
            "{oob_msg_ptr} = private unnamed_addr constant [{oob_msg_bytes} x i8] c\"Array index out of bounds\\0A\\00\""
        ));
        let oob_gep = self.next_temp();
        self.emit_body(format!(
            "{oob_gep} = getelementptr inbounds [{oob_msg_bytes} x i8], [{oob_msg_bytes} x i8]* {oob_msg_ptr}, i64 0, i64 0"
        ));
        self.emit_body(format!("call i32 @printf(i8* {oob_gep})"));
        self.emit_body(format!("br label %{cont_label}"));

        self.emit_body(format!("{bounds_label}:"));
        let data_field_ptr = self.next_temp();
        self.emit_body(format!(
            "{data_field_ptr} = getelementptr i8, i8* {0}, i64 16",
            object.repr
        ));
        let data_field_ptr_i8p = self.next_temp();
        self.emit_body(format!(
            "{data_field_ptr_i8p} = bitcast i8* {data_field_ptr} to i8**"
        ));
        let data_ptr = self.next_temp();
        self.emit_body(format!(
            "{data_ptr} = load i8*, i8** {data_field_ptr_i8p}"
        ));

        let offset_i64 = self.next_temp();
        self.emit_body(format!(
            "{offset_i64} = mul i64 {index_as_i64}, 8"
        ));
        let elem_ptr = self.next_temp();
        self.emit_body(format!(
            "{elem_ptr} = getelementptr i8, i8* {data_ptr}, i64 {offset_i64}"
        ));

        let result_type = match &object.value_type {
            ValueType::ArrayPtrOf(tag) => tag.to_value_type(),
            _ => self.infer_array_index_result_type(&expr.object),
        };

        let elem_val = match result_type {
            ValueType::Double => {
                let slot = self.next_temp();
                self.emit_body(format!("{slot} = bitcast i8* {elem_ptr} to double*"));
                let loaded = self.next_temp();
                self.emit_body(format!("{loaded} = load double, double* {slot}"));
                loaded
            }
            ValueType::Bool => {
                let slot = self.next_temp();
                self.emit_body(format!("{slot} = bitcast i8* {elem_ptr} to i64*"));
                let raw_i64 = self.next_temp();
                self.emit_body(format!("{raw_i64} = load i64, i64* {slot}"));
                let truncated = self.next_temp();
                self.emit_body(format!("{truncated} = trunc i64 {raw_i64} to i1"));
                truncated
            }
            _ => {
                let slot = self.next_temp();
                self.emit_body(format!("{slot} = bitcast i8* {elem_ptr} to i8**"));
                let loaded = self.next_temp();
                self.emit_body(format!("{loaded} = load i8*, i8** {slot}"));
                loaded
            }
        };

        self.emit_body(format!("br label %{cont_label}"));

        self.emit_body(format!("{cont_label}:"));

        Some(ValueRef {
            value_type: result_type,
            repr: elem_val,
        })
    }
}
