use crate::parser::expression::DestructiveAssignExpr;

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_destructive_assign(
        &mut self,
        assign: &DestructiveAssignExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let Some((scope_index, existing)) = self.lookup_with_scope_index(&assign.name) else {
            self.push_semantic_error(
                assign.name_span,
                source,
                format!(
                    "Variable '{}' is assigned before declaration. Declare it with 'let' first.",
                    assign.name
                ),
            );
            return None;
        };

        let value_type = self.check_expr(&assign.value, source)?;

        if existing != SemanticType::Unknown && existing != value_type {
            self.push_type_error(
                assign.span,
                source,
                format!(
                    "Destructive assignment ':=' requires the same type. Variable '{}' is {:?} but expression is {:?}.",
                    assign.name, existing, value_type
                ),
            );
            return None;
        }

        self.assign_in_scope(scope_index, assign.name.clone(), value_type);
        Some(value_type)
    }
}
