use crate::parser::expression::{FunctionDecl, Program, Statement};

use super::{
    backend::LlvmBackend,
    helper::{
        module_writer::format_ptr_global,
        state::{ValueRef, ValueType, VariableInfo},
    },
};

impl LlvmBackend {
    pub(super) fn emit_program(&mut self, program: &Program) {
        for function in &program.functions {
            self.functions.insert(
                function.name.clone(),
                super::backend::FunctionInfo {
                    arity: function.params.len(),
                    return_type: ValueType::Double,
                },
            );
        }

        for function in &program.functions {
            self.emit_function_decl(function);
        }

        for statement in &program.statements {
            let _ = self.emit_statement(statement);
        }
    }

    fn emit_function_decl(&mut self, function: &FunctionDecl) {
        let saved_body = std::mem::take(&mut self.body_lines);
        let saved_scopes = std::mem::take(&mut self.scopes);
        self.push_scope();

        let params = function
            .params
            .iter()
            .map(|param| format!("double %{}", param.name))
            .collect::<Vec<_>>();

        for param in &function.params {
            let ptr_name = self.next_temp();
            self.emit_body(format!("{ptr_name} = alloca double"));
            self.emit_body(format!("store double %{}, double* {ptr_name}", param.name));
            self.bind_current_scope(
                param.name.clone(),
                VariableInfo {
                    ptr_name,
                    value_type: ValueType::Double,
                },
            );
        }

        let result = self.emit_expr(&function.body).unwrap_or_else(|| ValueRef {
            value_type: ValueType::Unit,
            repr: "0".to_string(),
        });
        let function_body = std::mem::take(&mut self.body_lines);
        self.scopes = saved_scopes;
        self.body_lines = saved_body;

        if let Some(info) = self.functions.get_mut(&function.name) {
            info.return_type = result.value_type;
        }

        let return_type = result.value_type.llvm_type();
        self.emit_function_line(String::new());
        self.emit_function_line(format!(
            "define {return_type} @hulk_{}({}) {{",
            function.name,
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
        }
    }
}
