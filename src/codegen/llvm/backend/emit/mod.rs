pub(super) mod expr;
mod function;
mod method;
mod statement;

use crate::parser::expression::{Expr, Program};

use super::LlvmBackend;
use crate::codegen::llvm::helper::state::ValueRef;

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_program(&mut self, program: &Program) {
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

    pub(in crate::codegen::llvm) fn emit_expr(&mut self, expr: &Expr) -> Option<ValueRef> {
        expr::emit_expr(self, expr)
    }
}
