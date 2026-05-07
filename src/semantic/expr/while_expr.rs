use crate::parser::expression::WhileExpr;

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_while_expr(
        &mut self,
        while_expr: &WhileExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let mut condition_type = self.check_expr(&while_expr.condition, source)?;
        if condition_type == SemanticType::Unknown {
            condition_type =
                self.constrain_expr_type(&while_expr.condition, SemanticType::Boolean, source);
        }

        if condition_type == SemanticType::Unknown {
            return Some(SemanticType::Unknown);
        }

        if condition_type != SemanticType::Boolean {
            self.push_type_error(
                while_expr.condition.span(),
                source,
                format!(
                    "While condition expects Boolean, but got {}.",
                    condition_type.display_name()
                ),
            );
            return None;
        }

        if self.check_block_expr(&while_expr.body, source).is_none() {
            return Some(SemanticType::Unknown);
        }

        Some(SemanticType::Unit)
    }
}
