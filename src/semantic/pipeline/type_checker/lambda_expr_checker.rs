use crate::parser::expression::LambdaExpr;

use super::super::super::helper::{SemanticType, TypeId};
use super::super::TypeResolver;
use super::TypeChecker;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_lambda(
        &mut self,
        lambda: &LambdaExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let mut param_types = Vec::with_capacity(lambda.params.len());
        for param in &lambda.params {
            let param_type = match &param.type_annotation {
                Some(annotation) => {
                    TypeResolver::resolve_annotation_type(self.analyzer, annotation, source)
                        .unwrap_or(SemanticType::Unknown)
                }
                None => {
                    self.analyzer.push_semantic_error(
                        param.span,
                        source,
                        format!(
                            "Lambda parameter '{}' requires a type annotation.",
                            param.name
                        ),
                    );
                    SemanticType::Unknown
                }
            };
            param_types.push(param_type);
        }

        let annotated_return = lambda
            .return_type_annotation
            .as_ref()
            .and_then(|annotation| {
                TypeResolver::resolve_annotation_type(self.analyzer, annotation, source)
            });

        // El cuerpo ve los parámetros y, léxicamente, el entorno que lo rodea
        // (la captura por valor es un detalle de codegen).
        self.analyzer.push_scope();
        for (param, param_type) in lambda.params.iter().zip(param_types.iter().copied()) {
            self.analyzer
                .bind_current_scope(param.name.clone(), param_type);
        }
        let body_type = self
            .check_expr(&lambda.body, source)
            .unwrap_or(SemanticType::Unknown);
        self.analyzer.pop_scope();

        let return_type = match annotated_return {
            Some(expected) => {
                if body_type != SemanticType::Unknown
                    && !self.types_compatible(expected, body_type)
                {
                    self.analyzer.push_type_error(
                        lambda.body.span(),
                        source,
                        format!(
                            "Lambda body must produce {}, but got {}.",
                            expected.display_name_with_table(&self.analyzer.type_table),
                            body_type.display_name_with_table(&self.analyzer.type_table)
                        ),
                    );
                }
                expected
            }
            None => body_type,
        };

        let param_ids = param_types
            .iter()
            .map(|param_type| TypeResolver::semantic_type_to_type_id(self.analyzer, *param_type))
            .collect::<Vec<_>>();
        let return_id = TypeResolver::semantic_type_to_type_id(self.analyzer, return_type);
        let function_id = self.analyzer.type_table.function_type_of(param_ids, return_id);
        Some(SemanticType::Function(function_id.0))
    }

    /// Igualdad estructural entre tipos función (params contravariantes no
    /// aplican aquí: las firmas deben coincidir exactamente).
    pub(super) fn function_types_equal(&self, left: TypeId, right: TypeId) -> bool {
        if left == right {
            return true;
        }
        let (Some(a), Some(b)) = (
            self.analyzer.type_table.get_function(left),
            self.analyzer.type_table.get_function(right),
        ) else {
            return false;
        };
        if a.params.len() != b.params.len() {
            return false;
        }
        a.params
            .iter()
            .zip(b.params.iter())
            .all(|(x, y)| self.type_ids_structurally_equal(*x, *y))
            && self.type_ids_structurally_equal(a.return_type, b.return_type)
    }

    fn type_ids_structurally_equal(&self, left: TypeId, right: TypeId) -> bool {
        if left == right {
            return true;
        }
        let table = &self.analyzer.type_table;
        if table.get_function(left).is_some() && table.get_function(right).is_some() {
            return self.function_types_equal(left, right);
        }
        match (table.get_array_elem(left), table.get_array_elem(right)) {
            (Some(a), Some(b)) => self.type_ids_structurally_equal(a, b),
            _ => false,
        }
    }
}
