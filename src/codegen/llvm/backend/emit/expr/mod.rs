mod as_expr;
mod binary;
mod block;
mod call;
mod destructive_assign;
mod if_expr;
mod is_expr;
mod let_in;
mod literal;
mod member_access;
mod new_expr;
mod unary;
mod variable;
mod while_expr;

use crate::parser::expression::Expr;

use super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::ValueRef;

pub(in crate::codegen::llvm) fn emit_expr(
    backend: &mut LlvmBackend,
    expr: &Expr,
) -> Option<ValueRef> {
    match expr {
        Expr::Literal { value, .. } => backend.emit_literal(value),
        Expr::Variable { name, .. } => backend.emit_variable(name),
        Expr::Unary(unary) => backend.emit_unary_expr(unary),
        Expr::Block(block) => backend.emit_block_expr(block),
        Expr::DestructiveAssign(assign) => backend.emit_destructive_assign(assign),
        Expr::LetIn(let_in) => backend.emit_let_in_expr(let_in),
        Expr::While(while_expr) => backend.emit_while_expr(while_expr),
        Expr::If(if_expr) => backend.emit_if_expr(if_expr),
        Expr::BuiltinCall(call) => backend.emit_builtin_call(call.function, &call.args),
        Expr::FunctionCall(call) => backend.emit_function_call(call),
        Expr::MethodCall(call) => backend.emit_method_call(call),
        Expr::MemberAccess(access) => backend.emit_member_access(access),
        Expr::New(new_expr) => backend.emit_new_expr(new_expr),
        Expr::Binary(binary) => backend.emit_binary_expr(binary),
        Expr::Is(is_expr) => backend.emit_is_expr(is_expr),
        Expr::As(as_expr) => backend.emit_as_expr(as_expr),
    }
}
