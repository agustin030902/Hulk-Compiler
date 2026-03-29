mod binary;
mod block;
mod builtin_call;
mod destructive_assign;
mod let_in;
mod literal;
mod unary;
mod variable;
mod while_expr;

use crate::parser::expression::Expr;

use super::{backend::LlvmBackend, helper::state::ValueRef};

impl LlvmBackend {
    pub(super) fn emit_expr(&mut self, expr: &Expr) -> Option<ValueRef> {
        match expr {
            Expr::Literal { value, .. } => self.emit_literal(value),
            Expr::Variable { name, .. } => self.emit_variable(name),
            Expr::Unary(unary) => self.emit_unary_expr(unary),
            Expr::Block(block) => self.emit_block_expr(block),
            Expr::DestructiveAssign(assign) => self.emit_destructive_assign(assign),
            Expr::LetIn(let_in) => self.emit_let_in_expr(let_in),
            Expr::While(while_expr) => self.emit_while_expr(while_expr),
            Expr::BuiltinCall(call) => self.emit_builtin_call(call.function, &call.args),
            Expr::Binary(binary) => self.emit_binary_expr(binary),
        }
    }
}
