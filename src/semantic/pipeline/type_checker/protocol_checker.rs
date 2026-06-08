use crate::parser::expression::{ProtocolDecl, Span};

use super::super::super::{
    analyzer::SemanticAnalyzer,
    helper::{FunctionSignature, SemanticType, TypeId},
};
use super::super::{SymbolCollector, TypeResolver};

pub(in crate::semantic) struct InheritedProtocolMethod {
    pub name: String,
    pub type_id: TypeId,
    pub param_types: Vec<SemanticType>,
    pub return_type: SemanticType,
}

pub(in crate::semantic) struct ProtocolChecker;

impl ProtocolChecker {
    pub(in crate::semantic) fn collect_inherited_protocol_methods(
        analyzer: &SemanticAnalyzer,
        _protocol_id: TypeId,
        parent_id: TypeId,
    ) -> Vec<InheritedProtocolMethod> {
        let mut out = Vec::new();
        let mut cursor = Some(parent_id);
        while let Some(current) = cursor {
            let is_protocol = analyzer
                .type_table
                .get_struct(current)
                .is_some_and(|info| info.is_protocol);
            if is_protocol {
                if let Some(info) = analyzer.type_table.get_struct(current) {
                    for (method_name, method_type_id) in &info.methods {
                        if out.iter().any(|m: &InheritedProtocolMethod| &m.name == method_name) {
                            continue;
                        }
                        let key = SymbolCollector::method_symbol_key(current, method_name);
                        if let Some(signature) = analyzer.functions.get(&key) {
                            out.push(InheritedProtocolMethod {
                                name: method_name.to_string(),
                                type_id: *method_type_id,
                                param_types: signature.param_types.clone(),
                                return_type: signature.return_type,
                            });
                        }
                    }
                }
            }
            cursor = analyzer
                .type_table
                .get_struct(current)
                .and_then(|info| info.parent);
        }
        out
    }

    pub(in crate::semantic) fn validate_protocol_conformance(
        analyzer: &SemanticAnalyzer,
        impl_type: SemanticType,
        protocol_id: TypeId,
        source: &str,
    ) -> Option<()> {
        let SemanticType::Struct(impl_raw) = impl_type else {
            return None;
        };
        let impl_id = TypeId(impl_raw);
        if analyzer
            .type_table
            .get_struct(impl_id)
            .is_some_and(|info| info.is_protocol)
        {
            return None;
        }

        let protocol_methods = Self::collect_inherited_protocol_methods(analyzer, protocol_id, protocol_id);
        for method in &protocol_methods {
            let impl_signature = Self::resolve_method_symbol_key(analyzer, impl_id, &method.name)
                .or_else(|| Self::resolve_method_symbol_key_in_structs(analyzer, impl_id, &method.name))
                .and_then(|key| analyzer.functions.get(&key).cloned());

            let Some(impl_signature) = impl_signature else {
                return None;
            };

            if impl_signature.param_types.len() != method.param_types.len() {
                return None;
            }

            for (impl_t, proto_t) in impl_signature
                .param_types
                .iter()
                .zip(method.param_types.iter())
            {
                if !Self::variance_param_compatible(analyzer, *impl_t, *proto_t) {
                    let _ = source;
                    return None;
                }
            }

            if !Self::variance_return_compatible(analyzer, impl_signature.return_type, method.return_type) {
                return None;
            }
        }

        Some(())
    }

