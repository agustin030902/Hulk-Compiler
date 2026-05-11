use crate::parser::expression::{ElifBranch, Expr, IfExpr};

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_if_expr(&mut self, if_expr: &IfExpr, source: &str) -> Option<SemanticType> {
        let condition_ok = self.check_condition(&if_expr.condition, source);

        let then_branch = if_expr.then_branch.as_ref();
        let mut branch_types = Vec::with_capacity(if_expr.elif_branches.len() + 2);
        if let Some(value_type) = self.check_expr(then_branch, source) {
            branch_types.push((then_branch, value_type));
        }

        let mut elif_conditions_ok = true;
        for branch in &if_expr.elif_branches {
            elif_conditions_ok &= self.check_elif_branch_condition(branch, source);
            if let Some(value_type) = self.check_expr(&branch.body, source) {
                branch_types.push((&branch.body, value_type));
            }
        }

        let else_branch = if_expr.else_branch.as_ref();
        if let Some(value_type) = self.check_expr(else_branch, source) {
            branch_types.push((else_branch, value_type));
        }

        if !condition_ok || !elif_conditions_ok {
            return Some(SemanticType::Unknown);
        }

        let Some(expected_type) = branch_types.iter().find_map(|(_, value_type)| {
            (*value_type != SemanticType::Unknown).then_some(*value_type)
        }) else {
            return Some(SemanticType::Unknown);
        };

        for (branch_expr, value_type) in &mut branch_types {
            if *value_type == SemanticType::Unknown {
                *value_type = self.constrain_expr_type(*branch_expr, expected_type, source);
            }
        }

        if branch_types
            .iter()
            .any(|(_, value_type)| *value_type == SemanticType::Unknown)
        {
            return Some(SemanticType::Unknown);
        }

        for (branch_expr, actual_type) in branch_types.iter().copied() {
            if actual_type != expected_type {
                self.push_type_error(
                    branch_expr.span(),
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
            Some(SemanticType::Unknown) => {
                self.constrain_expr_type(condition, SemanticType::Boolean, source)
                    == SemanticType::Boolean
            }
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
