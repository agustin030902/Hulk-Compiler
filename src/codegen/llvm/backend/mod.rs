mod emit;
mod functions;
mod layout;
mod type_lowering;

use std::collections::HashMap;

use crate::{
    codegen::CodegenBackend,
    error::{CompilerError, ErrorCategory},
    parser::expression::{Program, TypeDecl},
};

use super::helper::state::{ValueRef, VariableInfo};
use functions::FunctionInfo;
use layout::StructLayout;

#[derive(Debug, Default)]
pub struct LlvmBackend {
    pub(super) body_lines: Vec<String>,
    pub(super) function_lines: Vec<String>,
    pub(super) global_lines: Vec<String>,
    pub(super) errors: Vec<CompilerError>,
    pub(super) scopes: Vec<HashMap<String, VariableInfo>>,
    pub(super) functions: HashMap<String, FunctionInfo>,
    pub(super) type_ids: HashMap<String, u32>,
    pub(super) type_decls: HashMap<String, TypeDecl>,
    pub(super) struct_layouts: HashMap<u32, StructLayout>,
    pub(super) method_dispatch: HashMap<(u32, String), String>,
    pub(super) temp_counter: usize,
    pub(super) label_counter: usize,
    pub(super) string_counter: usize,
    pub(crate) current_block: String,
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
        self.type_ids.clear();
        self.type_decls.clear();
        self.struct_layouts.clear();
        self.method_dispatch.clear();
        self.temp_counter = 0;
        self.label_counter = 0;
        self.string_counter = 0;
        self.push_scope();
    }

    pub(super) fn emit_body(&mut self, line: impl Into<String>) {
        let line = line.into();
    
        if line.ends_with(':') {
            self.current_block =
                line.trim_end_matches(':').to_string();
        }
    
        self.body_lines.push(line);
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
        self.allocate_storage_typed(value_ref.value_type, value_ref)
            .expect("value type should always be compatible with itself")
    }

    pub(super) fn allocate_storage_typed(
        &mut self,
        expected_type: super::helper::state::ValueType,
        value_ref: &ValueRef,
    ) -> Option<VariableInfo> {
        let repr = self.value_repr_for_expected_type(expected_type, value_ref)?;
        let ptr_name = self.next_temp();
        let llvm_ty = expected_type.llvm_type();
        self.emit_body(format!("{ptr_name} = alloca {llvm_ty}"));
        self.emit_body(format!("store {llvm_ty} {repr}, {llvm_ty}* {ptr_name}"));

        Some(VariableInfo {
            ptr_name,
            value_type: expected_type,
        })
    }

    pub(super) fn store_value_at(&mut self, ptr_name: &str, value_ref: &ValueRef) {
        self.store_value_at_typed(ptr_name, value_ref.value_type, value_ref);
    }

    pub(super) fn store_value_at_typed(
        &mut self,
        ptr_name: &str,
        expected_type: super::helper::state::ValueType,
        value_ref: &ValueRef,
    ) {
        let llvm_ty = expected_type.llvm_type();
        let repr = self
            .value_repr_for_expected_type(expected_type, value_ref)
            .unwrap_or_else(|| value_ref.repr.clone());
        self.emit_body(format!(
            "store {llvm_ty} {}, {llvm_ty}* {ptr_name}",
            repr
        ));
    }

    pub(super) fn is_nullable_value_type(
        value_type: super::helper::state::ValueType,
    ) -> bool {
        matches!(
            value_type,
            super::helper::state::ValueType::Null
                | super::helper::state::ValueType::StringPtr
                | super::helper::state::ValueType::Function
                | super::helper::state::ValueType::Struct(_)
        )
    }

    pub(super) fn are_compatible_value_types(
        expected: super::helper::state::ValueType,
        actual: super::helper::state::ValueType,
    ) -> bool {
        expected == actual
            || (actual == super::helper::state::ValueType::Null
                && Self::is_nullable_value_type(expected))
            || (expected == super::helper::state::ValueType::Null
                && Self::is_nullable_value_type(actual))
    }

    pub(super) fn value_repr_for_expected_type(
        &self,
        expected: super::helper::state::ValueType,
        value_ref: &ValueRef,
    ) -> Option<String> {
        if value_ref.value_type == expected {
            return Some(value_ref.repr.clone());
        }

        if value_ref.value_type == super::helper::state::ValueType::Null
            && Self::is_nullable_value_type(expected)
        {
            return Some("null".to_string());
        }

        None
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

        if !self.load_function_signatures(program) {
            return Err(self.errors.clone());
        }

        self.emit_program(program);

        if self.errors.is_empty() {
            Ok(self.compose_module())
        } else {
            Err(self.errors.clone())
        }
    }
}
