pub(super) mod expr;
mod function;
mod method;
mod statement;

use crate::parser::expression::{Expr, Program};
use crate::semantic::builtins;

use super::LlvmBackend;
use crate::codegen::llvm::helper::state::ValueRef;

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_program(&mut self, program: &Program) {
        let mut emitted_types: Vec<String> = Vec::new();
        for type_decl in &program.types {
            emitted_types.push(type_decl.name.clone());
            for method in &type_decl.methods {
                self.emit_method_decl(type_decl, method);
            }
        }

        if !emitted_types.iter().any(|n| n == builtins::RANGE_NAME) {
            let range_decl = builtins::build_range_type_decl();
            for method in &range_decl.methods {
                self.emit_method_decl(&range_decl, method);
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
