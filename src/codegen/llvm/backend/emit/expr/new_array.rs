use crate::parser::expression::NewArrayExpr;

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_new_array(
        &mut self,
        expr: &NewArrayExpr,
    ) -> Option<ValueRef> {
        let Some(array_type_id) = self.type_ids.get("Array").copied() else {
            self.semantic_error("Type 'Array' is not declared.");
            return None;
        };

        let mut size_values = Vec::with_capacity(expr.sizes.len());
        for size_expr in &expr.sizes {
            let size_val = self.emit_expr(size_expr)?;
            if size_val.value_type != ValueType::Double {
                self.semantic_error("Array size must be a Number.");
                return None;
            }
            size_values.push(size_val);
        }

        let num_elements = if let Some(first_size) = size_values.first() {
            first_size.repr.clone()
        } else {
            "0.0".to_string()
        };

        let num_elements_i64 = self.next_temp();
        self.emit_body(format!(
            "{num_elements_i64} = fptosi double {num_elements} to i64"
        ));

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
            "store i64 {num_elements_i64}, i64* {length_ptr_i64}"
        ));

        let data_bytes_i64 = self.next_temp();
        self.emit_body(format!(
            "{data_bytes_i64} = mul i64 {num_elements_i64}, 8"
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

        if let Some(lambda) = &expr.initializer {
            for i in 0..size_values.len() {
                self.push_scope();

                let index_val = self.emit_literal(&crate::parser::expression::Literal::Float(i as f64))?;
                let param_info = self.allocate_storage(&index_val);
                self.bind_current_scope(lambda.param_name.clone(), param_info);

                let elem_val = self.emit_expr(&lambda.body)?;

                let index_as_i64 = self.next_temp();
                let index_as_f64 = self.emit_literal(&crate::parser::expression::Literal::Float(i as f64))?;
                self.emit_body(format!(
                    "{index_as_i64} = fptosi double {0} to i64",
                    index_as_f64.repr
                ));
                let offset_i64 = self.next_temp();
                self.emit_body(format!(
                    "{offset_i64} = mul i64 {index_as_i64}, 8"
                ));
                let elem_ptr = self.next_temp();
                self.emit_body(format!(
                    "{elem_ptr} = getelementptr i8, i8* {data_ptr}, i64 {offset_i64}"
                ));
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

                self.pop_scope();
            }
        } else {
            for i in 0..size_values.len() {
                let index_as_i64 = self.next_temp();
                let index_as_f64 = self.emit_literal(&crate::parser::expression::Literal::Float(i as f64))?;
                self.emit_body(format!(
                    "{index_as_i64} = fptosi double {0} to i64",
                    index_as_f64.repr
                ));
                let offset_i64 = self.next_temp();
                self.emit_body(format!(
                    "{offset_i64} = mul i64 {index_as_i64}, 8"
                ));
                let elem_ptr = self.next_temp();
                self.emit_body(format!(
                    "{elem_ptr} = getelementptr i8, i8* {data_ptr}, i64 {offset_i64}"
                ));
                let slot = self.next_temp();
                self.emit_body(format!(
                    "{slot} = bitcast i8* {elem_ptr} to i8**"
                ));
                self.emit_body(format!(
                    "store i8* null, i8** {slot}"
                ));
            }
        }

        let elem_tag = match expr.type_name.as_str() {
            "Number" => crate::codegen::llvm::helper::state::ElementTag::Double,
            "Boolean" => crate::codegen::llvm::helper::state::ElementTag::Bool,
            "String" => crate::codegen::llvm::helper::state::ElementTag::StringPtr,
            "Unit" => crate::codegen::llvm::helper::state::ElementTag::Unit,
            _ => {
                if let Some(tid) = self.type_ids.get(&expr.type_name).copied() {
                    crate::codegen::llvm::helper::state::ElementTag::Struct(tid)
                } else {
                    crate::codegen::llvm::helper::state::ElementTag::Array
                }
            }
        };

        Some(ValueRef {
            value_type: ValueType::ArrayPtrOf(elem_tag),
            repr: header_ptr,
        })
    }
}
