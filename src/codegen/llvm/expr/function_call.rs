use crate::parser::expression::FunctionCallExpr;

use super::super::{
    backend::LlvmBackend,
    helper::state::{ValueRef, ValueType},
};

impl LlvmBackend {
    pub(super) fn emit_function_call(&mut self, call: &FunctionCallExpr) -> Option<ValueRef> {
        let Some(info) = self.functions.get(&call.name).copied() else {
            self.semantic_error(format!("Function '{}' is not declared", call.name));
            return None;
        };

        if info.arity != call.args.len() {
            self.semantic_error(format!(
                "Function '{}' expects {} argument(s), but got {}.",
                call.name,
                info.arity,
                call.args.len()
            ));
            return None;
        }

        let mut arg_values = Vec::with_capacity(call.args.len());
        for arg in &call.args {
            let value = self.emit_expr(arg)?;
            if value.value_type != ValueType::Double {
                self.semantic_error(format!(
                    "Function '{}' currently expects numeric arguments in code generation",
                    call.name
                ));
                return None;
            }
            arg_values.push(format!("double {}", value.repr));
        }

        let result = self.next_temp();
        let return_type = info.return_type.llvm_type();
        self.emit_body(format!(
            "{result} = call {return_type} @hulk_{}({})",
            call.name,
            arg_values.join(", ")
        ));

        Some(ValueRef {
            value_type: info.return_type,
            repr: result,
        })
    }
}
