use crate::parser::expression::LetInExpr;

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_let_in_expr(
        &mut self,
        let_in: &LetInExpr,
        source: &str,
    ) -> Option<SemanticType> {
        self.push_scope();

        for binding in &let_in.bindings {
            if self.is_declared_in_current_scope(&binding.name) {
                self.push_semantic_error(
                    binding.span,
                    source,
                    format!("Variable '{}' redeclared in let-in binding.", binding.name),
                );
                continue;
            }

            let binding_type = if let Some(annotation) = &binding.type_annotation {
                if let Some(annotation_type) = self.resolve_annotation_type(annotation, source) {
                    self.check_annotated_initializer(
                        &binding.name,
                        &binding.value,
                        annotation_type,
                        annotation.span,
                        source,
                    )
                } else {
                    self.check_expr(&binding.value, source)
                        .unwrap_or(SemanticType::Unknown)
                }
            } else {
                self.check_expr(&binding.value, source)
                    .unwrap_or(SemanticType::Unknown)
            };
            self.bind_current_scope(binding.name.clone(), binding_type);
        }

        let body_type = self.check_expr(&let_in.body, source);

        self.pop_scope();

        body_type
    }
}