    pub(in crate::semantic) fn validate_protocol_method_call(
        analyzer: &mut SemanticAnalyzer,
        impl_type: SemanticType,
        protocol_id: TypeId,
        method_name: &str,
        call_span: Span,
        source: &str,
    ) -> Option<SemanticType> {
        let SemanticType::Struct(impl_id_raw) = impl_type else {
            return None;
        };
        let impl_id = TypeId(impl_id_raw);

        if analyzer
            .type_table
            .get_struct(impl_id)
            .is_some_and(|info| info.is_protocol)
        {
            return None;
        }

        let impl_method_key = Self::resolve_method_symbol_key(analyzer, impl_id, method_name);
        let impl_signature = impl_method_key
            .as_ref()
            .and_then(|key| analyzer.functions.get(key).cloned());

        let protocol_methods = Self::collect_inherited_protocol_methods(analyzer, protocol_id, protocol_id);
        let Some(protocol_signature) = protocol_methods
            .iter()
            .find(|m| m.name == method_name)
            .map(|m| FunctionSignature {
                type_id: m.type_id.0,
                param_types: m.param_types.clone(),
                return_type: m.return_type,
            })
        else {
            analyzer.push_semantic_error(
                call_span,
                source,
                format!(
                    "Type does not conform to protocol: method '{}' is not declared in the protocol.",
                    method_name
                ),
            );
            return None;
        };

        let Some(impl_signature) = impl_signature else {
            analyzer.push_semantic_error(
                call_span,
                source,
                format!(
                    "Type does not conform to protocol: method '{}' is not declared in the implementing type.",
                    method_name
                ),
            );
            return None;
        };

        if impl_signature.param_types.len() != protocol_signature.param_types.len() {
            analyzer.push_type_error(
                call_span,
                source,
                format!(
                    "Type does not conform to protocol: method '{}' expects {} argument(s), but type provides {}.",
                    method_name,
                    protocol_signature.param_types.len(),
                    impl_signature.param_types.len()
                ),
            );
            return None;
        }

        for (index, (impl_t, proto_t)) in impl_signature
            .param_types
            .iter()
            .zip(protocol_signature.param_types.iter())
            .enumerate()
        {
            if !Self::variance_param_compatible(analyzer, *impl_t, *proto_t) {
                analyzer.push_type_error(
                    call_span,
                    source,
                    format!(
                        "Type does not conform to protocol: method '{}' argument #{} has incompatible variance: expected {} (contravariant) in protocol, got {}.",
                        method_name,
                        index + 1,
                        proto_t.display_name_with_table(&analyzer.type_table),
                        impl_t.display_name_with_table(&analyzer.type_table)
                    ),
                );
                return None;
            }
        }

        if !Self::variance_return_compatible(analyzer, impl_signature.return_type, protocol_signature.return_type)
        {
            analyzer.push_type_error(
                call_span,
                source,
                format!(
                    "Type does not conform to protocol: method '{}' return type is incompatible (covariant): expected {} in protocol, got {}.",
                    method_name,
                    protocol_signature.return_type.display_name_with_table(&analyzer.type_table),
                    impl_signature.return_type.display_name_with_table(&analyzer.type_table)
                ),
            );
            return None;
        }

        Some(impl_signature.return_type)
    }

