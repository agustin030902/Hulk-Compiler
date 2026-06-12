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
                    BuiltinFunction::Sin => "sin",
                    BuiltinFunction::Cos => "cos",
                    BuiltinFunction::Sqrt => "sqrt",
                    BuiltinFunction::Exp => "exp",
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
                    "{ln_base} = call double @log(double {})",
                    base.repr
                ));

                let ln_value = self.next_temp();
                self.emit_body(format!(
                    "{ln_base} = call double @log(double {})",
                    base.repr
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

            let param_is_protocol = if let ValueType::Struct(exp_id) = expected {
                !self.type_ids
                    .iter()
                    .find(|(_, tid)| **tid == exp_id)
                    .is_some_and(|(name, _)| self.type_decls.contains_key(name))
            } else {
                false
            };

            if !param_is_protocol && !self.are_compatible_value_types(expected, value.value_type) {
                self.semantic_error(format!(
                    "Function '{}' argument #{} expects {}, but got {}.",
                    call.name,
                    index + 1,
                    self.type_name_for_value_type(expected),
                    self.type_name_for_value_type(value.value_type)
                ));
                return None;
            }

            let arg_repr = if param_is_protocol {
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
        let mut receiver = self.emit_expr(&call.receiver)?;
        let ValueType::Struct(type_id) = receiver.value_type else {
            self.semantic_error(format!(
                "Method call expects a struct instance receiver, but got {}.",
                self.type_name_for_value_type(receiver.value_type)
            ));
            return None;
        };

        if let Expr::Variable { name, .. } = call.receiver.as_ref() {
            let real_id = self
                .protocol_real_types
                .get(name)
                .copied();
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

        let is_protocol = !self.type_ids.iter().any(|(name, tid)| {
            *tid == effective_type_id && self.type_decls.contains_key(name)
        });

        if is_protocol {
            return self.emit_protocol_method_dispatch(call, &receiver, effective_type_id);
        }

        let Some(method_key) = self.lookup_method_key(effective_type_id, &call.method_name).cloned() else {
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

            if !self.are_compatible_value_types(expected, value.value_type) {
                self.semantic_error(format!(
                    "Method '{}' argument #{} expects {}, but got {}.",
                    call.method_name,
                    index + 1,
                    self.type_name_for_value_type(expected),
                    self.type_name_for_value_type(value.value_type)
                ));
                return None;
            }

            let Some(arg_repr) = self.value_repr_for_expected_type(expected, &value) else {
                self.semantic_error(format!(
                    "Method '{}' argument #{} expects {}, but got {}.",
                    call.method_name,
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

    fn emit_protocol_method_dispatch(
        &mut self,
        call: &MethodCallExpr,
        receiver: &ValueRef,
        protocol_type_id: u32,
    ) -> Option<ValueRef> {
        let protocol_method_key = self.lookup_method_key(protocol_type_id, &call.method_name)?.clone();
        let protocol_info = self.functions.get(&protocol_method_key)?.clone();

        if protocol_info.param_types.len() != call.args.len() {
            self.semantic_error(format!(
                "Method '{}' expects {} argument(s), but got {}.",
                call.method_name,
                protocol_info.param_types.len(),
                call.args.len()
            ));
            return None;
        }

        let mut concrete_impls: Vec<(u32, String)> = self.type_ids.iter()
            .filter(|(name, _)| self.type_decls.contains_key(name.as_str()))
            .filter_map(|(_, tid)| {
                self.method_dispatch.get(&(*tid, call.method_name.clone()))
                    .map(|key| (*tid, key.clone()))
            })
            .collect();
        concrete_impls.sort_by_key(|(tid, _)| *tid);

        let mut arg_values = Vec::with_capacity(call.args.len() + 1);
        arg_values.push(format!("i8* {}", receiver.repr));
        for (index, arg) in call.args.iter().enumerate() {
            let value = self.emit_expr(arg)?;
            let expected = protocol_info.param_types[index];
            let arg_repr = if self.are_compatible_value_types(expected, value.value_type) {
                self.value_repr_for_expected_type(expected, &value)
                    .unwrap_or_else(|| value.repr.clone())
            } else {
                value.repr.clone()
            };
            arg_values.push(format!("{} {}", expected.llvm_type(), arg_repr));
        }

        let arg_str = arg_values.join(", ");
        let return_type = protocol_info.return_type.llvm_type();

        let type_id_temp = self.next_temp();
        self.emit_body(format!("{type_id_temp} = bitcast i8* {} to i64*", receiver.repr));
        let type_id_val = self.next_temp();
        self.emit_body(format!("{type_id_val} = load i64, i64* {type_id_temp}"));

        let done_label = self.next_label("dispatch.done");
        let default_label = self.next_label("dispatch.default");
        let mut branch_results: Vec<(String, String)> = Vec::new();

        for (i, (concrete_tid, method_key)) in concrete_impls.iter().enumerate() {
            let Some(concrete_info) = self.functions.get(method_key).cloned() else {
                continue;
            };

            let is_last = i == concrete_impls.len() - 1;
            let call_label = self.next_label("dispatch.call");
            let else_label = if is_last {
                default_label.clone()
            } else {
                self.next_label("dispatch.check")
            };

            let cmp = self.next_temp();
            self.emit_body(format!("{cmp} = icmp eq i64 {type_id_val}, {concrete_tid}"));
            self.emit_body(format!(
                "br i1 {cmp}, label %{call_label}, label %{else_label}"
            ));

            self.emit_body(format!("{call_label}:"));
            let call_result = if protocol_info.return_type != ValueType::Unit {
                let t = self.next_temp();
                self.emit_body(format!(
                    "{t} = call {return_type} @{}({arg_str})",
                    concrete_info.llvm_name
                ));
                Some(t)
            } else {
                self.emit_body(format!(
                    "call {return_type} @{}({arg_str})",
                    concrete_info.llvm_name
                ));
                None
            };

            let terminal_label = self.current_block.clone();
            self.emit_body(format!("br label %{done_label}"));

            if let Some(t) = call_result {
                branch_results.push((t, terminal_label));
            }

            if !is_last {
                self.emit_body(format!("{else_label}:"));
            }
        }

        self.emit_body(format!("{default_label}:"));
        let default_result = if protocol_info.return_type != ValueType::Unit {
            let t = self.next_temp();
            self.emit_body(format!(
                "{t} = call {return_type} @{}({arg_str})",
                protocol_info.llvm_name
            ));
            Some(t)
        } else {
            self.emit_body(format!(
                "call {return_type} @{}({arg_str})",
                protocol_info.llvm_name
            ));
            None
        };
        let default_terminal = self.current_block.clone();
        self.emit_body(format!("br label %{done_label}"));

        if let Some(t) = default_result {
            branch_results.push((t, default_terminal));
        }

        self.emit_body(format!("{done_label}:"));

        if protocol_info.return_type != ValueType::Unit {
            let result = self.next_temp();
            let phi_args = branch_results
                .iter()
                .map(|(val, label)| format!("[ {}, %{} ]", val, label))
                .collect::<Vec<_>>()
                .join(", ");
            self.emit_body(format!("{result} = phi {return_type} {phi_args}"));

            Some(ValueRef {
                value_type: protocol_info.return_type,
                repr: result,
            })
        } else {
            Some(self.unit_value())
        }
    }
}
