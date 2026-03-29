use crate::parser::expression::Statement;

use super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_statement(
        &mut self,
        statement: &Statement,
        source: &str,
    ) -> Option<SemanticType> {
        match statement {
            Statement::Let {
                name,
                name_span,
                value,
                ..
            } => {
                if self.is_declared_in_current_scope(name) {
                    self.push_semantic_error(
                        *name_span,
                        source,
                        format!(
                            "Variable '{}' redeclared. A variable can only be declared once.",
                            name
                        ),
                    );
                    return None;
                }

                let value_type = self
                    .check_expr(value, source)
                    .unwrap_or(SemanticType::Unknown);
                self.current_scope_mut().insert(name.clone(), value_type);
                Some(value_type)
            }
            Statement::Print { value, span } => self.check_print_argument(value, *span, source),
            Statement::Expr { value, .. } => self.check_expr(value, source),
            Statement::Assign {
                name,
                name_span,
                value,
                ..
            } => {
                let Some(scope_index) = self.find_scope_index(name) else {
                    self.push_semantic_error(
                        *name_span,
                        source,
                        format!(
                            "Variable '{}' is assigned before declaration. Declare it with 'let' first.",
                            name
                        ),
                    );
                    return None;
                };

                let value_type = self.check_expr(value, source)?;
                self.assign_in_scope(scope_index, name.clone(), value_type);
                Some(value_type)
            }
        }
    }
}
