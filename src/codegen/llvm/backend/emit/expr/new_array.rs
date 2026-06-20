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

        let num_elements_i64_val = self.next_temp();
        self.emit_body(format!(
            "{num_elements_i64_val} = load i64, i64* {length_ptr_i64}"
        ));

        if let Some(lambda) = &expr.initializer {
            let loop_i = self.next_temp();
            let init_ptr = self.next_temp();
            self.emit_body(format!("{loop_i} = alloca i64"));
            self.emit_body(format!("store i64 0, i64* {loop_i}"));
            let loop_cond = self.next_temp();
            let loop_body_label = format!("init_loop_body_{}", self.next_temp().replace('%', ""));
            let loop_end_label = format!("init_loop_end_{}", self.next_temp().replace('%', ""));
            self.emit_body(format!("br label %{loop_body_label}"));
            self.emit_body(format!("{loop_body_label}:"));
            let current_i = self.next_temp();
            self.emit_body(format!("{current_i} = load i64, i64* {loop_i}"));
            self.emit_body(format!(
                "{loop_cond} = icmp slt i64 {current_i}, {num_elements_i64_val}"
            ));
            let loop_exit_label = format!("init_loop_exit_{}", self.next_temp().replace('%', ""));
            self.emit_body(format!("br i1 {loop_cond}, label %{loop_exit_label}, label %{loop_end_label}"));
            self.emit_body(format!("{loop_exit_label}:"));

            let i_as_double = self.next_temp();
            self.emit_body(format!("{i_as_double} = sitofp i64 {current_i} to double"));
            let index_val = ValueRef {
                value_type: ValueType::Double,
                repr: i_as_double,
            };
            let param_info = self.allocate_storage(&index_val);
            self.bind_current_scope(lambda.param_name.clone(), param_info);
            let scope_cleanup = self.next_temp();
            self.emit_body(format!("{scope_cleanup} = alloca i64"));
            self.emit_body(format!("store i64 0, i64* {scope_cleanup}"));

            let elem_val = self.emit_expr(&lambda.body)?;

            let offset_i64 = self.next_temp();
            self.emit_body(format!("{offset_i64} = mul i64 {current_i}, 8"));
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
                _ => {
                    let slot = self.next_temp();
                    self.emit_body(format!("{slot} = bitcast i8* {elem_ptr} to i8**"));
                    self.emit_body(format!("store i8* {}, i8** {slot}", elem_val.repr));
                }
            }

            let next_i = self.next_temp();
            self.emit_body(format!("{next_i} = add i64 {current_i}, 1"));
            self.emit_body(format!("store i64 {next_i}, i64* {loop_i}"));
            self.emit_body(format!("br label %{loop_body_label}"));
            self.emit_body(format!("{loop_end_label}:"));
        } else {
            let zero_i = self.next_temp();
            self.emit_body(format!("{zero_i} = alloca i64"));
            self.emit_body(format!("store i64 0, i64* {zero_i}"));
            let zero_cond = self.next_temp();
            let zero_body = format!("zero_body_{}", self.next_temp().replace('%', ""));
            let zero_end = format!("zero_end_{}", self.next_temp().replace('%', ""));
            self.emit_body(format!("br label %{zero_body}"));
            self.emit_body(format!("{zero_body}:"));
            let z_i = self.next_temp();
            self.emit_body(format!("{z_i} = load i64, i64* {zero_i}"));
            let z_cond = self.next_temp();
            self.emit_body(format!("{z_cond} = icmp slt i64 {z_i}, {num_elements_i64_val}"));
            let z_exit = format!("zero_exit_{}", self.next_temp().replace('%', ""));
            self.emit_body(format!("br i1 {z_cond}, label %{z_exit}, label %{zero_end}"));
            self.emit_body(format!("{z_exit}:"));
            let z_offset = self.next_temp();
            self.emit_body(format!("{z_offset} = mul i64 {z_i}, 8"));
            let z_elem_ptr = self.next_temp();
            self.emit_body(format!("{z_elem_ptr} = getelementptr i8, i8* {data_ptr}, i64 {z_offset}"));
            let z_slot = self.next_temp();
            self.emit_body(format!("{z_slot} = bitcast i8* {z_elem_ptr} to i8**"));
            self.emit_body(format!("store i8* null, i8** {z_slot}"));
            let z_next = self.next_temp();
            self.emit_body(format!("{z_next} = add i64 {z_i}, 1"));
            self.emit_body(format!("store i64 {z_next}, i64* {zero_i}"));
            self.emit_body(format!("br label %{zero_body}"));
            self.emit_body(format!("{zero_end}:"));
        }

        let base_tag = match expr.type_name.as_str() {
            "Number" => crate::codegen::llvm::helper::state::ElementTag::Double,
            "Boolean" => crate::codegen::llvm::helper::state::ElementTag::Bool,
            "String" => crate::codegen::llvm::helper::state::ElementTag::StringPtr,
            "Unit" => crate::codegen::llvm::helper::state::ElementTag::Unit,
            _ => {
                if let Some(tid) = self.type_ids.get(&expr.type_name).copied() {
                    crate::codegen::llvm::helper::state::ElementTag::Struct(tid)
                } else {
                    crate::codegen::llvm::helper::state::ElementTag::Double
                }
            }
        };

        let elem_tag = if expr.element_type_dims > 0 {
            let mut _tag = base_tag;
            for _ in 0..expr.element_type_dims {
                _tag = crate::codegen::llvm::helper::state::ElementTag::Array;
            }
            _tag
        } else {
            base_tag
        };

        Some(ValueRef {
            value_type: ValueType::ArrayPtrOf(elem_tag),
            repr: header_ptr,
        })
    }
}
