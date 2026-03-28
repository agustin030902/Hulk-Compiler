use crate::parser::expression::Literal;

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_literal(&self, literal: &Literal) -> SemanticType {
        match literal {
            Literal::Integer(_) | Literal::Float(_) => SemanticType::Number,
            Literal::Boolean(_) => SemanticType::Boolean,
            Literal::String(_) => SemanticType::String,
        }
    }
}
