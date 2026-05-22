use crate::parser::expression::TypeAnnotation;

use super::super::{
    analyzer::SemanticAnalyzer,
    helper::{SemanticType, TypeId, TypeInfo},
};

pub(in crate::semantic) struct TypeResolver;

impl TypeResolver {
    pub(in crate::semantic) fn resolve_annotation_type(
        analyzer: &mut SemanticAnalyzer,
        annotation: &TypeAnnotation,
        source: &str,
    ) -> Option<SemanticType> {
        let Some(annotation_type) = Self::resolve_named_type(analyzer, &annotation.name) else {
            analyzer.push_semantic_error(
                annotation.span,
                source,
                format!(
                    "Unknown type annotation '{}'. Expected one of: {}.",
                    annotation.name,
                    Self::known_annotation_names(analyzer)
                ),
            );
            return None;
        };

        Some(annotation_type)
    }

    pub(in crate::semantic) fn resolve_named_type(
        analyzer: &SemanticAnalyzer,
        name: &str,
    ) -> Option<SemanticType> {
        if let Some(primitive) = SemanticType::from_annotation_name(name) {
            return Some(primitive);
        }

        analyzer
            .type_symbols
            .get(name)
            .copied()
            .map(|type_id| SemanticType::Struct(type_id.0))
    }

    pub(in crate::semantic) fn known_annotation_names(analyzer: &SemanticAnalyzer) -> String {
        let mut names = vec!["Number", "Boolean", "String", "Unit", "Null"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();

        let mut user_types = analyzer.type_symbols.keys().cloned().collect::<Vec<_>>();
        user_types.sort();
        names.extend(user_types);

        names.join(", ")
    }

    pub(in crate::semantic) fn semantic_type_to_type_id(
        analyzer: &SemanticAnalyzer,
        semantic_type: SemanticType,
    ) -> TypeId {
        match semantic_type {
            SemanticType::Number => analyzer.type_table.number,
            SemanticType::Boolean => analyzer.type_table.boolean,
            SemanticType::String => analyzer.type_table.string,
            SemanticType::Unit => analyzer.type_table.unit,
            SemanticType::Null => analyzer.type_table.null,
            SemanticType::Unknown => analyzer.type_table.unknown,
            SemanticType::Function(type_id) | SemanticType::Struct(type_id) => TypeId(type_id),
        }
    }

    pub(in crate::semantic) fn type_id_to_semantic_type(
        analyzer: &SemanticAnalyzer,
        type_id: TypeId,
    ) -> SemanticType {
        if type_id == analyzer.type_table.number {
            return SemanticType::Number;
        }
        if type_id == analyzer.type_table.boolean {
            return SemanticType::Boolean;
        }
        if type_id == analyzer.type_table.string {
            return SemanticType::String;
        }
        if type_id == analyzer.type_table.unit {
            return SemanticType::Unit;
        }
        if type_id == analyzer.type_table.null {
            return SemanticType::Null;
        }
        if type_id == analyzer.type_table.unknown {
            return SemanticType::Unknown;
        }

        match analyzer.type_table.get(type_id) {
            TypeInfo::Null => SemanticType::Null,
            TypeInfo::Function(_) => SemanticType::Function(type_id.0),
            TypeInfo::Type(_) => SemanticType::Struct(type_id.0),
            _ => SemanticType::Unknown,
        }
    }
}
