use crate::parser::expression::{ElifBranch, Expr, IfExpr};

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_if_expr(&mut self, if_expr: &IfExpr, source: &str) -> Option<SemanticType> {
        let condition_ok = self.check_condition(&if_expr.condition, source);

        let then_type = self.check_expr(&if_expr.then_branch, source);
        let mut branch_types = Vec::with_capacity(if_expr.elif_branches.len() + 2);
        if let Some(value_type) = then_type {
            branch_types.push((value_type, if_expr.then_branch.span()));
        }

        let mut elif_conditions_ok = true;
        for branch in &if_expr.elif_branches {
            elif_conditions_ok &= self.check_elif_branch_condition(branch, source);
            if let Some(value_type) = self.check_expr(&branch.body, source) {
                branch_types.push((value_type, branch.body.span()));
            }
        }

        if let Some(value_type) = self.check_expr(&if_expr.else_branch, source) {
            branch_types.push((value_type, if_expr.else_branch.span()));
        }

        if !condition_ok || !elif_conditions_ok {
            return Some(SemanticType::Unknown);
        }

        if branch_types
            .iter()
            .any(|(value_type, _)| *value_type == SemanticType::Unknown)
        {
            return Some(SemanticType::Unknown);
        }

        let Some((expected_type, _)) = branch_types.first().copied() else {
            return Some(SemanticType::Unknown);
        };

        for (actual_type, span) in branch_types.iter().skip(1).copied() {
            if actual_type != expected_type {
                self.push_type_error(
                    span,
                    source,
                    format!(
                        "If branches must return the same type, but got {} and {}.",
                        expected_type.display_name(),
                        actual_type.display_name()
                    ),
                );
                return None;
            }
        }

        Some(expected_type)
    }

    fn check_elif_branch_condition(&mut self, branch: &ElifBranch, source: &str) -> bool {
        self.check_condition(&branch.condition, source)
    }

    fn check_condition(&mut self, condition: &Expr, source: &str) -> bool {
        match self.check_expr(condition, source) {
            Some(SemanticType::Boolean) => true,
            Some(SemanticType::Unknown) => false,
            Some(condition_type) => {
                self.push_type_error(
                    condition.span(),
                    source,
                    format!(
                        "If condition expects Boolean, but got {}.",
                        condition_type.display_name()
                    ),
                );
                false
            }
            None => false,
        }
    }
}
