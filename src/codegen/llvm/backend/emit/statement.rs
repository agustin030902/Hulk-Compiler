use crate::parser::expression::Statement;

use super::super::LlvmBackend;
use crate::codegen::llvm::helper::{
    module_writer::format_ptr_global,
    state::{ValueRef, ValueType},
};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn unit_value(&self) -> ValueRef {
        ValueRef {
            value_type: ValueType::Unit,
            repr: "0".to_string(),
        }
    }

    pub(in crate::codegen::llvm) fn emit_statement(
        &mut self,
        statement: &Statement,
    ) -> Option<ValueRef> {
        match statement {
            Statement::Let { name, value, .. } => self.emit_let(name, value),
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
            Statement::Assign { name, value, .. } => self.emit_assign(name, value),
        }
    }

    pub(in crate::codegen::llvm) fn emit_let(
        &mut self,
        name: &str,
        value: &crate::parser::expression::Expr,
    ) -> Option<ValueRef> {
        if self.is_declared_in_current_scope(name) {
            self.semantic_error(format!("Variable '{}' already declared", name));
            return None;
        }

        let value_ref = self.emit_expr(value)?;
        let info = self.allocate_storage(&value_ref);
        self.bind_current_scope(name.to_string(), info);
        Some(value_ref)
    }

    pub(in crate::codegen::llvm) fn emit_assign(
        &mut self,
        name: &str,
        value: &crate::parser::expression::Expr,
    ) -> Option<ValueRef> {
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
            self.bind_scope(scope_index, name.to_string(), info);
            Some(value_ref)
        }
    }

    pub(in crate::codegen::llvm) fn emit_print_value(&mut self, value_ref: &ValueRef) {
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
            ValueType::Null | ValueType::Function | ValueType::Struct(_) => {
                self.semantic_error(format!(
                    "Function 'print' cannot print values of type {}.",
                    value_ref.value_type.display_name()
                ));
            }
        }
    }
}
