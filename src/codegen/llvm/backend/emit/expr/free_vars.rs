//! Recolección de variables libres de una lambda: variables referenciadas en
//! el cuerpo que no están ligadas dentro (parámetros, let, for, inits de
//! arreglo, lambdas anidadas) y sí existen en los scopes del backend. Son
//! exactamente los valores que el closure captura por valor.

use std::collections::HashSet;

use crate::parser::expression::{AssignTarget, Expr, Statement};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueType, VariableInfo};

impl LlvmBackend {
    /// Recolecta las variables libres de `expr` (no ligadas dentro de la
    /// lambda) que existen en los scopes actuales del backend.
    pub(super) fn collect_free_vars(
        &self,
        expr: &Expr,
        bound: &HashSet<String>,
        captures: &mut Vec<(String, VariableInfo)>,
    ) {
        match expr {
            Expr::Variable { name, .. } => {
                if !bound.contains(name) && !captures.iter().any(|(n, _)| n == name) {
                    if let Some(info) = self.lookup_var(name) {
                        captures.push((name.clone(), info));
                    }
                }
            }
            Expr::Binary(binary) => {
                self.collect_free_vars(&binary.left, bound, captures);
                self.collect_free_vars(&binary.right, bound, captures);
            }
            Expr::Unary(unary) => self.collect_free_vars(&unary.expr, bound, captures),
            Expr::BuiltinCall(call) => {
                for arg in &call.args {
                    self.collect_free_vars(arg, bound, captures);
                }
            }
            Expr::FunctionCall(call) => {
                // El nombre puede referirse a un valor función capturable.
                if !bound.contains(&call.name)
                    && !captures.iter().any(|(n, _)| n == &call.name)
                {
                    if let Some(info) = self.lookup_var(&call.name) {
                        if matches!(info.value_type, ValueType::Function(_)) {
                            captures.push((call.name.clone(), info));
                        }
                    }
                }
                for arg in &call.args {
                    self.collect_free_vars(arg, bound, captures);
                }
            }
            Expr::BaseCall(call) => {
                for arg in &call.args {
                    self.collect_free_vars(arg, bound, captures);
                }
            }
            Expr::MethodCall(call) => {
                self.collect_free_vars(&call.receiver, bound, captures);
                for arg in &call.args {
                    self.collect_free_vars(arg, bound, captures);
                }
            }
            Expr::MemberAccess(access) => {
                self.collect_free_vars(&access.object, bound, captures)
            }
            Expr::New(new_expr) => {
                for arg in &new_expr.args {
                    self.collect_free_vars(arg, bound, captures);
                }
            }
            Expr::DestructiveAssign(assign) => {
                match &assign.target {
                    AssignTarget::Variable { name, .. } => {
                        if !bound.contains(name) && !captures.iter().any(|(n, _)| n == name) {
                            if let Some(info) = self.lookup_var(name) {
                                captures.push((name.clone(), info));
                            }
                        }
                    }
                    AssignTarget::Member { object, .. } => {
                        self.collect_free_vars(object, bound, captures)
                    }
                    AssignTarget::Index { object, index, .. } => {
                        self.collect_free_vars(object, bound, captures);
                        self.collect_free_vars(index, bound, captures);
                    }
                }
                self.collect_free_vars(&assign.value, bound, captures);
            }
            Expr::LetIn(let_in) => {
                let mut inner = bound.clone();
                for binding in &let_in.bindings {
                    self.collect_free_vars(&binding.value, &inner, captures);
                    inner.insert(binding.name.clone());
                }
                self.collect_free_vars(&let_in.body, &inner, captures);
            }
            Expr::Block(block) => {
                let mut inner = bound.clone();
                for statement in &block.statements {
                    match statement {
                        Statement::Let { name, value, .. } => {
                            self.collect_free_vars(value, &inner, captures);
                            inner.insert(name.clone());
                        }
                        Statement::Assign { name, value, .. } => {
                            if !inner.contains(name)
                                && !captures.iter().any(|(n, _)| n == name)
                            {
                                if let Some(info) = self.lookup_var(name) {
                                    captures.push((name.clone(), info));
                                }
                            }
                            self.collect_free_vars(value, &inner, captures);
                        }
                        Statement::Print { value, .. } | Statement::Expr { value, .. } => {
                            self.collect_free_vars(value, &inner, captures);
                        }
                    }
                }
            }
            Expr::While(while_expr) => {
                self.collect_free_vars(&while_expr.condition, bound, captures);
                self.collect_free_vars(
                    &Expr::Block(while_expr.body.clone()),
                    bound,
                    captures,
                );
            }
            Expr::For(for_expr) => {
                self.collect_free_vars(&for_expr.iter, bound, captures);
                let mut inner = bound.clone();
                inner.insert(for_expr.id.clone());
                self.collect_free_vars(&Expr::Block(for_expr.body.clone()), &inner, captures);
            }
            Expr::If(if_expr) => {
                self.collect_free_vars(&if_expr.condition, bound, captures);
                self.collect_free_vars(&if_expr.then_branch, bound, captures);
                for elif in &if_expr.elif_branches {
                    self.collect_free_vars(&elif.condition, bound, captures);
                    self.collect_free_vars(&elif.body, bound, captures);
                }
                self.collect_free_vars(&if_expr.else_branch, bound, captures);
            }
            Expr::Is(is_expr) => self.collect_free_vars(&is_expr.expr, bound, captures),
            Expr::As(as_expr) => self.collect_free_vars(&as_expr.expr, bound, captures),
            Expr::ArrayLiteral(literal) => {
                for element in &literal.elements {
                    self.collect_free_vars(element, bound, captures);
                }
            }
            Expr::NewArray(new_array) => {
                self.collect_free_vars(&new_array.size, bound, captures);
                if let Some(init) = &new_array.init {
                    let mut inner = bound.clone();
                    inner.insert(init.var_name.clone());
                    self.collect_free_vars(&init.body, &inner, captures);
                }
            }
            Expr::Index(index_expr) => {
                self.collect_free_vars(&index_expr.object, bound, captures);
                self.collect_free_vars(&index_expr.index, bound, captures);
            }
            Expr::Lambda(lambda) => {
                let mut inner = bound.clone();
                for param in &lambda.params {
                    inner.insert(param.name.clone());
                }
                self.collect_free_vars(&lambda.body, &inner, captures);
            }
            Expr::Literal { .. } => {}
        }
    }
}
