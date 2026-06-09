use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_while_expr(
        &mut self,
        while_expr: &WhileExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let mut condition_type = self.check_expr(&while_expr.condition, source)?;
        if condition_type == SemanticType::Unknown {
            condition_type = TypeConstraintEngine::constrain_expr_type(
                self,
                &while_expr.condition,
                SemanticType::Boolean,
                source,
            );
        }

        if condition_type == SemanticType::Unknown {
            return Some(SemanticType::Unknown);
        }

        if condition_type != SemanticType::Boolean {
            self.analyzer.push_type_error(
                while_expr.condition.span(),
                source,
                format!(
                    "While condition expects Boolean, but got {}.",
                    condition_type.display_name_with_table(&self.analyzer.type_table)
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
