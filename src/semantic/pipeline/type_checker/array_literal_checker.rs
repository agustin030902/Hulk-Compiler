use crate::parser::expression::ArrayLiteralExpr;

use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_array_literal(
        &mut self,
        expr: &ArrayLiteralExpr,
        source: &str,
    ) -> Option<SemanticType> {
        if expr.elements.is_empty() {
            return Some(SemanticType::Array(Box::new(SemanticType::Unknown)));
        }

        let first_type = self.check_expr(&expr.elements[0], source)?;

        for (i, element) in expr.elements.iter().enumerate().skip(1) {
            let elem_type = self.check_expr(element, source)?;
            if elem_type != SemanticType::Unknown
                && first_type != SemanticType::Unknown
                && !self.types_compatible(first_type.clone(), elem_type.clone())
            {
                self.analyzer.push_type_error(
                    expr.span,
                    source,
                    format!(
                        "Array element #{} has type {}, but element #1 has type {}.",
                        i + 1,
                        elem_type.display_name_with_table(&self.analyzer.type_table),
                        first_type.display_name_with_table(&self.analyzer.type_table)
                    ),
                );
                return None;
            }
        }

        Some(SemanticType::Array(Box::new(first_type)))
    }
}
