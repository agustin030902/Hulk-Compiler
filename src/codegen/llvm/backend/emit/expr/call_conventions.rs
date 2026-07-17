//! Convenciones de llamada compartidas por toda la familia de llamadas
//! (funciones globales, métodos, dispatch de interfaces y closures):
//! validación de aridad + coerción de argumentos, y la instrucción `call`
//! con o sin temporal según el tipo de retorno.

use crate::parser::expression::Expr;

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::ValueType;

impl LlvmBackend {
    /// Emite `call` a `callee` (un `@nombre` o un `%fnptr`) y devuelve la
    /// representación del resultado: un temporal, o `"0"` para `Unit` (que no
    /// produce valor SSA).
    pub(in crate::codegen::llvm) fn emit_call_instruction(
        &mut self,
        return_type: ValueType,
        callee: &str,
        args: &str,
    ) -> String {
        let return_llvm = return_type.llvm_type();
        if return_type == ValueType::Unit {
            self.emit_body(format!("call {return_llvm} {callee}({args})"));
            "0".to_string()
        } else {
            let temp = self.next_temp();
            self.emit_body(format!("{temp} = call {return_llvm} {callee}({args})"));
            temp
        }
    }

    /// Valida la aridad, emite cada argumento y lo coerce estrictamente a su
    /// tipo de parámetro. Devuelve los argumentos ya tipados (`tipo repr`),
    /// precedidos por `receiver_repr` (el `i8*` del receptor o closure) si se
    /// proporciona. `context` prefija los mensajes de error
    /// (p. ej. `Method 'area'`).
    pub(in crate::codegen::llvm) fn emit_coerced_args(
        &mut self,
        context: &str,
        args: &[Expr],
        param_types: &[ValueType],
        receiver_repr: Option<&str>,
    ) -> Option<Vec<String>> {
        if param_types.len() != args.len() {
            self.semantic_error(format!(
                "{context} expects {} argument(s), but got {}.",
                param_types.len(),
                args.len()
            ));
            return None;
        }

        let mut arg_values = Vec::with_capacity(args.len() + 1);
        if let Some(receiver) = receiver_repr {
            arg_values.push(format!("i8* {receiver}"));
        }

        for (index, (arg, expected)) in args.iter().zip(param_types.iter().copied()).enumerate() {
            let value = self.emit_expr(arg)?;

            if !self.are_compatible_value_types(expected, value.value_type) {
                self.semantic_error(format!(
                    "{context} argument #{} expects {}, but got {}.",
                    index + 1,
                    self.type_name_for_value_type(expected),
                    self.type_name_for_value_type(value.value_type)
                ));
                return None;
            }

            let Some(arg_repr) = self.value_repr_for_expected_type(expected, &value) else {
                self.semantic_error(format!(
                    "{context} argument #{} expects {}, but got {}.",
                    index + 1,
                    self.type_name_for_value_type(expected),
                    self.type_name_for_value_type(value.value_type)
                ));
                return None;
            };

            arg_values.push(format!("{} {}", expected.llvm_type(), arg_repr));
        }

        Some(arg_values)
    }
}
