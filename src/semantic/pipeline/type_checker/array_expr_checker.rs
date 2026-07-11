use crate::parser::expression::{ArrayLiteralExpr, IndexExpr, NewArrayExpr};

use super::super::super::{
    analyzer::SemanticAnalyzer,
    helper::{SemanticType, TypeId},
};
use super::super::TypeResolver;
use super::TypeChecker;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_array_literal(
        &mut self,
        literal: &ArrayLiteralExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let mut elem_type = SemanticType::Unknown;

        for element in &literal.elements {
            let current = self
                .check_expr(element, source)
                .unwrap_or(SemanticType::Unknown);

            if elem_type == SemanticType::Unknown {
                elem_type = current;
                continue;
            }
            if current == SemanticType::Unknown || current == elem_type {
                continue;
            }
            if self.types_compatible(elem_type, current) {
                continue;
            }
            self.analyzer.push_type_error(
                element.span(),
                source,
                format!(
                    "Array literal elements must share one type: expected {}, but got {}.",
                    elem_type.display_name_with_table(&self.analyzer.type_table),
                    current.display_name_with_table(&self.analyzer.type_table)
                ),
            );
            return None;
        }

        let elem_id = TypeResolver::semantic_type_to_type_id(self.analyzer, elem_type);
        let array_id = self.analyzer.type_table.array_of(elem_id);
        Some(SemanticType::Array(array_id.0))
    }

    pub(super) fn check_new_array(
        &mut self,
        new_array: &NewArrayExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let elem_type = if new_array.elem_type_name.ends_with("[]") {
            TypeResolver::resolve_array_annotation(self.analyzer, &new_array.elem_type_name)
        } else {
            TypeResolver::resolve_named_type(self.analyzer, &new_array.elem_type_name)
        };
        let Some(elem_type) = elem_type else {
            self.analyzer.push_semantic_error(
                new_array.elem_type_span,
                source,
                format!(
                    "Unknown element type '{}' in array construction.",
                    new_array.elem_type_name
                ),
            );
            return None;
        };

        let size_type = self
            .check_expr(&new_array.size, source)
            .unwrap_or(SemanticType::Unknown);
        if size_type != SemanticType::Unknown && size_type != SemanticType::Number {
            self.analyzer.push_type_error(
                new_array.size.span(),
                source,
                format!(
                    "Array size must be a Number, but got {}.",
                    size_type.display_name_with_table(&self.analyzer.type_table)
                ),
            );
        }

        if let Some(init) = &new_array.init {
            // El inicializador `{ i -> expr }` liga el índice como Number.
            self.analyzer.push_scope();
            self.analyzer
                .bind_current_scope(init.var_name.clone(), SemanticType::Number);
            let body_type = self
                .check_expr(&init.body, source)
                .unwrap_or(SemanticType::Unknown);
            self.analyzer.pop_scope();

            if body_type != SemanticType::Unknown
                && !self.types_compatible(elem_type, body_type)
            {
                self.analyzer.push_type_error(
                    init.body.span(),
                    source,
                    format!(
                        "Array initializer must produce {}, but got {}.",
                        elem_type.display_name_with_table(&self.analyzer.type_table),
                        body_type.display_name_with_table(&self.analyzer.type_table)
                    ),
                );
            }
        }

        let elem_id = TypeResolver::semantic_type_to_type_id(self.analyzer, elem_type);
        let array_id = self.analyzer.type_table.array_of(elem_id);
        Some(SemanticType::Array(array_id.0))
    }

    pub(super) fn check_index_expr(
        &mut self,
        index_expr: &IndexExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let object_type = self.check_expr(&index_expr.object, source)?;

        let index_type = self
            .check_expr(&index_expr.index, source)
            .unwrap_or(SemanticType::Unknown);
        if index_type != SemanticType::Unknown && index_type != SemanticType::Number {
            self.analyzer.push_type_error(
                index_expr.index.span(),
                source,
                format!(
                    "Array index must be a Number, but got {}.",
                    index_type.display_name_with_table(&self.analyzer.type_table)
                ),
            );
        }

        Self::array_element_type(self.analyzer, object_type).or_else(|| {
            if object_type != SemanticType::Unknown {
                self.analyzer.push_type_error(
                    index_expr.object.span(),
                    source,
                    format!(
                        "Indexing requires an array, but got {}.",
                        object_type.display_name_with_table(&self.analyzer.type_table)
                    ),
                );
            }
            Some(SemanticType::Unknown)
        })
    }

    pub(in crate::semantic) fn array_element_type(
        analyzer: &SemanticAnalyzer,
        array_type: SemanticType,
    ) -> Option<SemanticType> {
        let SemanticType::Array(array_raw) = array_type else {
            return None;
        };
        analyzer
            .type_table
            .get_array_elem(TypeId(array_raw))
            .map(|elem_id| TypeResolver::type_id_to_semantic_type(analyzer, elem_id))
    }
}
