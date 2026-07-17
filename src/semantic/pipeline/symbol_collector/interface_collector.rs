//! Registro de interfaces (`protocol`) y su cadena de herencia (`extends`).
//! Misma estrategia de dos tiempos que los tipos: primero todos los nombres,
//! después los padres — así una interfaz puede extender otra declarada más
//! abajo en el fuente.

use crate::parser::expression::InterfaceDecl;
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::semantic::helper::{SemanticType, StructTypeInfo};

use super::SymbolCollector;

impl SymbolCollector {
    pub(in crate::semantic) fn collect_interfaces(
        analyzer: &mut SemanticAnalyzer,
        interface_decls: &[InterfaceDecl],
        source: &str,
    ) {
        Self::register_interface_names(analyzer, interface_decls, source);
        Self::resolve_interface_parents(analyzer, interface_decls, source);
    }

    fn register_interface_names(
        analyzer: &mut SemanticAnalyzer,
        interface_decls: &[InterfaceDecl],
        source: &str,
    ) {
        for interface_decl in interface_decls {
            if SemanticType::from_annotation_name(&interface_decl.name).is_some() {
                analyzer.push_semantic_error(
                    interface_decl.name_span,
                    source,
                    format!(
                        "Interface '{}' cannot be declared because the name is reserved.",
                        interface_decl.name
                    ),
                );
                continue;
            }

            if analyzer.type_symbols.contains_key(&interface_decl.name) {
                analyzer.push_semantic_error(
                    interface_decl.name_span,
                    source,
                    format!("Interface '{}' redeclared.", interface_decl.name),
                );
                continue;
            }

            let type_id = analyzer.type_table.register_type(StructTypeInfo {
                name: interface_decl.name.clone(),
                constructor_params: Vec::new(),
                fields: Vec::new(),
                methods: Vec::new(),
                parent: None,
                is_interface: true,
            });
            analyzer
                .type_symbols
                .insert(interface_decl.name.clone(), type_id);
        }
    }

    /// Un padre de interfaz debe ser otra interfaz, distinta de sí misma y
    /// sin formar ciclos de extensión.
    fn resolve_interface_parents(
        analyzer: &mut SemanticAnalyzer,
        interface_decls: &[InterfaceDecl],
        source: &str,
    ) {
        for interface_decl in interface_decls {
            let Some(interface_id) = analyzer.type_symbols.get(&interface_decl.name).copied()
            else {
                continue;
            };

            let parent_id = if let Some(parent_name) = &interface_decl.parent_name {
                match analyzer.type_symbols.get(parent_name).copied() {
                    Some(parent) => {
                        let is_interface = analyzer
                            .type_table
                            .get_struct(parent)
                            .is_some_and(|info| info.is_interface);
                        if !is_interface {
                            if let Some(parent_span) = interface_decl.parent_span {
                                analyzer.push_semantic_error(
                                    parent_span,
                                    source,
                                    format!(
                                        "Interface '{}' cannot extend type '{}' (only interfaces can be extended).",
                                        interface_decl.name, parent_name
                                    ),
                                );
                            }
                            None
                        } else if parent == interface_id {
                            if let Some(parent_span) = interface_decl.parent_span {
                                analyzer.push_semantic_error(
                                    parent_span,
                                    source,
                                    format!(
                                        "Circular extension detected for interface '{}'.",
                                        interface_decl.name
                                    ),
                                );
                            }
                            None
                        } else if Self::is_circular_inheritance(analyzer, parent, interface_id) {
                            if let Some(parent_span) = interface_decl.parent_span {
                                analyzer.push_semantic_error(
                                    parent_span,
                                    source,
                                    format!(
                                        "Circular extension detected for interface '{}'.",
                                        interface_decl.name
                                    ),
                                );
                            }
                            None
                        } else {
                            Some(parent)
                        }
                    }
                    None => {
                        if let Some(parent_span) = interface_decl.parent_span {
                            analyzer.push_semantic_error(
                                parent_span,
                                source,
                                format!("Parent interface '{}' not found.", parent_name),
                            );
                        }
                        None
                    }
                }
            } else {
                None
            };

            if let Some(parent_id) = parent_id
                && let Some(info) = analyzer.type_table.get_struct_mut(interface_id)
            {
                info.parent = Some(parent_id);
            }
        }
    }
}
