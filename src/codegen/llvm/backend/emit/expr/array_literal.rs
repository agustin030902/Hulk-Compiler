use crate::parser::expression::ArrayLiteralExpr;

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_array_literal(
        &mut self,
        expr: &ArrayLiteralExpr,
    ) -> Option<ValueRef> {
        let Some(array_type_id) = self.type_ids.get("Array").copied() else {
            self.semantic_error("Type 'Array' is not declared.");
            return None;
        };

        let num_elements = expr.elements.len();

        let mut elem_values = Vec::with_capacity(num_elements);
        for elem_expr in &expr.elements {
            let val = self.emit_expr(elem_expr)?;
            elem_values.push(val);
        }

        let header_ptr = self.next_temp();
        self.emit_body(format!(
            "{header_ptr} = call i8* @malloc(i64 32)"
        ));

        let type_id_ptr = self.next_temp();
        self.emit_body(format!(
            "{type_id_ptr} = getelementptr i8, i8* {header_ptr}, i64 0"
        ));
        let type_id_ptr_i64 = self.next_temp();
        self.emit_body(format!(
            "{type_id_ptr_i64} = bitcast i8* {type_id_ptr} to i64*"
        ));
        self.emit_body(format!(
            "store i64 {array_type_id}, i64* {type_id_ptr_i64}"
        ));

        let length_ptr = self.next_temp();
        self.emit_body(format!(
            "{length_ptr} = getelementptr i8, i8* {header_ptr}, i64 8"
        ));
        let length_ptr_i64 = self.next_temp();
        self.emit_body(format!(
            "{length_ptr_i64} = bitcast i8* {length_ptr} to i64*"
        ));
        self.emit_body(format!(
            "store i64 {num_elements}, i64* {length_ptr_i64}"
        ));

        let data_bytes_i64 = self.next_temp();
        self.emit_body(format!(
            "{data_bytes_i64} = mul i64 {num_elements}, 8"
        ));
        let data_ptr = self.next_temp();
        self.emit_body(format!(
            "{data_ptr} = call i8* @malloc(i64 {data_bytes_i64})"
        ));

        let data_field_ptr = self.next_temp();
        self.emit_body(format!(
            "{data_field_ptr} = getelementptr i8, i8* {header_ptr}, i64 16"
        ));
        let data_field_ptr_i8p = self.next_temp();
        self.emit_body(format!(
            "{data_field_ptr_i8p} = bitcast i8* {data_field_ptr} to i8**"
        ));
        self.emit_body(format!(
            "store i8* {data_ptr}, i8** {data_field_ptr_i8p}"
        ));

        let cursor_ptr = self.next_temp();
        self.emit_body(format!(
            "{cursor_ptr} = getelementptr i8, i8* {header_ptr}, i64 24"
        ));
        let cursor_ptr_i64 = self.next_temp();
        self.emit_body(format!(
            "{cursor_ptr_i64} = bitcast i8* {cursor_ptr} to i64*"
        ));
        self.emit_body(format!("store i64 -1, i64* {cursor_ptr_i64}"));

        for (i, elem_val) in elem_values.iter().enumerate() {
            let i_i64 = self.next_temp();
            self.emit_body(format!("{i_i64} = add i64 {i}, 0"));
            let offset_i64 = self.next_temp();
            self.emit_body(format!("{offset_i64} = mul i64 {i_i64}, 8"));
            let elem_ptr = self.next_temp();
            self.emit_body(format!("{elem_ptr} = getelementptr i8, i8* {data_ptr}, i64 {offset_i64}"));

            match elem_val.value_type {
                ValueType::Double => {
                    let slot = self.next_temp();
                    self.emit_body(format!("{slot} = bitcast i8* {elem_ptr} to double*"));
                    self.emit_body(format!("store double {}, double* {slot}", elem_val.repr));
                }
                ValueType::Bool => {
                    let slot = self.next_temp();
                    self.emit_body(format!("{slot} = bitcast i8* {elem_ptr} to i64*"));
                    let ext = self.next_temp();
                    self.emit_body(format!("{ext} = zext i1 {} to i64", elem_val.repr));
                    self.emit_body(format!("store i64 {ext}, i64* {slot}"));
                }
                ValueType::ArrayPtr | ValueType::ArrayPtrOf(_) | _ => {
                    let slot = self.next_temp();
                    self.emit_body(format!("{slot} = bitcast i8* {elem_ptr} to i8**"));
                    self.emit_body(format!("store i8* {}, i8** {slot}", elem_val.repr));
                }
            }
        }

        let elem_type: crate::codegen::llvm::helper::state::ElementTag = if let Some(first) = elem_values.first() {
            first.value_type.into()
        } else {
            crate::codegen::llvm::helper::state::ElementTag::Double
        };

        Some(ValueRef {
            value_type: ValueType::ArrayPtrOf(elem_type),
            repr: header_ptr,
        })
    }
}
