use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_literal_expr(&self, literal: &Literal) -> Option<SemanticType> {
        Some(self.check_literal(literal))
    }

    fn check_literal(&self, literal: &Literal) -> SemanticType {
        match literal {
            Literal::Integer(_) | Literal::Float(_) => SemanticType::Number,
            Literal::Boolean(_) => SemanticType::Boolean,
            Literal::String(_) => SemanticType::String,
            Literal::Null => SemanticType::Null,
        }
    }
}
