use crate::semantic::{SemanticAnalyzer, SemanticType, TypeId, TypeInfo};

use super::LlvmBackend;
use crate::codegen::llvm::helper::state::ValueType;

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn lower_semantic_type(
        &mut self,
        semantic_type: SemanticType,
        context: &str,
    ) -> Option<ValueType> {
        let lowered = match semantic_type {
            SemanticType::Number => ValueType::Double,
            SemanticType::Boolean => ValueType::Bool,
            SemanticType::String => ValueType::StringPtr,
            SemanticType::Unit => ValueType::Unit,
            SemanticType::Null => ValueType::Null,
            SemanticType::Function(type_id) => ValueType::Function(type_id),
            SemanticType::Struct(type_id) => ValueType::Struct(type_id),
            SemanticType::Array(type_id) => ValueType::Array(type_id),
            SemanticType::Unknown => {
                self.semantic_error(format!(
                    "Could not infer a concrete type for {context} before code generation."
                ));
                return None;
            }
        };

        Some(lowered)
    }

    /// Resuelve el nombre de una anotación (simple, arreglo `T[]` o tipo
    /// función canónico `(A,B)->C`) al ValueType correspondiente. Los tipos
    /// compuestos ya fueron internados por el análisis semántico, así que
    /// aquí solo se buscan por estructura.
    pub(in crate::codegen::llvm) fn resolve_annotation_value_type(
        &mut self,
        name: &str,
    ) -> Option<ValueType> {
        if name.starts_with('(') {
            let (param_names, ret_name) =
                crate::parser::expression::split_function_type_name(name)?;
            let mut params = Vec::with_capacity(param_names.len());
            for param_name in &param_names {
                params.push(self.resolve_annotation_value_type(param_name)?);
            }
            let ret = self.resolve_annotation_value_type(&ret_name)?;
            return self.function_type_for(&params, ret);
        }
        self.resolve_elem_type_name(name)
    }

    /// Búsqueda estructural inversa de una firma en `function_types`.
    pub(in crate::codegen::llvm) fn function_type_for(
        &self,
        params: &[ValueType],
        ret: ValueType,
    ) -> Option<ValueType> {
        self.function_types
            .iter()
            .find(|(_, (entry_params, entry_ret))| {
                entry_params.len() == params.len()
                    && entry_params
                        .iter()
                        .zip(params.iter())
                        .all(|(a, b)| *a == *b || self.are_compatible_value_types(*a, *b))
                    && (*entry_ret == ret || self.are_compatible_value_types(*entry_ret, ret))
            })
            .map(|(id, _)| ValueType::Function(*id))
    }

    /// Variante silenciosa de `lower_semantic_type`: devuelve None ante un
    /// tipo desconocido sin registrar error (para entradas de tabla que no
    /// participan en la generación, p. ej. firmas parcialmente inferidas).
    pub(in crate::codegen::llvm) fn lower_semantic_type_quiet(
        semantic_type: SemanticType,
    ) -> Option<ValueType> {
        Some(match semantic_type {
            SemanticType::Number => ValueType::Double,
            SemanticType::Boolean => ValueType::Bool,
            SemanticType::String => ValueType::StringPtr,
            SemanticType::Unit => ValueType::Unit,
            SemanticType::Null => ValueType::Null,
            SemanticType::Function(type_id) => ValueType::Function(type_id),
            SemanticType::Struct(type_id) => ValueType::Struct(type_id),
            SemanticType::Array(type_id) => ValueType::Array(type_id),
            SemanticType::Unknown => return None,
        })
    }

    pub(in crate::codegen::llvm) fn semantic_type_from_type_id(
        &self,
        analyzer: &SemanticAnalyzer,
        type_id: TypeId,
    ) -> SemanticType {
        match analyzer.type_table().get(type_id) {
            TypeInfo::Number => SemanticType::Number,
            TypeInfo::Boolean => SemanticType::Boolean,
            TypeInfo::String => SemanticType::String,
            TypeInfo::Unit => SemanticType::Unit,
            TypeInfo::Null => SemanticType::Null,
            TypeInfo::Unknown => SemanticType::Unknown,
            TypeInfo::Function(_) => SemanticType::Function(type_id.0),
            TypeInfo::Type(_) => SemanticType::Struct(type_id.0),
            TypeInfo::Array { .. } => SemanticType::Array(type_id.0),
        }
    }

    pub(in crate::codegen::llvm) fn align_to(value: usize, alignment: usize) -> usize {
        if alignment <= 1 {
            return value;
        }
        let remainder = value % alignment;
        if remainder == 0 {
            value
        } else {
            value + (alignment - remainder)
        }
    }

    pub(in crate::codegen::llvm) fn value_layout(value_type: ValueType) -> (usize, usize) {
        match value_type {
            ValueType::Double => (8, 8),
            ValueType::Bool => (1, 1),
            ValueType::StringPtr
            | ValueType::Null
            | ValueType::Function(_)
            | ValueType::Struct(_)
            | ValueType::Array(_) => (8, 8),
            ValueType::Unit => (1, 1),
        }
    }
}
