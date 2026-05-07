use crate::parser::expression::FunctionCallExpr;

use super::super::{
    backend::LlvmBackend,
    helper::state::{ValueRef, ValueType},
};

impl LlvmBackend {
    pub(super) fn emit_function_call(&mut self, call: &FunctionCallExpr) -> Option<ValueRef> {
        let Some(info) = self.functions.get(&call.name).cloned() else {
            self.semantic_error(format!("Function '{}' is not declared", call.name));
            return None;
        };

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

            if value.value_type != expected {
                self.semantic_error(format!(
                    "Function '{}' argument #{} expects {}, but got {}.",
                    call.name,
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
                "call {return_type} @hulk_{}({})",
                call.name,
                arg_values.join(", ")
            ));
            "0".to_string()
        } else {
            let temp = self.next_temp();
            self.emit_body(format!(
                "{temp} = call {return_type} @hulk_{}({})",
                call.name,
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
