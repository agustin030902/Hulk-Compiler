use crate::parser::expression::NewArrayExpr;

use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_new_array(
        &mut self,
        expr: &NewArrayExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let element_type = TypeResolver::resolve_named_type(self.analyzer, &expr.type_name)?;

        for size in &expr.sizes {
            let size_type = self.check_expr(size, source)?;
            if size_type != SemanticType::Number {
                self.analyzer.push_type_error(
                    expr.span,
                    source,
                    format!(
                        "Array size must be Number, but got {}.",
                        size_type.display_name_with_table(&self.analyzer.type_table)
                    ),
                );
                return None;
            }
        }

        if let Some(initializer) = &expr.initializer {
            let param_type = self
                .analyzer
                .lookup(&initializer.param_name)
                .unwrap_or(SemanticType::Number);
            self.analyzer.push_scope();
            self.analyzer
                .bind_current_scope(initializer.param_name.clone(), param_type.clone());
            let body_type = self.check_expr(&initializer.body, source)?;
            self.analyzer.pop_scope();

            if body_type != SemanticType::Unknown
                && !self.types_compatible(element_type.clone(), body_type.clone())
            {
                self.analyzer.push_type_error(
                    initializer.span,
                    source,
                    format!(
                        "Array initializer must return {}, but returns {}.",
                        element_type.display_name_with_table(&self.analyzer.type_table),
                        body_type.display_name_with_table(&self.analyzer.type_table)
                    ),
                );
            }
        }

        let mut result = element_type;
        for _ in 0..expr.sizes.len() {
            result = SemanticType::Array(Box::new(result));
        }

        Some(result)
    }
}
