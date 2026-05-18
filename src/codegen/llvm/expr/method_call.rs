use crate::parser::expression::MethodCallExpr;

use super::super::{
    backend::LlvmBackend,
    helper::state::{ValueRef, ValueType},
};

impl LlvmBackend {
    pub(super) fn emit_method_call(&mut self, call: &MethodCallExpr) -> Option<ValueRef> {
        let receiver = self.emit_expr(&call.receiver)?;
        let ValueType::Struct(type_id) = receiver.value_type else {
            self.semantic_error(format!(
                "Method call expects a struct instance receiver, but got {}.",
                receiver.value_type.display_name()
            ));
            return None;
        };

        let Some(method_key) = self.lookup_method_key(type_id, &call.method_name).cloned() else {
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

        if info.param_types.len() != call.args.len() {
            self.semantic_error(format!(
                "Method '{}' expects {} argument(s), but got {}.",
                call.method_name,
                info.param_types.len(),
                call.args.len()
            ));
            return None;
        }

        let mut arg_values = Vec::with_capacity(call.args.len() + 1);
        arg_values.push(format!("i8* {}", receiver.repr));

        for (index, arg) in call.args.iter().enumerate() {
            let value = self.emit_expr(arg)?;
            let expected = info.param_types[index];

            if value.value_type != expected {
                self.semantic_error(format!(
                    "Method '{}' argument #{} expects {}, but got {}.",
                    call.method_name,
                    index + 1,
                    expected.display_name(),
                    value.value_type.display_name()
                ));
                return None;
            }

            arg_values.push(format!("{} {}", expected.llvm_type(), value.repr));
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