    pub(in crate::semantic) fn check_protocol_variance(
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

                let return_type = TypeResolver::resolve_annotation_type(
                    analyzer,
                    &method.return_type_annotation,
                    source,
                )
                .unwrap_or(SemanticType::Unknown);

                if let Some(parent_signature) = Self::find_protocol_method_in_parent(
                    analyzer,
                    receiver_type_id,
                    &method.name,
                ) {
                    let params_compatible = param_types
                        .iter()
                        .zip(parent_signature.param_types.iter())
                        .all(|(p_t, pp_t)| Self::variance_param_compatible(analyzer, *p_t, *pp_t));
                    let arity_matches = param_types.len() == parent_signature.param_types.len();
                    let return_compatible = Self::variance_return_compatible(
                        analyzer,
                        return_type,
                        parent_signature.return_type,
                    );

                    if !arity_matches
                        || !params_compatible
                        || !return_compatible
                    {
                        analyzer.push_semantic_error(
                            method.name_span,
                            source,
                            format!(
                                "Method '{}' override in protocol '{}' does not respect variance constraints of parent protocol.",
                                method.name, protocol_decl.name
                            ),
                        );
                    }
                }
            }
        }
    }

    fn find_protocol_method_in_parent(
        analyzer: &SemanticAnalyzer,
        type_id: TypeId,
        method_name: &str,
    ) -> Option<FunctionSignature> {
        let parent_id = analyzer.type_table.get_struct(type_id)?.parent?;
        if !analyzer
            .type_table
            .get_struct(parent_id)
            .is_some_and(|info| info.is_protocol)
        {
            return None;
        }
        let key = SymbolCollector::method_symbol_key(parent_id, method_name);
        if let Some(signature) = analyzer.functions.get(&key) {
            return Some(signature.clone());
        }
        Self::find_protocol_method_in_parent(analyzer, parent_id, method_name)
    }

    fn variance_param_compatible(
        analyzer: &SemanticAnalyzer,
        impl_type: SemanticType,
        protocol_type: SemanticType,
    ) -> bool {
        Self::variance_compatible(analyzer, impl_type, protocol_type, true)
    }

    fn variance_return_compatible(
        analyzer: &SemanticAnalyzer,
        impl_type: SemanticType,
        protocol_type: SemanticType,
    ) -> bool {
        Self::variance_compatible(analyzer, impl_type, protocol_type, false)
    }

    fn variance_compatible(
        analyzer: &SemanticAnalyzer,
        impl_type: SemanticType,
        protocol_type: SemanticType,
        contravariant: bool,
    ) -> bool {
        if impl_type == SemanticType::Unknown || protocol_type == SemanticType::Unknown {
            return true;
        }
        if impl_type == protocol_type {
            return true;
        }
        let (left, right) = if contravariant {
            (protocol_type, impl_type)
        } else {
            (impl_type, protocol_type)
        };
        match (left, right) {
            (SemanticType::Struct(a), SemanticType::Struct(b)) => {
                let a_id = TypeId(a);
                let b_id = TypeId(b);
                analyzer
                    .type_table
                    .get_struct(a_id)
                    .zip(analyzer.type_table.get_struct(b_id))
                    .is_some_and(|(info_a, info_b)| !info_a.is_protocol && !info_b.is_protocol)
                    && Self::is_subtype(analyzer, b_id, a_id)
            }
            (SemanticType::Struct(_), _) => false,
            _ => {
                if left == right {
                    return true;
                }
                if let SemanticType::Struct(b) = right {
                    return b == analyzer.type_table.object.0;
                }
                false
            }
        }
    }

    fn is_subtype(analyzer: &SemanticAnalyzer, child: TypeId, parent: TypeId) -> bool {
        if child == parent {
            return true;
        }
        if parent == analyzer.type_table.object {
            return true;
        }
        let mut cursor = analyzer
            .type_table
            .get_struct(child)
            .and_then(|info| info.parent);
        while let Some(current) = cursor {
            if current == parent {
                return true;
            }
            cursor = analyzer
                .type_table
                .get_struct(current)
                .and_then(|info| info.parent);
        }
        false
    }

    fn resolve_method_symbol_key(analyzer: &SemanticAnalyzer, receiver: TypeId, method_name: &str) -> Option<String> {
        let mut cursor = Some(receiver);
        while let Some(current) = cursor {
            let key = SymbolCollector::method_symbol_key(current, method_name);
            if analyzer.function_symbols.contains_key(&key) {
                return Some(key);
            }
            cursor = analyzer
                .type_table
                .get_struct(current)
                .and_then(|info| info.parent);
        }
        None
    }

    fn resolve_method_symbol_key_in_structs(
        analyzer: &SemanticAnalyzer,
        receiver: TypeId,
        method_name: &str,
    ) -> Option<String> {
        let mut cursor = Some(receiver);
        while let Some(current) = cursor {
            if let Some(info) = analyzer.type_table.get_struct(current) {
                if info.methods.iter().any(|(name, _)| name == method_name) {
                    let key = SymbolCollector::method_symbol_key(current, method_name);
                    if analyzer.function_symbols.contains_key(&key) {
                        return Some(key);
                    }
                }
                cursor = info.parent;
            } else {
                return None;
            }
        }
        None
    }
}
