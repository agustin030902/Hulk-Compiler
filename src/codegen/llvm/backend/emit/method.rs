use crate::parser::expression::{MethodDecl, InterfaceDecl, TypeDecl};

use super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueType, VariableInfo};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_method_decl(
        &mut self,
        type_decl: &TypeDecl,
        method: &MethodDecl,
    ) {
        let Some(type_id) = self.type_ids.get(&type_decl.name).copied() else {
            self.semantic_error(format!(
                "Type '{}' has no registered id for method emission.",
                type_decl.name
            ));
            return;
        };

        let key = format!("type#{}::{}", type_id, method.name);
        let Some(info) = self.functions.get(&key).cloned() else {
            self.semantic_error(format!(
                "Method '{}.{}' has no inferred signature for code generation.",
                type_decl.name, method.name
            ));
            return;
        };

        if info.param_types.len() != method.params.len() {
            self.semantic_error(format!(
                "Method '{}.{}' has inconsistent parameter metadata.",
                type_decl.name, method.name
            ));
            return;
        }

        let saved_body = std::mem::take(&mut self.body_lines);
        let saved_scopes = std::mem::take(&mut self.scopes);
        let saved_type_id = self.current_type_id;
        let saved_method_name = self.current_method_name.take();
        let saved_self_ref = self.current_self_ref.take();
        self.push_scope();

        let self_ptr_name = self.next_temp();
        self.emit_body(format!("{self_ptr_name} = alloca i8*"));
        self.emit_body(format!("store i8* %self, i8** {self_ptr_name}"));
        let self_var_info = VariableInfo {
            ptr_name: self_ptr_name,
            value_type: ValueType::Struct(type_id),
        };
        self.bind_current_scope("self".to_string(), self_var_info.clone());

        self.current_type_id = Some(type_id);
        self.current_method_name = Some(method.name.clone());
        self.current_self_ref = Some(self_var_info);

        self.push_scope();

        let mut params = vec!["i8* %self".to_string()];
        params.extend(
            method
                .params
                .iter()
                .zip(info.param_types.iter().copied())
                .map(|(param, value_type)| format!("{} %{}", value_type.llvm_type(), param.name))
                .collect::<Vec<_>>(),
        );

        for (param, value_type) in method.params.iter().zip(info.param_types.iter().copied()) {
            let ptr_name = self.next_temp();
            let llvm_type = value_type.llvm_type();
            self.emit_body(format!("{ptr_name} = alloca {llvm_type}"));
            self.emit_body(format!(
                "store {llvm_type} %{}, {llvm_type}* {ptr_name}",
                param.name
            ));
            self.bind_current_scope(
                param.name.clone(),
                VariableInfo {
                    ptr_name,
                    value_type,
                },
            );
        }

        let Some(result) = self.emit_expr(&method.body) else {
            self.scopes = saved_scopes;
            self.body_lines = saved_body;
            self.current_type_id = saved_type_id;
            self.current_method_name = saved_method_name;
            self.current_self_ref = saved_self_ref;
            return;
        };

        if result.value_type != info.return_type {
            self.semantic_error(format!(
                "Method '{}.{}' returns {} but inferred signature expects {}.",
                type_decl.name,
                method.name,
                self.type_name_for_value_type(result.value_type),
                self.type_name_for_value_type(info.return_type)
            ));
            self.scopes = saved_scopes;
            self.body_lines = saved_body;
            self.current_type_id = saved_type_id;
            self.current_method_name = saved_method_name;
            self.current_self_ref = saved_self_ref;
            return;
        }

        let function_body = std::mem::take(&mut self.body_lines);
        self.scopes = saved_scopes;
        self.body_lines = saved_body;
        self.current_type_id = saved_type_id;
        self.current_method_name = saved_method_name;
        self.current_self_ref = saved_self_ref;

        let return_type = info.return_type.llvm_type();
        self.emit_function_line(String::new());
        self.emit_function_line(format!(
            "define {return_type} @{}({}) {{",
            info.llvm_name,
            params.join(", ")
        ));
        self.emit_function_line("entry:");
        for line in function_body {
            if line.ends_with(':') {
                self.emit_function_line(line);
            } else {
                self.emit_function_line(format!("  {line}"));
            }
        }
        self.emit_function_line(format!("  ret {return_type} {}", result.repr));
        self.emit_function_line("}");
    }

    pub(in crate::codegen::llvm) fn emit_interface_methods(&mut self, interface_decl: &InterfaceDecl) {
        let Some(interface_type_id) = self.type_ids.get(&interface_decl.name).copied() else {
            return;
        };

        for method in &interface_decl.methods {
            let key = format!("type#{}::{}", interface_type_id, method.name);
            let Some(info) = self.functions.get(&key).cloned() else {
                continue;
            };

            let mut params = vec!["i8* %self".to_string()];
            params.extend(
                method
                    .params
                    .iter()
                    .zip(info.param_types.iter().copied())
                    .map(|(param, value_type)| {
                        format!("{} %{}", value_type.llvm_type(), param.name)
                    })
                    .collect::<Vec<_>>(),
            );

            let return_type = info.return_type.llvm_type();
            let default_val = match info.return_type {
                ValueType::Double => "0.0".to_string(),
                ValueType::Bool => "false".to_string(),
                ValueType::StringPtr
                | ValueType::Struct(_)
                | ValueType::Null
                | ValueType::Function(_)
                | ValueType::Array(_) => "null".to_string(),
                ValueType::Unit => "0".to_string(),
            };

            self.emit_function_line(String::new());
            self.emit_function_line(format!(
                "define {return_type} @{}({}) {{",
                info.llvm_name,
                params.join(", ")
            ));
            self.emit_function_line("entry:".to_string());
            self.emit_function_line(format!("  ret {return_type} {default_val}"));
            self.emit_function_line("}".to_string());
        }
    }
}
