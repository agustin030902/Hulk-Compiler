mod binary;
mod block;
mod builtin_call;
mod destructive_assign;
mod function_call;
mod if_expr;
mod let_in;
mod literal;
mod unary;
mod variable;
mod while_expr;

use crate::parser::expression::Expr;

use super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_expr(&mut self, expr: &Expr, source: &str) -> Option<SemanticType> {
        match expr {
            Expr::Literal { value, .. } => Some(self.check_literal(value)),
            Expr::DestructiveAssign(assign) => self.check_destructive_assign(assign, source),
            Expr::Variable { name, span } => self.check_variable(name, *span, source),
            Expr::Unary(unary) => self.check_unary_expr(unary, source),
            Expr::Block(block) => self.check_block_expr(block, source),
            Expr::LetIn(let_in) => self.check_let_in_expr(let_in, source),
            Expr::While(while_expr) => self.check_while_expr(while_expr, source),
            Expr::If(if_expr) => self.check_if_expr(if_expr, source),
            Expr::BuiltinCall(call) => {
                self.check_builtin_call(call.function, &call.args, call.span, source)
            }
            Expr::FunctionCall(call) => self.check_function_call(call, source),
            Expr::Binary(binary) => self.check_binary_expr(binary, source),
        }
    }
}
