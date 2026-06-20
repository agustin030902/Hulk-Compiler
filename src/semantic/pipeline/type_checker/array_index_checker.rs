use crate::parser::expression::ArrayIndexExpr;

use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_array_index(
        &mut self,
        expr: &ArrayIndexExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let object_type = self.check_expr(&expr.object, source)?;
        let index_type = self.check_expr(&expr.index, source)?;

        if index_type != SemanticType::Number {
            self.analyzer.push_type_error(
                expr.span,
                source,
                format!(
                    "Array index must be Number, but got {}.",
                    index_type.display_name_with_table(&self.analyzer.type_table)
                ),
            );
            return None;
        }

        match &object_type {
            SemanticType::Array(element_type) => Some(*element_type.clone()),
            _ => {
                self.analyzer.push_type_error(
                    expr.span,
                    source,
                    format!(
                        "Cannot index into non-array type {}.",
                        object_type.display_name_with_table(&self.analyzer.type_table)
                    ),
                );
                None
            }
        }
    }
}
