//! Sistema de scopes y almacenamiento de variables: cada variable vive en un
//! `alloca` tipado y los scopes anidados forman una pila de tablas
//! nombre → [`VariableInfo`].

use std::collections::HashMap;

use crate::codegen::llvm::helper::state::{ValueRef, ValueType, VariableInfo};

use super::LlvmBackend;

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(in crate::codegen::llvm) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(in crate::codegen::llvm) fn is_declared_in_current_scope(&self, name: &str) -> bool {
        self.scopes
            .last()
            .map(|scope| scope.contains_key(name))
            .unwrap_or(false)
    }

    pub(in crate::codegen::llvm) fn lookup_var(&self, name: &str) -> Option<VariableInfo> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    pub(in crate::codegen::llvm) fn lookup_var_with_index(
        &self,
        name: &str,
    ) -> Option<(usize, VariableInfo)> {
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(idx, scope)| scope.get(name).cloned().map(|info| (idx, info)))
    }

    pub(in crate::codegen::llvm) fn bind_current_scope(&mut self, name: String, info: VariableInfo) {
        self.scopes
            .last_mut()
            .expect("a scope should always be present")
            .insert(name, info);
    }

    pub(in crate::codegen::llvm) fn bind_scope(
        &mut self,
        scope_index: usize,
        name: String,
        info: VariableInfo,
    ) {
        self.scopes[scope_index].insert(name, info);
    }

    pub(in crate::codegen::llvm) fn allocate_storage(&mut self, value_ref: &ValueRef) -> VariableInfo {
        self.allocate_storage_typed(value_ref.value_type, value_ref)
            .expect("value type should always be compatible with itself")
    }

    pub(in crate::codegen::llvm) fn allocate_storage_typed(
        &mut self,
        expected_type: ValueType,
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

    pub(in crate::codegen::llvm) fn store_value_at(&mut self, ptr_name: &str, value_ref: &ValueRef) {
        self.store_value_at_typed(ptr_name, value_ref.value_type, value_ref);
    }

    pub(in crate::codegen::llvm) fn store_value_at_typed(
        &mut self,
        ptr_name: &str,
        expected_type: ValueType,
        value_ref: &ValueRef,
    ) {
        let llvm_ty = expected_type.llvm_type();
        let repr = self
            .value_repr_for_expected_type(expected_type, value_ref)
            .unwrap_or_else(|| value_ref.repr.clone());
        self.emit_body(format!("store {llvm_ty} {}, {llvm_ty}* {ptr_name}", repr));
    }
}
