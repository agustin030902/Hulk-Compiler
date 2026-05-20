use crate::parser::expression::{BuiltinFunction, Expr, FunctionCallExpr, MethodCallExpr};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_builtin_call(
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

    pub(in crate::codegen::llvm) fn emit_function_call(
        &mut self,
        call: &FunctionCallExpr,
    ) -> Option<ValueRef> {
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

    pub(in crate::codegen::llvm) fn emit_method_call(
        &mut self,
        call: &MethodCallExpr,
    ) -> Option<ValueRef> {
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
