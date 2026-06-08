use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_unary_expr(
        &mut self,
        unary: &UnaryExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let mut expr_type = self.check_expr(&unary.expr, source)?;

        match unary.op {
            UnaryOp::Neg => {
                if expr_type == SemanticType::Unknown {
                    expr_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &unary.expr,
                        SemanticType::Number,
                        source,
                    );
                }

                if expr_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if expr_type == SemanticType::Number {
                    Some(SemanticType::Number)
                } else {
                    self.analyzer.push_type_error(
                        unary.span,
                        source,
                        format!(
                            "Unary '-' expects Number, but got {}.",
                            expr_type.display_name_with_table(&self.analyzer.type_table)
                        ),
                    );
                    None
                }
            }
            UnaryOp::Not => {
                if expr_type == SemanticType::Unknown {
                    expr_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &unary.expr,
                        SemanticType::Boolean,
                        source,
                    );
                }

                if expr_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if expr_type == SemanticType::Boolean {
                    Some(SemanticType::Boolean)
                } else {
                    self.analyzer.push_type_error(
                        unary.span,
                        source,
                        format!(
                            "Unary '!' expects Boolean, but got {}.",
                            expr_type.display_name_with_table(&self.analyzer.type_table)
                        ),
                    );
                    None
                }
            }
        }
    }
}
