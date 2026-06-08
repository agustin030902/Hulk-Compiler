use crate::parser::expression::{FunctionDecl, ProtocolDecl, TypeDecl};

use super::super::{
    analyzer::SemanticAnalyzer,
    helper::{FunctionSignature, FunctionSymbol, SemanticType, StructTypeInfo, TypeId},
};
use super::{ProtocolChecker, TypeResolver};

pub(in crate::semantic) struct SymbolCollector;

impl SymbolCollector {
    pub(in crate::semantic) fn method_symbol_key(receiver: TypeId, method_name: &str) -> String {
        format!("type#{}::{}", receiver.0, method_name)
    }

    pub(in crate::semantic) fn collect_types(
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
                is_protocol: false,
            });
            analyzer
                .type_symbols
                .insert(type_decl.name.clone(), type_id);
        }

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
                            format!("Circular inheritance detected for type '{}'.", type_decl.name),
                        );
                    }
                    continue;
                }
            }

            if let Some(struct_info) = analyzer.type_table.get_struct_mut(type_id) {
                struct_info.parent = parent_type_id;
            }
        }

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

    pub(in crate::semantic) fn collect_protocols(
        analyzer: &mut SemanticAnalyzer,
        protocol_decls: &[ProtocolDecl],
        source: &str,
    ) {
        for protocol_decl in protocol_decls {
            if SemanticType::from_annotation_name(&protocol_decl.name).is_some() {
                analyzer.push_semantic_error(
                    protocol_decl.name_span,
                    source,
                    format!(
                        "Protocol '{}' cannot be declared because the name is reserved.",
                        protocol_decl.name
                    ),
                );
                continue;
            }

            if analyzer.type_symbols.contains_key(&protocol_decl.name) {
                analyzer.push_semantic_error(
                    protocol_decl.name_span,
                    source,
                    format!("Protocol '{}' redeclared.", protocol_decl.name),
                );
                continue;
            }

            let type_id = analyzer.type_table.register_type(StructTypeInfo {
                name: protocol_decl.name.clone(),
                constructor_params: Vec::new(),
                fields: Vec::new(),
                methods: Vec::new(),
                parent: None,
                is_protocol: true,
            });
            analyzer
                .type_symbols
                .insert(protocol_decl.name.clone(), type_id);
        }

        for protocol_decl in protocol_decls {
            let Some(protocol_id) = analyzer.type_symbols.get(&protocol_decl.name).copied()
            else {
                continue;
            };

            let parent_id = if let Some(parent_name) = &protocol_decl.parent_name {
                match analyzer.type_symbols.get(parent_name).copied() {
                    Some(parent) => {
                        let is_protocol = analyzer
                            .type_table
                            .get_struct(parent)
                            .is_some_and(|info| info.is_protocol);
                        if !is_protocol {
                            if let Some(parent_span) = protocol_decl.parent_span {
                                analyzer.push_semantic_error(
                                    parent_span,
                                    source,
                                    format!(
                                        "Protocol '{}' cannot extend type '{}' (only protocols can be extended).",
                                        protocol_decl.name, parent_name
                                    ),
                                );
                            }
                            None
                        } else if parent == protocol_id {
                            if let Some(parent_span) = protocol_decl.parent_span {
                                analyzer.push_semantic_error(
                                    parent_span,
                                    source,
                                    format!(
                                        "Circular extension detected for protocol '{}'.",
                                        protocol_decl.name
                                    ),
                                );
                            }
                            None
                        } else if Self::is_circular_inheritance(analyzer, parent, protocol_id) {
                            if let Some(parent_span) = protocol_decl.parent_span {
                                analyzer.push_semantic_error(
                                    parent_span,
                                    source,
                                    format!(
                                        "Circular extension detected for protocol '{}'.",
                                        protocol_decl.name
                                    ),
                                );
                            }
                            None
                        } else {
                            Some(parent)
                        }
                    }
                    None => {
                        if let Some(parent_span) = protocol_decl.parent_span {
                            analyzer.push_semantic_error(
                                parent_span,
                                source,
                                format!("Parent protocol '{}' not found.", parent_name),
                            );
                        }
                        None
                    }
                }
            } else {
                None
            };

            if let Some(parent_id) = parent_id
                && let Some(info) = analyzer.type_table.get_struct_mut(protocol_id)
            {
                info.parent = Some(parent_id);
            }
        }
    }

    pub(in crate::semantic) fn is_protocol(
        analyzer: &SemanticAnalyzer,
        type_id: TypeId,
    ) -> bool {
        analyzer
            .type_table
            .get_struct(type_id)
            .is_some_and(|info| info.is_protocol)
    }

    fn is_circular_inheritance(
        analyzer: &SemanticAnalyzer,
        parent_id: TypeId,
        child_id: TypeId,
    ) -> bool {
        let mut cursor = Some(parent_id);
        while let Some(current) = cursor {
            if current == child_id {
                return true;
            }
            cursor = analyzer
                .type_table
                .get_struct(current)
                .and_then(|info| info.parent);
        }
        false
    }

    pub(in crate::semantic) fn collect_functions(
        analyzer: &mut SemanticAnalyzer,
        functions: &[FunctionDecl],
        source: &str,
    ) {
        for function in functions {
            if analyzer.function_symbols.contains_key(&function.name) {
                analyzer.push_semantic_error(
                    function.name_span,
                    source,
                    format!("Function '{}' redeclared.", function.name),
                );
                continue;
            }

            let param_types = function
                .params
                .iter()
                .map(|param| {
                    param
                        .type_annotation
                        .as_ref()
                        .and_then(|annotation| {
                            TypeResolver::resolve_annotation_type(analyzer, annotation, source)
                        })
                        .unwrap_or(SemanticType::Unknown)
                })
                .collect::<Vec<_>>();

            let return_type = function
                .return_type_annotation
                .as_ref()
                .and_then(|annotation| {
                    TypeResolver::resolve_annotation_type(analyzer, annotation, source)
                })
                .unwrap_or(SemanticType::Unknown);

            let param_type_ids = param_types
                .iter()
                .copied()
                .map(|semantic_type| {
                    TypeResolver::semantic_type_to_type_id(analyzer, semantic_type)
                })
                .collect::<Vec<_>>();
            let return_type_id = TypeResolver::semantic_type_to_type_id(analyzer, return_type);
            let function_type_id = analyzer
                .type_table
                .register_plain_function(param_type_ids, return_type_id);

            let signature = FunctionSignature {
                type_id: function_type_id.0,
                param_types,
                return_type,
            };
            analyzer.function_symbols.insert(
                function.name.clone(),
                FunctionSymbol::new_function(function.name.clone(), function_type_id),
            );
            analyzer.functions.insert(function.name.clone(), signature);
        }
    }

    pub(in crate::semantic) fn collect_methods(
        analyzer: &mut SemanticAnalyzer,
        type_decls: &[TypeDecl],
        source: &str,
    ) {
        for type_decl in type_decls {
            let Some(receiver_type_id) = analyzer.type_symbols.get(&type_decl.name).copied() else {
                continue;
            };

            for method in &type_decl.methods {
                let key = Self::method_symbol_key(receiver_type_id, &method.name);
                if analyzer.function_symbols.contains_key(&key) {
                    analyzer.push_semantic_error(
                        method.name_span,
                        source,
                        format!(
                            "Method '{}' redeclared in type '{}'.",
                            method.name, type_decl.name
                        ),
                    );
                    continue;
                }

                let param_types = method
                    .params
                    .iter()
                    .map(|param| {
                        param
                            .type_annotation
                            .as_ref()
                            .and_then(|annotation| {
                                TypeResolver::resolve_annotation_type(analyzer, annotation, source)
                            })
                            .unwrap_or(SemanticType::Unknown)
                    })
                    .collect::<Vec<_>>();

                let return_type = method
                    .return_type_annotation
                    .as_ref()
                    .and_then(|annotation| {
                        TypeResolver::resolve_annotation_type(analyzer, annotation, source)
                    })
                    .unwrap_or(SemanticType::Unknown);

                let param_type_ids = param_types
                    .iter()
                    .copied()
                    .map(|semantic_type| {
                        TypeResolver::semantic_type_to_type_id(analyzer, semantic_type)
                    })
                    .collect::<Vec<_>>();
                let return_type_id = TypeResolver::semantic_type_to_type_id(analyzer, return_type);

                if let Some(parent_signature) =
                    Self::find_method_in_parent(analyzer, receiver_type_id, &method.name)
                {
                    if parent_signature.param_types != param_types
                        || parent_signature.return_type != return_type
                    {
                        analyzer.push_semantic_error(
                            method.name_span,
                            source,
                            format!(
                                "Method '{}' override in type '{}' has different signature than parent.",
                                method.name, type_decl.name
                            ),
                        );
                        continue;
                    }
                }

                let method_type_id = analyzer.type_table.register_method(
                    receiver_type_id,
                    param_type_ids,
                    return_type_id,
                );

                analyzer.function_symbols.insert(
                    key.clone(),
                    FunctionSymbol::new_method(
                        method.name.clone(),
                        method_type_id,
                        receiver_type_id,
                    ),
                );
                analyzer.functions.insert(
                    key.clone(),
                    FunctionSignature {
                        type_id: method_type_id.0,
                        param_types,
                        return_type,
                    },
                );

                if let Some(info) = analyzer.type_table.get_struct_mut(receiver_type_id) {
                    info.methods.push((method.name.clone(), method_type_id));
                }
            }
        }
    }

    pub(in crate::semantic) fn collect_protocol_methods(
        analyzer: &mut SemanticAnalyzer,
        protocol_decls: &[ProtocolDecl],
        source: &str,
    ) {
        for protocol_decl in protocol_decls {
            let Some(receiver_type_id) =
                analyzer.type_symbols.get(&protocol_decl.name).copied()
            else {
                continue;
            };

            for method in &protocol_decl.methods {
                let key = Self::method_symbol_key(receiver_type_id, &method.name);
                if analyzer.function_symbols.contains_key(&key) {
                    analyzer.push_semantic_error(
                        method.name_span,
                        source,
                        format!(
                            "Method '{}' redeclared in protocol '{}'.",
                            method.name, protocol_decl.name
                        ),
                    );
                    continue;
                }

                for param in &method.params {
                    if param.type_annotation.is_none() {
                        analyzer.push_semantic_error(
                            param.span,
                            source,
                            format!(
                                "Parameter '{}' in protocol method '{}' must have an explicit type annotation.",
                                param.name, method.name
                            ),
                        );
                    }
                }

                let param_types = method
                    .params
                    .iter()
                    .map(|param| {
                        param
                            .type_annotation
                            .as_ref()
                            .and_then(|annotation| {
                                TypeResolver::resolve_annotation_type(
                                    analyzer,
                                    annotation,
                                    source,
                                )
                            })
                            .unwrap_or(SemanticType::Unknown)
                    })
                    .collect::<Vec<_>>();

                let return_type = TypeResolver::resolve_annotation_type(
                    analyzer,
                    &method.return_type_annotation,
                    source,
                )
                .unwrap_or(SemanticType::Unknown);

                if return_type == SemanticType::Unknown {
                    analyzer.push_semantic_error(
                        method.return_type_annotation.span,
                        source,
                        format!(
                            "Protocol method '{}' must declare a fully resolvable return type.",
                            method.name
                        ),
                    );
                }

                let param_type_ids = param_types
                    .iter()
                    .copied()
                    .map(|semantic_type| {
                        TypeResolver::semantic_type_to_type_id(analyzer, semantic_type)
                    })
                    .collect::<Vec<_>>();
                let return_type_id =
                    TypeResolver::semantic_type_to_type_id(analyzer, return_type);

                let method_type_id = analyzer.type_table.register_method(
                    receiver_type_id,
                    param_type_ids,
                    return_type_id,
                );

                analyzer.function_symbols.insert(
                    key.clone(),
                    FunctionSymbol::new_method(
                        method.name.clone(),
                        method_type_id,
                        receiver_type_id,
                    ),
                );
                analyzer.functions.insert(
                    key.clone(),
                    FunctionSignature {
                        type_id: method_type_id.0,
                        param_types,
                        return_type,
                    },
                );

                if let Some(info) = analyzer.type_table.get_struct_mut(receiver_type_id) {
                    info.methods.push((method.name.clone(), method_type_id));
                }
            }
        }

        ProtocolChecker::check_protocol_variance(analyzer, protocol_decls, source);
    }

    fn find_method_in_parent(
        analyzer: &SemanticAnalyzer,
        type_id: TypeId,
        method_name: &str,
    ) -> Option<FunctionSignature> {
        let parent_id = analyzer.type_table.get_struct(type_id)?.parent?;
        let key = Self::method_symbol_key(parent_id, method_name);
        if let Some(signature) = analyzer.functions.get(&key) {
            return Some(signature.clone());
        }
        Self::find_method_in_parent(analyzer, parent_id, method_name)
    }
}
