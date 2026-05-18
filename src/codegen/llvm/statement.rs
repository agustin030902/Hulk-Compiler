use crate::parser::expression::{FunctionDecl, MethodDecl, Program, Statement, TypeDecl};

use super::{
    backend::LlvmBackend,
    helper::{
        module_writer::format_ptr_global,
        state::{ValueRef, ValueType, VariableInfo},
    },
};

impl LlvmBackend {
    pub(super) fn emit_program(&mut self, program: &Program) {
        for type_decl in &program.types {
            for method in &type_decl.methods {
                self.emit_method_decl(type_decl, method);
            }
        }

        for function in &program.functions {
            self.emit_function_decl(function);
        }

        for statement in &program.statements {
            let _ = self.emit_statement(statement);
        }
    }

    fn emit_function_decl(&mut self, function: &FunctionDecl) {
        let Some(info) = self.functions.get(&function.name).cloned() else {
            self.semantic_error(format!(
                "Function '{}' has no inferred signature for code generation.",
                function.name
            ));
            return;
        };

        if info.param_types.len() != function.params.len() {
            self.semantic_error(format!(
                "Function '{}' has inconsistent parameter metadata.",
                function.name
            ));
            return;
        }

        let saved_body = std::mem::take(&mut self.body_lines);
        let saved_scopes = std::mem::take(&mut self.scopes);
        self.push_scope();

        let params = function
            .params
            .iter()
            .zip(info.param_types.iter().copied())
            .map(|(param, value_type)| format!("{} %{}", value_type.llvm_type(), param.name))
            .collect::<Vec<_>>();

        for (param, value_type) in function.params.iter().zip(info.param_types.iter().copied()) {
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

        let Some(result) = self.emit_expr(&function.body) else {
            self.scopes = saved_scopes;
            self.body_lines = saved_body;
            return;
        };

        if result.value_type != info.return_type {
            self.semantic_error(format!(
                "Function '{}' returns {} but inferred signature expects {}.",
                function.name,
                result.value_type.display_name(),
                info.return_type.display_name()
            ));
            self.scopes = saved_scopes;
            self.body_lines = saved_body;
            return;
        }

        let function_body = std::mem::take(&mut self.body_lines);
        self.scopes = saved_scopes;
        self.body_lines = saved_body;

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

    fn emit_method_decl(&mut self, type_decl: &TypeDecl, method: &MethodDecl) {
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
        self.push_scope();

        let self_ptr_name = self.next_temp();
        self.emit_body(format!("{self_ptr_name} = alloca i8*"));
        self.emit_body(format!("store i8* %self, i8** {self_ptr_name}"));
        self.bind_current_scope(
            "self".to_string(),
            VariableInfo {
                ptr_name: self_ptr_name,
                value_type: ValueType::Struct(type_id),
            },
        );

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
            return;
        };

        if result.value_type != info.return_type {
            self.semantic_error(format!(
                "Method '{}.{}' returns {} but inferred signature expects {}.",
                type_decl.name,
                method.name,
                result.value_type.display_name(),
                info.return_type.display_name()
            ));
            self.scopes = saved_scopes;
            self.body_lines = saved_body;
            return;
        }

        let function_body = std::mem::take(&mut self.body_lines);
        self.scopes = saved_scopes;
        self.body_lines = saved_body;

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

    pub(super) fn emit_statement(&mut self, statement: &Statement) -> Option<ValueRef> {
        match statement {
            Statement::Let { name, value, .. } => {
                if self.is_declared_in_current_scope(name) {
                    self.semantic_error(format!("Variable '{}' already declared", name));
                    return None;
                }

                let value_ref = self.emit_expr(value)?;
                let info = self.allocate_storage(&value_ref);
                self.bind_current_scope(name.clone(), info);
                Some(value_ref)
            }
            Statement::Print { value, .. } => {
                let value_ref = self.emit_expr(value)?;
                if value_ref.value_type == ValueType::Unit {
                    self.semantic_error("Function 'print' expects a non-Unit argument");
                    return None;
                }
                self.emit_print_value(&value_ref);
                Some(self.unit_value())
            }
            Statement::Expr { value, .. } => self.emit_expr(value),
            Statement::Assign { name, value, .. } => {
                let Some((scope_index, existing)) = self.lookup_var_with_index(name) else {
                    self.semantic_error(format!("Variable '{}' is not declared", name));
                    return None;
                };

                let value_ref = self.emit_expr(value)?;

                if existing.value_type == value_ref.value_type {
                    self.store_value_at(&existing.ptr_name, &value_ref);
                    Some(value_ref)
                } else {
                    let info = self.allocate_storage(&value_ref);
                    self.bind_scope(scope_index, name.clone(), info);
                    Some(value_ref)
                }
            }
        }
    }

    pub(super) fn emit_print_value(&mut self, value_ref: &ValueRef) {
        match value_ref.value_type {
            ValueType::Double => {
                let fmt = format_ptr_global("@.fmt.number", 4);
                let call_tmp = self.next_temp();
                self.emit_body(format!(
                    "{call_tmp} = call i32 (i8*, ...) @printf(i8* {fmt}, double {})",
                    value_ref.repr
                ));
            }
            ValueType::StringPtr => {
                let fmt = format_ptr_global("@.fmt.string", 4);
                let call_tmp = self.next_temp();
                self.emit_body(format!(
                    "{call_tmp} = call i32 (i8*, ...) @printf(i8* {fmt}, i8* {})",
                    value_ref.repr
                ));
            }
            ValueType::Bool => {
                let bool_tmp = self.next_temp();
                self.emit_body(format!("{bool_tmp} = zext i1 {} to i32", value_ref.repr));
                let fmt = format_ptr_global("@.fmt.bool", 4);
                let call_tmp = self.next_temp();
                self.emit_body(format!(
                    "{call_tmp} = call i32 (i8*, ...) @printf(i8* {fmt}, i32 {bool_tmp})"
                ));
            }
            ValueType::Unit => {
                self.semantic_error("Function 'print' expects a non-Unit argument");
            }
            ValueType::Function | ValueType::Struct(_) => {
                self.semantic_error(format!(
                    "Function 'print' cannot print values of type {}.",
                    value_ref.value_type.display_name()
                ));
            }
        }
    }
}
