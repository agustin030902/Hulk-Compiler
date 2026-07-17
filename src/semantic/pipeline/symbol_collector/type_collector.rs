//! Registro de tipos declarados con `type`, en tres sub-pasadas: primero los
//! nombres (para que las referencias cruzadas resuelvan), después los padres
//! (con detección de ciclos) y por último los parámetros de constructor (que
//! pueden anotar cualquier tipo ya registrado).

use crate::parser::expression::TypeDecl;
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::semantic::helper::{SemanticType, StructTypeInfo};
use crate::semantic::pipeline::TypeResolver;

use super::SymbolCollector;

impl SymbolCollector {
    pub(in crate::semantic) fn collect_types(
        analyzer: &mut SemanticAnalyzer,
        type_decls: &[TypeDecl],
        source: &str,
    ) {
        Self::register_type_names(analyzer, type_decls, source);
        Self::resolve_type_parents(analyzer, type_decls, source);
        Self::collect_constructor_params(analyzer, type_decls, source);
    }

    /// Sub-pasada 1: registra cada nombre de tipo con una entrada vacía,
    /// validando nombres reservados y redeclaraciones.
    fn register_type_names(
        analyzer: &mut SemanticAnalyzer,
        type_decls: &[TypeDecl],
        source: &str,
    ) {
        for type_decl in type_decls {
            if SemanticType::from_annotation_name(&type_decl.name).is_some() {
                analyzer.push_semantic_error(
                    type_decl.name_span,
                    source,
                    format!(
                        "Type '{}' cannot be declared because the name is reserved.",
                        type_decl.name
                    ),
                );
                continue;
            }

            if analyzer.type_symbols.contains_key(&type_decl.name) {
                analyzer.push_semantic_error(
                    type_decl.name_span,
                    source,
                    format!("Type '{}' redeclared.", type_decl.name),
                );
                continue;
            }

            let type_id = analyzer.type_table.register_type(StructTypeInfo {
                name: type_decl.name.clone(),
                constructor_params: Vec::new(),
                fields: Vec::new(),
                methods: Vec::new(),
                parent: None,
                is_interface: false,
            });
            analyzer
                .type_symbols
                .insert(type_decl.name.clone(), type_id);
        }
    }

    /// Sub-pasada 2: resuelve el padre de cada tipo (`Object` implícito si no
    /// hay `inherits`) rechazando auto-herencia y ciclos.
    fn resolve_type_parents(
        analyzer: &mut SemanticAnalyzer,
        type_decls: &[TypeDecl],
        source: &str,
    ) {
        for type_decl in type_decls {
            let Some(type_id) = analyzer.type_symbols.get(&type_decl.name).copied() else {
                continue;
            };

            let parent_type_id = if let Some(parent_name) = &type_decl.parent_name {
                match analyzer.type_symbols.get(parent_name).copied() {
                    Some(parent_id) => {
                        if parent_id == type_id {
                            if let Some(parent_span) = type_decl.parent_span {
                                analyzer.push_semantic_error(
                                    parent_span,
                                    source,
                                    format!(
                                        "Circular inheritance detected for type '{}'.",
                                        type_decl.name
                                    ),
                                );
                            }
                            None
                        } else {
                            Some(parent_id)
                        }
                    }
                    None => {
                        if let Some(parent_span) = type_decl.parent_span {
                            analyzer.push_semantic_error(
                                parent_span,
                                source,
                                format!("Parent type '{}' not found.", parent_name),
                            );
                        }
                        None
                    }
                }
            } else {
                Some(analyzer.type_table.object)
            };

            if let Some(parent_id) = parent_type_id {
                if Self::is_circular_inheritance(analyzer, parent_id, type_id) {
                    if let Some(parent_span) = type_decl.parent_span {
                        analyzer.push_semantic_error(
                            parent_span,
                            source,
                            format!(
                                "Circular inheritance detected for type '{}'.",
                                type_decl.name
                            ),
                        );
                    }
                    continue;
                }
            }

            if let Some(struct_info) = analyzer.type_table.get_struct_mut(type_id) {
                struct_info.parent = parent_type_id;
            }
        }
    }

    /// Sub-pasada 3: resuelve las anotaciones de los parámetros de constructor
    /// (ya con todos los tipos registrados).
    fn collect_constructor_params(
        analyzer: &mut SemanticAnalyzer,
        type_decls: &[TypeDecl],
        source: &str,
    ) {
        for type_decl in type_decls {
            let Some(type_id) = analyzer.type_symbols.get(&type_decl.name).copied() else {
                continue;
            };

            let mut constructor_params = Vec::with_capacity(type_decl.params.len());
            for param in &type_decl.params {
                let param_type = param
                    .type_annotation
                    .as_ref()
                    .and_then(|annotation| {
                        TypeResolver::resolve_annotation_type(analyzer, annotation, source)
                    })
                    .unwrap_or(SemanticType::Unknown);
                constructor_params.push((
                    param.name.clone(),
                    TypeResolver::semantic_type_to_type_id(analyzer, param_type),
                ));
            }

            if let Some(struct_info) = analyzer.type_table.get_struct_mut(type_id) {
                struct_info.constructor_params = constructor_params;
            }
        }
    }
}
