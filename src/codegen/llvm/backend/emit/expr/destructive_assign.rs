use crate::parser::expression::{AssignTarget, DestructiveAssignExpr};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_destructive_assign(
        &mut self,
        assign: &DestructiveAssignExpr,
    ) -> Option<ValueRef> {
        match &assign.target {
            AssignTarget::Variable { name, .. } => {
                let Some((scope_index, existing)) = self.lookup_var_with_index(name) else {
                    self.semantic_error(format!(
                        "Variable '{}' is assigned before declaration. Declare it with 'let' first.",
                        name
                    ));
                    return None;
                };

                let value_ref = self.emit_expr(&assign.value)?;

                if !self.are_compatible_value_types(existing.value_type, value_ref.value_type) {
                    self.semantic_error(format!(
                        "Destructive assignment ':=' requires the same type. Variable '{}' is {:?} but expression is {:?}.",
                        name, existing.value_type, value_ref.value_type
                    ));
                    return None;
                }

                if existing.value_type == ValueType::Null
                    && value_ref.value_type != ValueType::Null
                    && Self::is_nullable_value_type(value_ref.value_type)
                {
                    let Some(info) = self.allocate_storage_typed(value_ref.value_type, &value_ref)
                    else {
                        self.semantic_error(format!(
                            "Destructive assignment ':=' requires the same type. Variable '{}' is {:?} but expression is {:?}.",
                            name, existing.value_type, value_ref.value_type
                        ));
                        return None;
                    };
                    self.bind_scope(scope_index, name.clone(), info);
                    return Some(value_ref);
                }

                self.store_value_at_typed(&existing.ptr_name, existing.value_type, &value_ref);
                Some(ValueRef {
                    value_type: existing.value_type,
                    repr: value_ref.repr,
                })
            }
            AssignTarget::Index { object, index, .. } => {
                let object_ref = self.emit_expr(object)?;
                if !object_ref.value_type.is_array() {
                    self.semantic_error(format!(
                        "Array index assignment expects an Array, but got {}.",
                        self.type_name_for_value_type(object_ref.value_type)
                    ));
                    return None;
                }

                let index_ref = self.emit_expr(index)?;
                if index_ref.value_type != ValueType::Double {
                    self.semantic_error("Array index must be a Number.");
                    return None;
                }

                let value_ref = self.emit_expr(&assign.value)?;

                let index_as_i64 = self.next_temp();
                self.emit_body(format!(
                    "{index_as_i64} = fptosi double {0} to i64",
                    index_ref.repr
                ));

                let data_field_ptr = self.next_temp();
                self.emit_body(format!(
                    "{data_field_ptr} = getelementptr i8, i8* {0}, i64 16",
                    object_ref.repr
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
                let elem_type = object_ref.value_type.array_element_type().unwrap_or_else(|| {
                    self.infer_target_array_element_type(&object)
                });

                match elem_type {
                    ValueType::Double => {
                        let slot = self.next_temp();
                        self.emit_body(format!("{slot} = bitcast i8* {elem_ptr} to double*"));
                        self.emit_body(format!("store double {}, double* {slot}", value_ref.repr));
                    }
                    ValueType::Bool => {
                        let slot = self.next_temp();
                        self.emit_body(format!("{slot} = bitcast i8* {elem_ptr} to i64*"));
                        let ext = self.next_temp();
                        self.emit_body(format!("{ext} = zext i1 {} to i64", value_ref.repr));
                        self.emit_body(format!("store i64 {ext}, i64* {slot}"));
                    }
                    _ => {
                        let slot = self.next_temp();
                        self.emit_body(format!("{slot} = bitcast i8* {elem_ptr} to i8**"));
                        self.emit_body(format!("store i8* {}, i8** {slot}", value_ref.repr));
                    }
                }

                Some(ValueRef {
                    value_type: value_ref.value_type,
                    repr: value_ref.repr,
                })
            }
            AssignTarget::Member { object, member, .. } => {
                let object_ref = self.emit_expr(object)?;
                let ValueType::Struct(type_id) = object_ref.value_type else {
                    self.semantic_error(format!(
                        "Member assignment expects a struct instance, but got {}.",
                        self.type_name_for_value_type(object_ref.value_type)
                    ));
                    return None;
                };

                let Some(field_layout) = self.field_layout(type_id, member).cloned() else {
                    self.semantic_error(format!(
                        "Attribute '{}' is not declared in this type.",
                        member
                    ));
                    return None;
                };

                let value_ref = self.emit_expr(&assign.value)?;
                if !self.are_compatible_value_types(field_layout.value_type, value_ref.value_type)
                {
                    self.semantic_error(format!(
                        "Destructive assignment ':=' requires type {}, but expression is {}.",
                        self.type_name_for_value_type(field_layout.value_type),
                        self.type_name_for_value_type(value_ref.value_type)
                    ));
                    return None;
                }

                let Some(stored_repr) =
                    self.value_repr_for_expected_type(field_layout.value_type, &value_ref)
                else {
                    self.semantic_error(format!(
                        "Destructive assignment ':=' requires type {}, but expression is {}.",
                        self.type_name_for_value_type(field_layout.value_type),
                        self.type_name_for_value_type(value_ref.value_type)
                    ));
                    return None;
                };

                let field_ptr = self.emit_field_ptr(
                    &object_ref.repr,
                    field_layout.offset,
                    field_layout.value_type,
                );
                let llvm_type = field_layout.value_type.llvm_type();
                self.emit_body(format!(
                    "store {llvm_type} {}, {llvm_type}* {field_ptr}",
                    stored_repr
                ));

                Some(ValueRef {
                    value_type: field_layout.value_type,
                    repr: value_ref.repr,
                })
            }
        }
    }
}
