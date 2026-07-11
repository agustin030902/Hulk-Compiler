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
        let resolved_name = if annotation.is_splat {
            format!("Iterable_{}", annotation.name)
        } else {
            annotation.name.clone()
        };

        // Anotaciones de arreglo: `Number[]`, `Number[][]`. Se interna un tipo
        // de arreglo por cada dimensión, de adentro hacia afuera.
        if resolved_name.ends_with("[]") {
            if let Some(array_type) = Self::resolve_array_annotation(analyzer, &resolved_name) {
                return Some(array_type);
            }
        }

        // Anotaciones de tipo función: `(Number)->Number` (codificadas como
        // texto canónico por el parser).
        if resolved_name.starts_with('(') {
            if let Some(function_type) =
                Self::resolve_function_annotation(analyzer, &resolved_name)
            {
                return Some(function_type);
            }
            analyzer.push_semantic_error(
                annotation.span,
                source,
                format!("Invalid function type annotation '{}'.", annotation.name),
            );
            return None;
        }

        let Some(annotation_type) = Self::resolve_named_type(analyzer, &resolved_name) else {
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

    /// Resuelve un nombre con sufijos `[]` a su tipo de arreglo internado.
    pub(in crate::semantic) fn resolve_array_annotation(
        analyzer: &mut SemanticAnalyzer,
        name: &str,
    ) -> Option<SemanticType> {
        let mut base = name;
        let mut dims = 0usize;
        while let Some(stripped) = base.strip_suffix("[]") {
            base = stripped;
            dims += 1;
        }
        if dims == 0 {
            return None;
        }

        let base_type = Self::resolve_named_type(analyzer, base)?;
        let mut current = Self::semantic_type_to_type_id(analyzer, base_type);
        for _ in 0..dims {
            current = analyzer.type_table.array_of(current);
        }
        Some(SemanticType::Array(current.0))
    }

    /// Resuelve un nombre canónico de tipo función `(A,B)->C` (con posible
    /// anidamiento) al tipo función internado.
    pub(in crate::semantic) fn resolve_function_annotation(
        analyzer: &mut SemanticAnalyzer,
        name: &str,
    ) -> Option<SemanticType> {
        let (param_names, ret_name) = Self::split_function_type_name(name)?;

        let mut param_ids = Vec::with_capacity(param_names.len());
        for param_name in &param_names {
            let param_type = Self::resolve_composite_name(analyzer, param_name)?;
            param_ids.push(Self::semantic_type_to_type_id(analyzer, param_type));
        }
        let ret_type = Self::resolve_composite_name(analyzer, &ret_name)?;
        let ret_id = Self::semantic_type_to_type_id(analyzer, ret_type);

        let function_id = analyzer.type_table.function_type_of(param_ids, ret_id);
        Some(SemanticType::Function(function_id.0))
    }

    /// Nombre compuesto: tipo función, arreglo o nombre simple.
    fn resolve_composite_name(
        analyzer: &mut SemanticAnalyzer,
        name: &str,
    ) -> Option<SemanticType> {
        if name.starts_with('(') {
            return Self::resolve_function_annotation(analyzer, name);
        }
        if name.ends_with("[]") {
            return Self::resolve_array_annotation(analyzer, name);
        }
        Self::resolve_named_type(analyzer, name)
    }

    /// Separa `(A,B)->C` en (["A","B"], "C") respetando paréntesis anidados.
    fn split_function_type_name(name: &str) -> Option<(Vec<String>, String)> {
        let inner_start = name.find('(')? + 1;
        let mut depth = 1usize;
        let mut inner_end = None;
        for (offset, ch) in name[inner_start..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        inner_end = Some(inner_start + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        let inner_end = inner_end?;
        let params_text = &name[inner_start..inner_end];
        let ret_name = name[inner_end + 1..].strip_prefix("->")?.to_string();

        let mut params = Vec::new();
        if !params_text.is_empty() {
            let mut depth = 0usize;
            let mut start = 0usize;
            for (offset, ch) in params_text.char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => depth = depth.saturating_sub(1),
                    ',' if depth == 0 => {
                        params.push(params_text[start..offset].to_string());
                        start = offset + 1;
                    }
                    _ => {}
                }
            }
            params.push(params_text[start..].to_string());
        }

        Some((params, ret_name))
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
            SemanticType::Function(type_id)
            | SemanticType::Struct(type_id)
            | SemanticType::Array(type_id) => TypeId(type_id),
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
            TypeInfo::Array { .. } => SemanticType::Array(type_id.0),
            _ => SemanticType::Unknown,
        }
    }
}
