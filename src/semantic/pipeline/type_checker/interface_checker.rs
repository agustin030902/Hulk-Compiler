use crate::parser::expression::InterfaceDecl;

use super::super::super::{
    analyzer::SemanticAnalyzer,
    helper::{FunctionSignature, SemanticType, TypeId},
};
use super::super::{SymbolCollector, TypeResolver};

pub(in crate::semantic) struct InheritedInterfaceMethod {
    pub name: String,
    pub type_id: TypeId,
    pub param_types: Vec<SemanticType>,
    pub return_type: SemanticType,
}

pub(in crate::semantic) struct InterfaceChecker;

impl InterfaceChecker {
    pub(in crate::semantic) fn collect_inherited_interface_methods(
        analyzer: &SemanticAnalyzer,
        _interface_id: TypeId,
        parent_id: TypeId,
    ) -> Vec<InheritedInterfaceMethod> {
        let mut out = Vec::new();
        let mut cursor = Some(parent_id);
        while let Some(current) = cursor {
            let is_interface = analyzer
                .type_table
                .get_struct(current)
                .is_some_and(|info| info.is_interface);
            if is_interface {
                if let Some(info) = analyzer.type_table.get_struct(current) {
                    for (method_name, method_type_id) in &info.methods {
                        if out.iter().any(|m: &InheritedInterfaceMethod| &m.name == method_name) {
                            continue;
                        }
                        let key = SymbolCollector::method_symbol_key(current, method_name);
                        if let Some(signature) = analyzer.functions.get(&key) {
                            out.push(InheritedInterfaceMethod {
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

    pub(in crate::semantic) fn validate_interface_conformance(
        analyzer: &SemanticAnalyzer,
        impl_type: SemanticType,
        interface_id: TypeId,
        source: &str,
    ) -> Option<()> {
        let SemanticType::Struct(impl_raw) = impl_type else {
            return None;
        };
        let impl_id = TypeId(impl_raw);
        if analyzer
            .type_table
            .get_struct(impl_id)
            .is_some_and(|info| info.is_interface)
        {
            return None;
        }

        let interface_methods = Self::collect_inherited_interface_methods(analyzer, interface_id, interface_id);
        for method in &interface_methods {
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

    pub(in crate::semantic) fn check_interface_variance(
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

                if let Some(parent_signature) = Self::find_interface_method_in_parent(
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
                                "Method '{}' override in interface '{}' does not respect variance constraints of parent interface.",
                                method.name, interface_decl.name
                            ),
                        );
                    }
                }
            }
        }
    }

    fn find_interface_method_in_parent(
        analyzer: &SemanticAnalyzer,
        type_id: TypeId,
        method_name: &str,
    ) -> Option<FunctionSignature> {
        let parent_id = analyzer.type_table.get_struct(type_id)?.parent?;
        if !analyzer
            .type_table
            .get_struct(parent_id)
            .is_some_and(|info| info.is_interface)
        {
            return None;
        }
        let key = SymbolCollector::method_symbol_key(parent_id, method_name);
        if let Some(signature) = analyzer.functions.get(&key) {
            return Some(signature.clone());
        }
        Self::find_interface_method_in_parent(analyzer, parent_id, method_name)
    }

    fn variance_param_compatible(
        analyzer: &SemanticAnalyzer,
        impl_type: SemanticType,
        interface_type: SemanticType,
    ) -> bool {
        Self::variance_compatible(analyzer, impl_type, interface_type, true)
    }

    fn variance_return_compatible(
        analyzer: &SemanticAnalyzer,
        impl_type: SemanticType,
        interface_type: SemanticType,
    ) -> bool {
        Self::variance_compatible(analyzer, impl_type, interface_type, false)
    }

    fn variance_compatible(
        analyzer: &SemanticAnalyzer,
        impl_type: SemanticType,
        interface_type: SemanticType,
        contravariant: bool,
    ) -> bool {
        if impl_type == SemanticType::Unknown || interface_type == SemanticType::Unknown {
            return true;
        }
        if impl_type == interface_type {
            return true;
        }
        let (left, right) = if contravariant {
            (interface_type, impl_type)
        } else {
            (impl_type, interface_type)
        };
        match (left, right) {
            (SemanticType::Struct(a), SemanticType::Struct(b)) => {
                let a_id = TypeId(a);
                let b_id = TypeId(b);
                analyzer
                    .type_table
                    .get_struct(a_id)
                    .zip(analyzer.type_table.get_struct(b_id))
                    .is_some_and(|(info_a, info_b)| !info_a.is_interface && !info_b.is_interface)
                    && Self::is_subtype(analyzer, a_id, b_id)
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
