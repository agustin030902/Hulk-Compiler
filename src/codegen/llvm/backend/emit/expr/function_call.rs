//! Llamadas a funciones globales `f(args)`. Una variable local de tipo
//! función tiene prioridad (shadowing) y se despacha como closure; los
//! parámetros de tipo interfaz aceptan cualquier implementador sin coerción
//! (el dispatch en destino resuelve por type-tag).

use crate::parser::expression::FunctionCallExpr;

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_function_call(
        &mut self,
        call: &FunctionCallExpr,
    ) -> Option<ValueRef> {
        // Una variable local de tipo función (closure) tiene prioridad sobre
        // las funciones globales: `f(x)` con `f` ligada a una lambda.
        if let Some(var_info) = self.lookup_var(&call.name) {
            if let ValueType::Function(function_type_id) = var_info.value_type {
                return self.emit_closure_call(&var_info, function_type_id, &call.args, &call.name);
            }
        }

        let Some(info) = self.functions.get(&call.name).cloned() else {
            self.semantic_error(format!("Function '{}' is not declared", call.name));
            return None;
        };

        if info.receiver_type_id.is_some() {
            self.semantic_error(format!(
                "Method '{}' requires a receiver and cannot be called as a global function.",
                call.name
            ));
            return None;
        }

        if info.param_types.len() != call.args.len() {
            self.semantic_error(format!(
                "Function '{}' expects {} argument(s), but got {}.",
                call.name,
                info.param_types.len(),
                call.args.len()
            ));
            return None;
        }

        let mut arg_values = Vec::with_capacity(call.args.len());
        for (index, arg) in call.args.iter().enumerate() {
            let value = self.emit_expr(arg)?;
            let expected = info.param_types[index];

            // Un parámetro de tipo interfaz acepta cualquier implementador:
            // se pasa el i8* tal cual y el dispatch dinámico resuelve después.
            let param_is_interface = if let ValueType::Struct(exp_id) = expected {
                !self
                    .type_ids
                    .iter()
                    .find(|(_, tid)| **tid == exp_id)
                    .is_some_and(|(name, _)| self.type_decls.contains_key(name))
            } else {
                false
            };

            if !param_is_interface && !self.are_compatible_value_types(expected, value.value_type) {
                self.semantic_error(format!(
                    "Function '{}' argument #{} expects {}, but got {}.",
                    call.name,
                    index + 1,
                    self.type_name_for_value_type(expected),
                    self.type_name_for_value_type(value.value_type)
                ));
                return None;
            }

            let arg_repr = if param_is_interface {
                value.repr.clone()
            } else {
                let Some(repr) = self.value_repr_for_expected_type(expected, &value) else {
                    self.semantic_error(format!(
                        "Function '{}' argument #{} expects {}, but got {}.",
                        call.name,
                        index + 1,
                        self.type_name_for_value_type(expected),
                        self.type_name_for_value_type(value.value_type)
                    ));
                    return None;
                };
                repr
            };

            arg_values.push(format!("{} {}", expected.llvm_type(), arg_repr));
        }

        let callee = format!("@{}", info.llvm_name);
        let result =
            self.emit_call_instruction(info.return_type, &callee, &arg_values.join(", "));

        Some(ValueRef {
            value_type: info.return_type,
            repr: result,
        })
    }
}
