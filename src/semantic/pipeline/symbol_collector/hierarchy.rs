//! Utilidades de jerarquía de tipos compartidas por las demás pasadas del
//! recolector y por el resto de `semantic/`.

use crate::semantic::analyzer::SemanticAnalyzer;
use crate::semantic::helper::{FunctionSignature, TypeId};

use super::SymbolCollector;

impl SymbolCollector {
    pub(in crate::semantic) fn is_interface(
        analyzer: &SemanticAnalyzer,
        type_id: TypeId,
    ) -> bool {
        analyzer
            .type_table
            .get_struct(type_id)
            .is_some_and(|info| info.is_interface)
    }

    /// `true` si `child_id` aparece en la cadena de ancestros de `parent_id`
    /// (adoptar ese padre crearía un ciclo de herencia).
    pub(super) fn is_circular_inheritance(
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

    /// Busca la firma de `method_name` subiendo por la cadena de padres de
    /// `type_id` (sin incluir al propio tipo).
    pub(in crate::semantic) fn find_method_in_parent(
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
