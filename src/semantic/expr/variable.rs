use crate::parser::expression::Span;

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_variable(
        &mut self,
        name: &str,
        span: Span,
        source: &str,
    ) -> Option<SemanticType> {
        if let Some(var_type) = self.lookup(name) {
            Some(var_type)
        } else {
            self.push_semantic_error(
                span,
                source,
                format!(
                    "Variable '{}' is used before declaration. Declare it with 'let' first.",
                    name
                ),
            );
            None
        }
    }
}
