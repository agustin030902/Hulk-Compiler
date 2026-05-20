use crate::parser::expression::{FunctionDecl, TypeDecl};

use super::super::{
    analyzer::SemanticAnalyzer,
    helper::{FunctionSignature, FunctionSymbol, SemanticType, StructTypeInfo, TypeId},
};
use super::TypeResolver;

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
            });
            analyzer
                .type_symbols
                .insert(type_decl.name.clone(), type_id);
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
}
