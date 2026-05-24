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
            SemanticType::Function(_) => ValueType::Function,
            SemanticType::Struct(type_id) => ValueType::Struct(type_id),
            SemanticType::Unknown => {
                self.semantic_error(format!(
                    "Could not infer a concrete type for {context} before code generation."
                ));
                return None;
            }
        };

        Some(lowered)
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
            ValueType::StringPtr | ValueType::Null | ValueType::Function | ValueType::Struct(_) => {
                (8, 8)
            }
            ValueType::Unit => (1, 1),
        }
    }
}
