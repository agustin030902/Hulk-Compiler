use crate::parser::expression::{
    BlockExpr, Expr, ForExpr, LetBinding, LetInExpr, MethodCallExpr, Statement, WhileExpr,
};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::ValueRef;
use crate::parser::expression::Span;

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_for_expr(
        &mut self,
        for_expr: &ForExpr,
    ) -> Option<ValueRef> {
        let zero = Span::new(0, 0);
        let iter_var_name = "__hulk_iter__".to_string();
        let iter_ref = Expr::Variable {
            name: iter_var_name.clone(),
            span: zero,
        };

        let iter_expr = if self.has_iter_method(&for_expr.iter) {
            Expr::MethodCall(MethodCallExpr {
                receiver: for_expr.iter.clone(),
                method_name: "iter".to_string(),
                method_name_span: zero,
                args: Vec::new(),
                span: zero,
            })
        } else {
            (*for_expr.iter).clone()
        };

        let condition = Expr::MethodCall(MethodCallExpr {
            receiver: Box::new(iter_ref.clone()),
            method_name: "next".to_string(),
            method_name_span: zero,
            args: Vec::new(),
            span: zero,
        });

        let current_value = Expr::MethodCall(MethodCallExpr {
            receiver: Box::new(iter_ref),
            method_name: "current".to_string(),
            method_name_span: zero,
            args: Vec::new(),
            span: zero,
        });

        let inner_binding = LetBinding {
            name: for_expr.id.clone(),
            type_annotation: None,
            value: current_value,
            span: zero,
        };

        let inner_let_in = Expr::LetIn(LetInExpr {
            bindings: vec![inner_binding],
            body: Box::new(Expr::Block(for_expr.body.clone())),
            span: zero,
        });

        let while_body = BlockExpr {
            statements: vec![Statement::Expr {
                value: inner_let_in,
                span: zero,
            }],
            span: zero,
        };

        let while_loop = Expr::While(WhileExpr {
            condition: Box::new(condition),
            body: while_body,
            span: zero,
        });

        let outer_binding = LetBinding {
            name: iter_var_name,
            type_annotation: None,
            value: iter_expr,
            span: zero,
        };

        let let_in = Expr::LetIn(LetInExpr {
            bindings: vec![outer_binding],
            body: Box::new(while_loop),
            span: zero,
        });

        self.emit_expr(&let_in)
    }

    fn has_iter_method(&mut self, expr: &Expr) -> bool {
        let Expr::Variable { name, .. } = expr else {
            return false;
        };
        let Some(var_type) = self.lookup_variable_type(name) else {
            return false;
        };
        let crate::codegen::llvm::helper::state::ValueType::Struct(type_id) = var_type else {
            return false;
        };
        self.lookup_method_key(type_id, "iter").is_some()
    }

    fn lookup_variable_type(&mut self, name: &str) -> Option<crate::codegen::llvm::helper::state::ValueType> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info.value_type);
            }
        }
        None
    }
}
