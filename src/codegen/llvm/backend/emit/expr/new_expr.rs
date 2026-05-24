use crate::parser::expression::NewExpr;

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

        self.push_scope();

        for (param, value) in type_decl.params.iter().zip(ctor_values.iter()) {
            let expected_type = param
                .type_annotation
                .as_ref()
                .and_then(|annotation| self.value_type_from_annotation_name(&annotation.name))
                .unwrap_or(value.value_type);

            if !Self::are_compatible_value_types(expected_type, value.value_type) {
                self.semantic_error(format!(
                    "Type '{}' constructor parameter '{}' expects {}, but got {}.",
                    new_expr.type_name,
                    param.name,
                    expected_type.display_name(),
                    value.value_type.display_name()
                ));
                self.pop_scope();
                return None;
            }

            let Some(info) = self.allocate_storage_typed(expected_type, value) else {
                self.semantic_error(format!(
                    "Type '{}' constructor parameter '{}' expects {}, but got {}.",
                    new_expr.type_name,
                    param.name,
                    expected_type.display_name(),
                    value.value_type.display_name()
                ));
                self.pop_scope();
                return None;
            };
            self.bind_current_scope(param.name.clone(), info);
        }

        for attribute in &type_decl.attributes {
            let Some(field_layout) = layout.fields.get(&attribute.name).cloned() else {
                self.semantic_error(format!(
                    "Type '{}' has no layout entry for attribute '{}'.",
                    new_expr.type_name, attribute.name
                ));
                self.pop_scope();
                return None;
            };

            let value = self.emit_expr(&attribute.value)?;
            if !Self::are_compatible_value_types(field_layout.value_type, value.value_type) {
                self.semantic_error(format!(
                    "Attribute '{}' in type '{}' expects {}, but initializer produced {}.",
                    attribute.name,
                    new_expr.type_name,
                    field_layout.value_type.display_name(),
                    value.value_type.display_name()
                ));
                self.pop_scope();
                return None;
            }

            let Some(stored_repr) = self.value_repr_for_expected_type(field_layout.value_type, &value)
            else {
                self.semantic_error(format!(
                    "Attribute '{}' in type '{}' expects {}, but initializer produced {}.",
                    attribute.name,
                    new_expr.type_name,
                    field_layout.value_type.display_name(),
                    value.value_type.display_name()
                ));
                self.pop_scope();
                return None;
            };

            let field_ptr =
                self.emit_field_ptr(&object_ptr, field_layout.offset, field_layout.value_type);
            let llvm_type = field_layout.value_type.llvm_type();
            self.emit_body(format!(
                "store {llvm_type} {}, {llvm_type}* {field_ptr}",
                stored_repr
            ));
        }

        self.pop_scope();

        Some(ValueRef {
            value_type: ValueType::Struct(type_id),
            repr: object_ptr,
        })
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
