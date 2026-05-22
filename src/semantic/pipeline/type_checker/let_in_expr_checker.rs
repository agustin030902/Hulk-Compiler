use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_let_in_expr(
        &mut self,
        let_in: &LetInExpr,
        source: &str,
    ) -> Option<SemanticType> {
        self.analyzer.push_scope();

        for binding in &let_in.bindings {
            if self.analyzer.is_declared_in_current_scope(&binding.name) {
                self.analyzer.push_semantic_error(
                    binding.span,
                    source,
                    format!("Variable '{}' redeclared in let-in binding.", binding.name),
                );
                continue;
            }

            let binding_type = if let Some(annotation) = &binding.type_annotation {
                if let Some(annotation_type) =
                    TypeResolver::resolve_annotation_type(self.analyzer, annotation, source)
                {
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
            self.analyzer
                .bind_current_scope(binding.name.clone(), binding_type);
        }

        let body_type = self.check_expr(&let_in.body, source);

        self.analyzer.pop_scope();

        body_type
    }
}
