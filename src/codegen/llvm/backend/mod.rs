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

use super::helper::state::{ValueRef, ValueType, VariableInfo};
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
    pub(super) interface_real_types: HashMap<String, u32>,
    pub(super) param_real_types: HashMap<String, u32>,
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
        self.interface_real_types.clear();
        self.param_real_types.clear();
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

    pub(super) fn type_name_for_value_type(&self, vt: ValueType) -> String {
        match vt {
            ValueType::Struct(id) => {
                self.type_ids
                    .iter()
                    .find(|(_, tid)| **tid == id)
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| "Struct".to_string())
            }
            _ => vt.display_name().to_string(),
        }
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
        &self,
        expected: super::helper::state::ValueType,
        actual: super::helper::state::ValueType,
    ) -> bool {
        if expected == actual {
            return true;
        }

        if actual == super::helper::state::ValueType::Null
            && Self::is_nullable_value_type(expected)
        {
            return true;
        }

        if expected == super::helper::state::ValueType::Null
            && Self::is_nullable_value_type(actual)
        {
            return true;
        }

        match (expected, actual) {
            (
                super::helper::state::ValueType::Struct(parent),
                super::helper::state::ValueType::Struct(child),
            ) => self.is_subtype_struct(child, parent),
            _ => false,
        }
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

        if self.are_compatible_value_types(expected, value_ref.value_type)
            && expected.llvm_type() == value_ref.value_type.llvm_type()
        {
            return Some(value_ref.repr.clone());
        }

        None
    }

    pub(super) fn is_subtype_struct(&self, child: u32, parent: u32) -> bool {
        if child == parent {
            return true;
        }

        if self
            .type_ids
            .get("Object")
            .is_some_and(|object_id| *object_id == parent)
        {
            return true;
        }

        let mut cursor = self
            .type_ids
            .iter()
            .find_map(|(name, type_id)| (*type_id == child).then_some(name.as_str()))
            .and_then(|name| self.type_decls.get(name))
            .and_then(|decl| decl.parent_name.clone());

        while let Some(parent_name) = cursor {
            let Some(parent_id) = self.type_ids.get(&parent_name).copied() else {
                return false;
            };
            if parent_id == parent {
                return true;
            }
            cursor = self
                .type_decls
                .get(&parent_name)
                .and_then(|decl| decl.parent_name.clone());
        }

        false
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

    pub(super) fn emit_type_hierarchy_globals(&mut self) {
        let max_type_id = self.type_ids.values().copied().max().unwrap_or(8) + 1;
        let mut parent_entries = vec!["-1".to_string(); max_type_id as usize];

        if let Some(object_id) = self.type_ids.get("Object").copied() {
            parent_entries[object_id as usize] = "-1".to_string();
        }
        if let Some(iterable_id) = self.type_ids.get("Iterable").copied() {
            parent_entries[iterable_id as usize] = "-1".to_string();
        }
        if let Some(enumerable_id) = self.type_ids.get("Enumerable").copied() {
            parent_entries[enumerable_id as usize] = "-1".to_string();
        }
        if let Some(range_id) = self.type_ids.get("Range").copied() {
            if let Some(object_id) = self.type_ids.get("Object").copied() {
                parent_entries[range_id as usize] = object_id.to_string();
            }
        }
        if let Some(array_id) = self.type_ids.get("Array").copied() {
            if let Some(object_id) = self.type_ids.get("Object").copied() {
                parent_entries[array_id as usize] = object_id.to_string();
            }
        }

        for (name, type_id) in &self.type_ids {
            if let Some(decl) = self.type_decls.get(name) {
                if let Some(parent_name) = &decl.parent_name {
                    if let Some(parent_id) = self.type_ids.get(parent_name).copied() {
                        parent_entries[*type_id as usize] = parent_id.to_string();
                    }
                }
            }
        }

        let entries_str = parent_entries
            .iter()
            .map(|v| format!("i64 {v}"))
            .collect::<Vec<_>>()
            .join(", ");
        self.emit_global(format!(
            "@hulk_type_parents = internal global [{max_type_id} x i64] [{entries_str}]"
        ));

        self.emit_function_line(format!(
            "define i1 @hulk_is_subtype(i64 %child, i64 %parent) {{"
        ));
        self.emit_function_line("entry:".to_string());
        self.emit_function_line("  %cmp0 = icmp eq i64 %child, %parent".to_string());
        self.emit_function_line("  br i1 %cmp0, label %ret_true, label %walk".to_string());
        self.emit_function_line("walk:".to_string());
        self.emit_function_line(
            "  %current = phi i64 [ %child, %entry ], [ %parent_id, %check ]".to_string(),
        );
        self.emit_function_line(format!(
            "  %idx = getelementptr [{max_type_id} x i64], [{max_type_id} x i64]* @hulk_type_parents, i64 0, i64 %current"
        ));
        self.emit_function_line("  %parent_id = load i64, i64* %idx".to_string());
        self.emit_function_line("  %is_neg1 = icmp eq i64 %parent_id, -1".to_string());
        self.emit_function_line(
            "  br i1 %is_neg1, label %ret_false, label %check".to_string(),
        );
        self.emit_function_line("check:".to_string());
        self.emit_function_line("  %cmp_eq = icmp eq i64 %parent_id, %parent".to_string());
        self.emit_function_line(
            "  br i1 %cmp_eq, label %ret_true, label %walk".to_string(),
        );
        self.emit_function_line("ret_true:".to_string());
        self.emit_function_line("  ret i1 true".to_string());
        self.emit_function_line("ret_false:".to_string());
        self.emit_function_line("  ret i1 false".to_string());
        self.emit_function_line("}".to_string());
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
