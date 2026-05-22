use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_variable_expr(
        &mut self,
        name: &str,
        span: Span,
        source: &str,
    ) -> Option<SemanticType> {
        self.check_variable(name, span, source)
    }

    fn check_variable(&mut self, name: &str, span: Span, source: &str) -> Option<SemanticType> {
        if let Some(var_type) = self.analyzer.lookup(name) {
            Some(var_type)
        } else {
            self.analyzer.push_semantic_error(
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
