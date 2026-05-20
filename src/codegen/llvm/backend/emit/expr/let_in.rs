use crate::parser::expression::LetInExpr;

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::ValueRef;

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_let_in_expr(
        &mut self,
        let_in: &LetInExpr,
    ) -> Option<ValueRef> {
        self.push_scope();

        for binding in &let_in.bindings {
            if self.is_declared_in_current_scope(&binding.name) {
                self.semantic_error(format!(
                    "Variable '{}' redeclared in let-in binding",
                    binding.name
                ));
                continue;
            }

            let value_ref = self.emit_expr(&binding.value)?;
            let info = self.allocate_storage(&value_ref);
            self.bind_current_scope(binding.name.clone(), info);
        }

        let body_value = self.emit_expr(&let_in.body);

        self.pop_scope();

        body_value
    }
}
