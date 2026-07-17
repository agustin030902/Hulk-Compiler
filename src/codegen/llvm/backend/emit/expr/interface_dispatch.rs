//! Dispatch dinámico de métodos de interfaz: cascada de comparaciones sobre
//! el type-tag del receptor con una rama por cada tipo concreto que tenga el
//! método (propio o heredado), fusionadas con un `phi`. La rama default es
//! inalcanzable en programas bien tipados y produce el valor por defecto del
//! tipo de retorno.

use crate::parser::expression::MethodCallExpr;

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

impl LlvmBackend {
    pub(super) fn emit_interface_method_dispatch(
        &mut self,
        call: &MethodCallExpr,
        receiver: &ValueRef,
        interface_type_id: u32,
    ) -> Option<ValueRef> {
        let interface_method_key = self
            .lookup_method_key(interface_type_id, &call.method_name)?
            .clone();
        let interface_info = self.functions.get(&interface_method_key)?.clone();

        if interface_info.param_types.len() != call.args.len() {
            self.semantic_error(format!(
                "Method '{}' expects {} argument(s), but got {}.",
                call.method_name,
                interface_info.param_types.len(),
                call.args.len()
            ));
            return None;
        }

        // lookup_method_key sube por la jerarquía: un tipo que hereda el
        // método sin sobrescribirlo también recibe su rama de dispatch.
        let mut concrete_impls: Vec<(u32, String)> = self
            .type_ids
            .iter()
            .filter(|(name, _)| self.type_decls.contains_key(name.as_str()))
            .filter_map(|(_, tid)| {
                self.lookup_method_key(*tid, &call.method_name)
                    .map(|key| (*tid, key.clone()))
            })
            .collect();
        concrete_impls.sort_by_key(|(tid, _)| *tid);

        // Los argumentos se coercen laxamente: los tipos exactos varían por
        // implementación, así que ante incompatibilidad se pasa el repr crudo.
        let mut arg_values = Vec::with_capacity(call.args.len() + 1);
        arg_values.push(format!("i8* {}", receiver.repr));
        for (index, arg) in call.args.iter().enumerate() {
            let value = self.emit_expr(arg)?;
            let expected = interface_info.param_types[index];
            let arg_repr = if self.are_compatible_value_types(expected, value.value_type) {
                self.value_repr_for_expected_type(expected, &value)
                    .unwrap_or_else(|| value.repr.clone())
            } else {
                value.repr.clone()
            };
            arg_values.push(format!("{} {}", expected.llvm_type(), arg_repr));
        }

        let arg_str = arg_values.join(", ");
        let return_type = interface_info.return_type;

        let type_id_temp = self.next_temp();
        self.emit_body(format!(
            "{type_id_temp} = bitcast i8* {} to i64*",
            receiver.repr
        ));
        let type_id_val = self.next_temp();
        self.emit_body(format!("{type_id_val} = load i64, i64* {type_id_temp}"));

        let done_label = self.next_label("dispatch.done");
        let default_label = self.next_label("dispatch.default");
        let mut branch_results: Vec<(String, String)> = Vec::new();

        if concrete_impls.is_empty() {
            self.emit_body(format!("br label %{default_label}"));
        }

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
            let callee = format!("@{}", concrete_info.llvm_name);
            let call_result = self.emit_call_instruction(return_type, &callee, &arg_str);

            let terminal_label = self.current_block.clone();
            self.emit_body(format!("br label %{done_label}"));

            if return_type != ValueType::Unit {
                branch_results.push((call_result, terminal_label));
            }

            if !is_last {
                self.emit_body(format!("{else_label}:"));
            }
        }

        // Rama default (inalcanzable en programas bien tipados): produce el
        // valor por defecto del tipo de retorno en vez de llamar al stub de la
        // interfaz, que no existe para interfaces builtin o sintetizadas.
        self.emit_body(format!("{default_label}:"));
        let default_result = if return_type != ValueType::Unit {
            let default_val = match return_type {
                ValueType::Double => "0.0".to_string(),
                ValueType::Bool => "false".to_string(),
                _ => "null".to_string(),
            };
            Some(default_val)
        } else {
            None
        };
        let default_terminal = self.current_block.clone();
        self.emit_body(format!("br label %{done_label}"));

        if let Some(t) = default_result {
            branch_results.push((t, default_terminal));
        }

        self.emit_body(format!("{done_label}:"));

        if return_type != ValueType::Unit {
            let result = self.next_temp();
            let phi_args = branch_results
                .iter()
                .map(|(val, label)| format!("[ {}, %{} ]", val, label))
                .collect::<Vec<_>>()
                .join(", ");
            self.emit_body(format!(
                "{result} = phi {} {phi_args}",
                return_type.llvm_type()
            ));

            Some(ValueRef {
                value_type: return_type,
                repr: result,
            })
        } else {
            Some(self.unit_value())
        }
    }
}
