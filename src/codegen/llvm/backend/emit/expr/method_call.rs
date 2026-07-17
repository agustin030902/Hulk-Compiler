//! Llamadas a métodos `obj.metodo(args)`. El receptor se devirtualiza
//! estáticamente cuando el tipo concreto se conoce (`interface_real_types`);
//! si sigue siendo una interfaz, se delega al dispatch dinámico por type-tag.
//! Los arreglos exponen su único método intrínseco `size()`.

use crate::parser::expression::{Expr, MethodCallExpr};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_method_call(
        &mut self,
        call: &MethodCallExpr,
    ) -> Option<ValueRef> {
        let mut receiver = self.emit_expr(&call.receiver)?;

        // Método intrínseco de arreglos: size(): Number.
        if let ValueType::Array(_) = receiver.value_type {
            if call.method_name == "size" && call.args.is_empty() {
                return Some(self.emit_array_size(&receiver));
            }
            self.semantic_error(format!(
                "Method '{}' is not declared for arrays. Only 'size' is available.",
                call.method_name
            ));
            return None;
        }

        let ValueType::Struct(type_id) = receiver.value_type else {
            self.semantic_error(format!(
                "Method call expects a struct instance receiver, but got {}.",
                self.type_name_for_value_type(receiver.value_type)
            ));
            return None;
        };

        // Devirtualización: si la variable interfaz tiene un tipo real
        // conocido estáticamente, se llama directo a esa implementación.
        if let Expr::Variable { name, .. } = call.receiver.as_ref() {
            let real_id = self.interface_real_types.get(name).copied();
            if let Some(real_id) = real_id {
                if real_id != type_id {
                    receiver.value_type = ValueType::Struct(real_id);
                }
            }
        }

        let effective_type_id = if let ValueType::Struct(t) = receiver.value_type {
            t
        } else {
            type_id
        };

        let is_interface = !self
            .type_ids
            .iter()
            .any(|(name, tid)| *tid == effective_type_id && self.type_decls.contains_key(name));

        if is_interface {
            return self.emit_interface_method_dispatch(call, &receiver, effective_type_id);
        }

        let Some(method_key) = self
            .lookup_method_key(effective_type_id, &call.method_name)
            .cloned()
        else {
            self.semantic_error(format!(
                "Method '{}' is not declared for this type.",
                call.method_name
            ));
            return None;
        };

        let Some(info) = self.functions.get(&method_key).cloned() else {
            self.semantic_error(format!(
                "Method '{}' has no inferred metadata for code generation.",
                call.method_name
            ));
            return None;
        };

        let context = format!("Method '{}'", call.method_name);
        let receiver_repr = receiver.repr.clone();
        let arg_values = self.emit_coerced_args(
            &context,
            &call.args,
            &info.param_types,
            Some(&receiver_repr),
        )?;

        let callee = format!("@{}", info.llvm_name);
        let result =
            self.emit_call_instruction(info.return_type, &callee, &arg_values.join(", "));

        Some(ValueRef {
            value_type: info.return_type,
            repr: result,
        })
    }
}
