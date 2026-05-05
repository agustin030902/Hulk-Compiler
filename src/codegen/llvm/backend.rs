use std::collections::HashMap;

use crate::{
    codegen::CodegenBackend,
    error::{CompilerError, ErrorCategory},
    parser::expression::Program,
};

use super::helper::state::{ValueRef, ValueType, VariableInfo};

#[derive(Debug, Clone, Copy)]
pub(super) struct FunctionInfo {
    pub(super) arity: usize,
    pub(super) return_type: ValueType,
}

#[derive(Debug, Default)]
pub struct LlvmBackend {
    pub(super) body_lines: Vec<String>,
    pub(super) function_lines: Vec<String>,
    pub(super) global_lines: Vec<String>,
    pub(super) errors: Vec<CompilerError>,
    pub(super) scopes: Vec<HashMap<String, VariableInfo>>,
    pub(super) functions: HashMap<String, FunctionInfo>,
    pub(super) temp_counter: usize,
    pub(super) label_counter: usize,
    pub(super) string_counter: usize,
}

impl LlvmBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub(super) fn reset(&mut self) {
        self.body_lines.clear();
        self.function_lines.clear();
        self.global_lines.clear();
        self.errors.clear();
        self.scopes.clear();
        self.functions.clear();
        self.temp_counter = 0;
        self.label_counter = 0;
        self.string_counter = 0;
        self.push_scope();
    }

    pub(super) fn emit_body(&mut self, line: impl Into<String>) {
        self.body_lines.push(line.into());
    }

    pub(super) fn emit_function_line(&mut self, line: impl Into<String>) {
        self.function_lines.push(line.into());
    }

    pub(super) fn emit_global(&mut self, line: impl Into<String>) {
        self.global_lines.push(line.into());
    }

    pub(super) fn next_temp(&mut self) -> String {
        let current = self.temp_counter;
        self.temp_counter += 1;
        format!("%t{}", current)
    }

    pub(super) fn next_label(&mut self, prefix: &str) -> String {
        let current = self.label_counter;
        self.label_counter += 1;
        format!("{prefix}.{current}")
    }

    pub(super) fn next_string_name(&mut self) -> String {
        let current = self.string_counter;
        self.string_counter += 1;
        format!("@.str.{}", current)
    }

    pub(super) fn semantic_error(&mut self, message: impl Into<String>) {
        self.errors
            .push(CompilerError::new(ErrorCategory::Semantic, message, 1, 1));
    }

    pub(super) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn is_declared_in_current_scope(&self, name: &str) -> bool {
        self.scopes
            .last()
            .map(|scope| scope.contains_key(name))
            .unwrap_or(false)
    }

    pub(super) fn lookup_var(&self, name: &str) -> Option<VariableInfo> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    pub(super) fn lookup_var_with_index(&self, name: &str) -> Option<(usize, VariableInfo)> {
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(idx, scope)| scope.get(name).cloned().map(|info| (idx, info)))
    }

    pub(super) fn allocate_storage(&mut self, value_ref: &ValueRef) -> VariableInfo {
        let ptr_name = self.next_temp();
        let llvm_ty = value_ref.value_type.llvm_type();
        self.emit_body(format!("{ptr_name} = alloca {llvm_ty}"));
        self.emit_body(format!(
            "store {llvm_ty} {}, {llvm_ty}* {ptr_name}",
            value_ref.repr
        ));

        VariableInfo {
            ptr_name,
            value_type: value_ref.value_type,
        }
    }

    pub(super) fn store_value_at(&mut self, ptr_name: &str, value_ref: &ValueRef) {
        let llvm_ty = value_ref.value_type.llvm_type();
        self.emit_body(format!(
            "store {llvm_ty} {}, {llvm_ty}* {ptr_name}",
            value_ref.repr
        ));
    }

    pub(super) fn bind_current_scope(&mut self, name: String, info: VariableInfo) {
        self.scopes
            .last_mut()
            .expect("a scope should always be present")
            .insert(name, info);
    }

    pub(super) fn bind_scope(&mut self, scope_index: usize, name: String, info: VariableInfo) {
        self.scopes[scope_index].insert(name, info);
    }
}

impl CodegenBackend for LlvmBackend {
    fn generate(&mut self, program: &Program) -> Result<String, Vec<CompilerError>> {
        self.reset();
        self.emit_program(program);

        if self.errors.is_empty() {
            Ok(self.compose_module())
        } else {
            Err(self.errors.clone())
        }
    }
}
