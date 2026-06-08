use crate::parser::expression::{NewExpr, TypeDecl, TypeParam};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_new_expr(
        &mut self,
        new_expr: &NewExpr,
    ) -> Option<ValueRef> {
        let Some(type_id) = self.type_ids.get(&new_expr.type_name).copied() else {
            self.semantic_error(format!("Type '{}' is not declared.", new_expr.type_name));
            return None;
        };

        let Some(type_decl) = self.type_decls.get(&new_expr.type_name).cloned() else {
            self.semantic_error(format!(
                "Type '{}' has no declaration metadata for code generation.",
                new_expr.type_name
            ));
            return None;
        };

        if type_decl.params.len() != new_expr.args.len() {
            self.semantic_error(format!(
                "Type '{}' constructor expects {} argument(s), but got {}.",
                new_expr.type_name,
                type_decl.params.len(),
                new_expr.args.len()
            ));
            return None;
        }

        let mut ctor_values = Vec::with_capacity(new_expr.args.len());
        for arg in &new_expr.args {
            ctor_values.push(self.emit_expr(arg)?);
        }

        let Some(layout) = self.struct_layout(type_id).cloned() else {
            self.semantic_error(format!(
                "Type '{}' has no layout metadata for code generation.",
                new_expr.type_name
            ));
            return None;
        };

        let object_ptr = self.next_temp();
        self.emit_body(format!(
            "{object_ptr} = call i8* @malloc(i64 {})",
            layout.size_bytes
        ));

        let tag_ptr = self.next_temp();
        self.emit_body(format!(
            "{tag_ptr} = getelementptr i8, i8* {object_ptr}, i64 0"
        ));
        let tag_ptr_i64 = self.next_temp();
        self.emit_body(format!(
            "{tag_ptr_i64} = bitcast i8* {tag_ptr} to i64*"
        ));
        self.emit_body(format!(
            "store i64 {type_id}, i64* {tag_ptr_i64}"
        ));

        self.push_scope();

        for (param, value) in type_decl.params.iter().zip(ctor_values.iter()) {
            let expected_type = param
                .type_annotation
                .as_ref()
                .and_then(|annotation| self.value_type_from_annotation_name(&annotation.name))
                .unwrap_or(value.value_type);

            if !self.are_compatible_value_types(expected_type, value.value_type) {
                self.semantic_error(format!(
                    "Type '{}' constructor parameter '{}' expects {}, but got {}.",
                    new_expr.type_name,
                    param.name,
                    self.type_name_for_value_type(expected_type),
                    self.type_name_for_value_type(value.value_type)
                ));
                self.pop_scope();
                return None;
            }

            let Some(info) = self.allocate_storage_typed(expected_type, value) else {
                self.semantic_error(format!(
                    "Type '{}' constructor parameter '{}' expects {}, but got {}.",
                    new_expr.type_name,
                    param.name,
                    self.type_name_for_value_type(expected_type),
                    self.type_name_for_value_type(value.value_type)
                ));
                self.pop_scope();
                return None;
            };
            self.bind_current_scope(param.name.clone(), info);
        }

        if !self.emit_type_initializers(&type_decl, &object_ptr, &layout, &new_expr.type_name) {
            self.pop_scope();
            return None;
        }

        self.pop_scope();

        Some(ValueRef {
            value_type: ValueType::Struct(type_id),
            repr: object_ptr,
        })
    }

    fn emit_type_initializers(
        &mut self,
        type_decl: &TypeDecl,
        object_ptr: &str,
        layout: &crate::codegen::llvm::backend::layout::StructLayout,
        concrete_type_name: &str,
    ) -> bool {
        if let Some(parent_name) = &type_decl.parent_name {
            let Some(parent_decl) = self.type_decls.get(parent_name).cloned() else {
                self.semantic_error(format!(
                    "Type '{}' inherits from '{}', but parent metadata is missing.",
                    type_decl.name, parent_name
                ));
                return false;
            };

            if parent_decl.params.len() != type_decl.parent_init_exprs.len() {
                self.semantic_error(format!(
                    "Type '{}' parent initializer for '{}' expects {} argument(s), but got {}.",
                    type_decl.name,
                    parent_name,
                    parent_decl.params.len(),
                    type_decl.parent_init_exprs.len()
                ));
                return false;
            }

            let mut parent_values = Vec::with_capacity(type_decl.parent_init_exprs.len());
            for arg in &type_decl.parent_init_exprs {
                let Some(value) = self.emit_expr(arg) else {
                    return false;
                };
                parent_values.push(value);
            }

            self.push_scope();
            if !self.bind_constructor_params(&parent_decl.params, &parent_values, parent_name) {
                self.pop_scope();
                return false;
            }
            if !self.emit_type_initializers(&parent_decl, object_ptr, layout, concrete_type_name) {
                self.pop_scope();
                return false;
            }
            self.pop_scope();
        }

        for attribute in &type_decl.attributes {
            let Some(field_layout) = layout.fields.get(&attribute.name).cloned() else {
                self.semantic_error(format!(
                    "Type '{}' has no layout entry for attribute '{}'.",
                    concrete_type_name, attribute.name
                ));
                return false;
            };

            let Some(value) = self.emit_expr(&attribute.value) else {
                return false;
            };
            if !self.are_compatible_value_types(field_layout.value_type, value.value_type) {
                self.semantic_error(format!(
                    "Attribute '{}' in type '{}' expects {}, but initializer produced {}.",
                    attribute.name,
                    concrete_type_name,
                    self.type_name_for_value_type(field_layout.value_type),
                    self.type_name_for_value_type(value.value_type)
                ));
                return false;
            }

            let Some(stored_repr) = self.value_repr_for_expected_type(field_layout.value_type, &value)
            else {
                self.semantic_error(format!(
                    "Attribute '{}' in type '{}' expects {}, but initializer produced {}.",
                    attribute.name,
                    concrete_type_name,
                    self.type_name_for_value_type(field_layout.value_type),
                    self.type_name_for_value_type(value.value_type)
                ));
                return false;
            };

            let field_ptr =
                self.emit_field_ptr(&object_ptr, field_layout.offset, field_layout.value_type);
            let llvm_type = field_layout.value_type.llvm_type();
            self.emit_body(format!(
                "store {llvm_type} {}, {llvm_type}* {field_ptr}",
                stored_repr
            ));
        }

        true
    }

    fn bind_constructor_params(
        &mut self,
        params: &[TypeParam],
        values: &[ValueRef],
        type_name: &str,
    ) -> bool {
        for (param, value) in params.iter().zip(values.iter()) {
            let expected_type = param
                .type_annotation
                .as_ref()
                .and_then(|annotation| self.value_type_from_annotation_name(&annotation.name))
                .unwrap_or(value.value_type);

            if !self.are_compatible_value_types(expected_type, value.value_type) {
                self.semantic_error(format!(
                    "Type '{}' constructor parameter '{}' expects {}, but got {}.",
                    type_name,
                    param.name,
                    self.type_name_for_value_type(expected_type),
                    self.type_name_for_value_type(value.value_type)
                ));
                return false;
            }

            let Some(info) = self.allocate_storage_typed(expected_type, value) else {
                self.semantic_error(format!(
                    "Type '{}' constructor parameter '{}' expects {}, but got {}.",
                    type_name,
                    param.name,
                    self.type_name_for_value_type(expected_type),
                    self.type_name_for_value_type(value.value_type)
                ));
                return false;
            };
            self.bind_current_scope(param.name.clone(), info);
        }

        true
    }

    fn value_type_from_annotation_name(&self, name: &str) -> Option<ValueType> {
        match name {
            "Number" => Some(ValueType::Double),
            "Boolean" => Some(ValueType::Bool),
            "String" => Some(ValueType::StringPtr),
            "Unit" => Some(ValueType::Unit),
            "Null" => Some(ValueType::Null),
            _ => self.type_ids.get(name).copied().map(ValueType::Struct),
        }
    }
}
