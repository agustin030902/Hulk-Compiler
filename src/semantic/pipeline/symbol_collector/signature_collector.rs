//! Recolección de firmas: funciones globales, métodos de tipo y métodos de
//! interfaz. Las tres pasadas comparten el mismo núcleo — resolver anotaciones
//! a tipos semánticos y sus `TypeId` ([`SignatureParts`]) y registrar el
//! método en las tres tablas ([`SymbolCollector::register_method_symbol`]) —
//! y solo difieren en sus validaciones propias (redeclaración, firma del
//! override, anotaciones obligatorias en interfaces).

use crate::parser::expression::{FunctionDecl, FunctionParam, InterfaceDecl, TypeAnnotation, TypeDecl};
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::semantic::helper::{FunctionSignature, FunctionSymbol, SemanticType, TypeId};
use crate::semantic::pipeline::{InterfaceChecker, TypeResolver};

use super::SymbolCollector;

/// Resultado de resolver las anotaciones de una firma: nombres y tipos de
/// parámetros (en ambas representaciones) y tipo de retorno.
pub(super) struct SignatureParts {
    pub(super) param_names: Vec<String>,
    pub(super) param_types: Vec<SemanticType>,
    pub(super) param_type_ids: Vec<TypeId>,
    pub(super) return_type: SemanticType,
    pub(super) return_type_id: TypeId,
}

impl SymbolCollector {
    /// Núcleo compartido: resuelve las anotaciones de parámetros y retorno
    /// (ausentes → `Unknown`, que la inferencia completará después).
    pub(super) fn resolve_signature_parts(
        analyzer: &mut SemanticAnalyzer,
        params: &[FunctionParam],
        return_annotation: Option<&TypeAnnotation>,
        source: &str,
    ) -> SignatureParts {
        let param_names: Vec<String> =
            params.iter().map(|param| param.name.clone()).collect();

        let param_types = params
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

        let return_type = return_annotation
            .and_then(|annotation| {
                TypeResolver::resolve_annotation_type(analyzer, annotation, source)
            })
            .unwrap_or(SemanticType::Unknown);

        let param_type_ids = param_types
            .iter()
            .copied()
            .map(|semantic_type| TypeResolver::semantic_type_to_type_id(analyzer, semantic_type))
            .collect::<Vec<_>>();
        let return_type_id = TypeResolver::semantic_type_to_type_id(analyzer, return_type);

        SignatureParts {
            param_names,
            param_types,
            param_type_ids,
            return_type,
            return_type_id,
        }
    }

    /// Registra un método en las tres tablas: entrada `Function` en el
    /// `TypeTable`, símbolo, firma, y el listado de métodos del receptor.
    pub(super) fn register_method_symbol(
        analyzer: &mut SemanticAnalyzer,
        receiver_type_id: TypeId,
        method_name: &str,
        parts: SignatureParts,
    ) {
        let key = Self::method_symbol_key(receiver_type_id, method_name);
        let method_type_id = analyzer.type_table.register_method(
            receiver_type_id,
            parts.param_type_ids,
            parts.return_type_id,
        );

        analyzer.function_symbols.insert(
            key.clone(),
            FunctionSymbol::new_method(
                method_name.to_string(),
                method_type_id,
                receiver_type_id,
            ),
        );
        analyzer.functions.insert(
            key,
            FunctionSignature {
                type_id: method_type_id.0,
                param_names: parts.param_names,
                param_types: parts.param_types,
                return_type: parts.return_type,
            },
        );

        if let Some(info) = analyzer.type_table.get_struct_mut(receiver_type_id) {
            info.methods.push((method_name.to_string(), method_type_id));
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

            let parts = Self::resolve_signature_parts(
                analyzer,
                &function.params,
                function.return_type_annotation.as_ref(),
                source,
            );

            let function_type_id = analyzer
                .type_table
                .register_plain_function(parts.param_type_ids, parts.return_type_id);

            analyzer.function_symbols.insert(
                function.name.clone(),
                FunctionSymbol::new_function(function.name.clone(), function_type_id),
            );
            analyzer.functions.insert(
                function.name.clone(),
                FunctionSignature {
                    type_id: function_type_id.0,
                    param_names: parts.param_names,
                    param_types: parts.param_types,
                    return_type: parts.return_type,
                },
            );
        }
    }

    pub(in crate::semantic) fn collect_methods(
        analyzer: &mut SemanticAnalyzer,
        type_decls: &[TypeDecl],
        source: &str,
    ) {
        for type_decl in type_decls {
            let Some(receiver_type_id) = analyzer.type_symbols.get(&type_decl.name).copied()
            else {
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

                let parts = Self::resolve_signature_parts(
                    analyzer,
                    &method.params,
                    method.return_type_annotation.as_ref(),
                    source,
                );

                // Un override debe conservar exactamente la firma del padre.
                if let Some(parent_signature) =
                    Self::find_method_in_parent(analyzer, receiver_type_id, &method.name)
                {
                    if parent_signature.param_types != parts.param_types
                        || parent_signature.return_type != parts.return_type
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

                Self::register_method_symbol(analyzer, receiver_type_id, &method.name, parts);
            }
        }
    }

    pub(in crate::semantic) fn collect_interface_methods(
        analyzer: &mut SemanticAnalyzer,
        interface_decls: &[InterfaceDecl],
        source: &str,
    ) {
        for interface_decl in interface_decls {
            let Some(receiver_type_id) =
                analyzer.type_symbols.get(&interface_decl.name).copied()
            else {
                continue;
            };

            for method in &interface_decl.methods {
                let key = Self::method_symbol_key(receiver_type_id, &method.name);
                if analyzer.function_symbols.contains_key(&key) {
                    analyzer.push_semantic_error(
                        method.name_span,
                        source,
                        format!(
                            "Method '{}' redeclared in interface '{}'.",
                            method.name, interface_decl.name
                        ),
                    );
                    continue;
                }

                // Las interfaces son contratos: exigen anotaciones explícitas.
                for param in &method.params {
                    if param.type_annotation.is_none() {
                        analyzer.push_semantic_error(
                            param.span,
                            source,
                            format!(
                                "Parameter '{}' in interface method '{}' must have an explicit type annotation.",
                                param.name, method.name
                            ),
                        );
                    }
                }

                let parts = Self::resolve_signature_parts(
                    analyzer,
                    &method.params,
                    Some(&method.return_type_annotation),
                    source,
                );

                if parts.return_type == SemanticType::Unknown {
                    analyzer.push_semantic_error(
                        method.return_type_annotation.span,
                        source,
                        format!(
                            "Interface method '{}' must declare a fully resolvable return type.",
                            method.name
                        ),
                    );
                }

                Self::register_method_symbol(analyzer, receiver_type_id, &method.name, parts);
            }
        }

        InterfaceChecker::check_interface_variance(analyzer, interface_decls, source);
    }
}
