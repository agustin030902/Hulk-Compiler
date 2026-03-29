use crate::parser::expression::{BuiltinFunction, Expr};

use super::super::{
    backend::LlvmBackend,
    helper::state::{ValueRef, ValueType},
};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn unit_value(&self) -> ValueRef {
        ValueRef {
            value_type: ValueType::Unit,
            repr: "0".to_string(),
        }
    }

    pub(super) fn emit_builtin_call(
        &mut self,
        function: BuiltinFunction,
        args: &[Expr],
    ) -> Option<ValueRef> {
        match function {
            BuiltinFunction::Print => {
                let Some(arg_expr) = args.first() else {
                    self.semantic_error("Function 'print' expects 1 argument");
                    return None;
                };
                let value = self.emit_expr(arg_expr)?;
                if value.value_type == ValueType::Unit {
                    self.semantic_error("Function 'print' expects a non-Unit argument");
                    return None;
                }
                self.emit_print_value(&value);
                Some(self.unit_value())
            }
            BuiltinFunction::Sin
            | BuiltinFunction::Cos
            | BuiltinFunction::Sqrt
            | BuiltinFunction::Exp => {
                let Some(arg_expr) = args.first() else {
                    self.semantic_error(format!(
                        "Function '{}' expects 1 argument",
                        function.name()
                    ));
                    return None;
                };

                let arg = self.emit_expr(arg_expr)?;
                if arg.value_type != ValueType::Double {
                    self.semantic_error(format!(
                        "Function '{}' only supports numeric values",
                        function.name()
                    ));
                    return None;
                }

                let intrinsic = match function {
                    BuiltinFunction::Sin => "llvm.sin.f64",
                    BuiltinFunction::Cos => "llvm.cos.f64",
                    BuiltinFunction::Sqrt => "llvm.sqrt.f64",
                    BuiltinFunction::Exp => "llvm.exp.f64",
                    BuiltinFunction::Log => unreachable!("log handled in dedicated branch"),
                    BuiltinFunction::Rand => unreachable!("rand handled in dedicated branch"),
                    BuiltinFunction::Print => unreachable!("print handled in dedicated branch"),
                };

                let result = self.next_temp();
                self.emit_body(format!(
                    "{result} = call double @{intrinsic}(double {})",
                    arg.repr
                ));

                Some(ValueRef {
                    value_type: ValueType::Double,
                    repr: result,
                })
            }
            BuiltinFunction::Log => {
                if args.len() != 2 {
                    self.semantic_error("Function 'log' expects 2 arguments");
                    return None;
                }

                let base = self.emit_expr(&args[0])?;
                let value = self.emit_expr(&args[1])?;
                if base.value_type != ValueType::Double || value.value_type != ValueType::Double {
                    self.semantic_error("Function 'log' only supports numeric values");
                    return None;
                }

                let ln_base = self.next_temp();
                self.emit_body(format!(
                    "{ln_base} = call double @llvm.log.f64(double {})",
                    base.repr
                ));

                let ln_value = self.next_temp();
                self.emit_body(format!(
                    "{ln_value} = call double @llvm.log.f64(double {})",
                    value.repr
                ));

                let result = self.next_temp();
                self.emit_body(format!("{result} = fdiv double {ln_value}, {ln_base}"));

                Some(ValueRef {
                    value_type: ValueType::Double,
                    repr: result,
                })
            }
            BuiltinFunction::Rand => {
                if !args.is_empty() {
                    self.semantic_error("Function 'rand' expects 0 arguments");
                    return None;
                }

                let raw = self.next_temp();
                self.emit_body(format!("{raw} = call i32 @rand()"));

                let as_double = self.next_temp();
                self.emit_body(format!("{as_double} = sitofp i32 {raw} to double"));

                let normalized = self.next_temp();
                self.emit_body(format!(
                    "{normalized} = fdiv double {as_double}, 2147483647.0"
                ));

                Some(ValueRef {
                    value_type: ValueType::Double,
                    repr: normalized,
                })
            }
        }
    }
}
