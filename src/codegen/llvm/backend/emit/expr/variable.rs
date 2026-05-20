use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::ValueRef;

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_variable(&mut self, name: &str) -> Option<ValueRef> {
        let Some(info) = self.lookup_var(name) else {
            self.semantic_error(format!("Variable '{}' is not declared", name));
            return None;
        };

        let loaded = self.next_temp();
        let llvm_ty = info.value_type.llvm_type();
        self.emit_body(format!(
            "{loaded} = load {llvm_ty}, {llvm_ty}* {}",
            info.ptr_name
        ));

        Some(ValueRef {
            value_type: info.value_type,
            repr: loaded,
        })
    }
}
