use crate::parser::expression::BaseCallExpr;

use super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(super) fn emit_base_call(&mut self, call: &BaseCallExpr) -> Option<ValueRef> {
        let Some(method_name) = &self.current_method_name else {
            self.semantic_error("Base call 'base()' can only be used inside a type method.".to_string());
            return None;
        };

        let Some(type_id) = self.current_type_id else {
            self.semantic_error("Base call 'base()' can only be used inside a type method.".to_string());
            return None;
        };

        let type_name = self
            .type_ids
            .iter()
            .find(|(_, id)| **id == type_id)
            .map(|(name, _)| name.clone())
            .unwrap_or_default();

        let type_decl = self.type_decls.get(&type_name)?;
        let Some(parent_name) = &type_decl.parent_name else {
            self.semantic_error(format!(
                "Base call 'base()' failed: type '{}' has no parent type.",
                type_name
            ));
            return None;
        };

        let parent_method_key = format!("type#{}::{}", self.type_ids.get(parent_name).copied().unwrap_or(0), method_name);
        let Some(info) = self.functions.get(&parent_method_key).cloned() else {
            self.semantic_error(format!(
                "Base call 'base()' failed: parent type '{}' has no method '{}'.",
                parent_name, method_name
            ));
            return None;
        };

        if info.param_types.len() != call.args.len() {
            self.semantic_error(format!(
                "Base call 'base()' expects {} argument(s), but got {}.",
                info.param_types.len(),
                call.args.len()
            ));
            return None;
        }

        let self_var = self.current_self_ref.clone()?;
        let self_loaded = self.next_temp();
        self.emit_body(format!(
            "{self_loaded} = load i8*, i8** {}",
            self_var.ptr_name
        ));
        let mut arg_values = Vec::with_capacity(call.args.len() + 1);
        arg_values.push(format!("i8* {self_loaded}"));

        for (index, arg) in call.args.iter().enumerate() {
            let value = self.emit_expr(arg)?;
            let expected = info.param_types[index];

            if !self.are_compatible_value_types(expected, value.value_type) {
                self.semantic_error(format!(
                    "Base call argument #{} expects {}, but got {}.",
                    index + 1,
                    self.type_name_for_value_type(expected),
                    self.type_name_for_value_type(value.value_type)
                ));
                return None;
            }

            let Some(arg_repr) = self.value_repr_for_expected_type(expected, &value) else {
                self.semantic_error(format!(
                    "Base call argument #{} expects {}, but got {}.",
                    index + 1,
                    self.type_name_for_value_type(expected),
                    self.type_name_for_value_type(value.value_type)
                ));
                return None;
            };

            arg_values.push(format!("{} {}", expected.llvm_type(), arg_repr));
        }

        let return_type = info.return_type.llvm_type();
        let result = if info.return_type == ValueType::Unit {
            self.emit_body(format!(
                "call {return_type} @{}({})",
                info.llvm_name,
                arg_values.join(", ")
            ));
            "0".to_string()
        } else {
            let temp = self.next_temp();
            self.emit_body(format!(
                "{temp} = call {return_type} @{}({})",
                info.llvm_name,
                arg_values.join(", ")
            ));
            temp
        };

        Some(ValueRef {
            value_type: info.return_type,
            repr: result,
        })
    }
}
