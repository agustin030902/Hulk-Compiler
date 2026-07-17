//! Compatibilidad de tipos en codegen: subtipado de structs por jerarquía,
//! nulabilidad de tipos-puntero e igualdad **estructural** de tipos función y
//! arreglo (dos ids internados con la misma forma son el mismo tipo).

use crate::codegen::llvm::helper::state::{ValueRef, ValueType};

use super::LlvmBackend;

impl LlvmBackend {
    /// Nombre legible de un ValueType para mensajes de error.
    pub(in crate::codegen::llvm) fn type_name_for_value_type(&self, vt: ValueType) -> String {
        match vt {
            ValueType::Struct(id) => self
                .type_ids
                .iter()
                .find(|(_, tid)| **tid == id)
                .map(|(name, _)| name.clone())
                .unwrap_or_else(|| "Struct".to_string()),
            _ => vt.display_name().to_string(),
        }
    }

    /// `Null` solo es asignable a tipos representados como puntero.
    pub(in crate::codegen::llvm) fn is_nullable_value_type(value_type: ValueType) -> bool {
        matches!(
            value_type,
            ValueType::Null
                | ValueType::StringPtr
                | ValueType::Function(_)
                | ValueType::Struct(_)
                | ValueType::Array(_)
        )
    }

    pub(in crate::codegen::llvm) fn are_compatible_value_types(
        &self,
        expected: ValueType,
        actual: ValueType,
    ) -> bool {
        if expected == actual {
            return true;
        }

        if actual == ValueType::Null && Self::is_nullable_value_type(expected) {
            return true;
        }

        if expected == ValueType::Null && Self::is_nullable_value_type(actual) {
            return true;
        }

        match (expected, actual) {
            (ValueType::Struct(parent), ValueType::Struct(child)) => {
                self.is_subtype_struct(child, parent)
            }
            // Tipos función: igualdad estructural de firmas — entradas
            // internadas distintas con la misma forma son el mismo tipo.
            (ValueType::Function(left), ValueType::Function(right)) => {
                self.function_types_equal(left, right)
            }
            _ => false,
        }
    }

    pub(in crate::codegen::llvm) fn function_types_equal(&self, left: u32, right: u32) -> bool {
        if left == right {
            return true;
        }
        let (Some((params_a, ret_a)), Some((params_b, ret_b))) = (
            self.function_types.get(&left),
            self.function_types.get(&right),
        ) else {
            return false;
        };
        if params_a.len() != params_b.len() {
            return false;
        }
        params_a
            .iter()
            .zip(params_b.iter())
            .all(|(x, y)| self.value_types_structurally_equal(*x, *y))
            && self.value_types_structurally_equal(*ret_a, *ret_b)
    }

    fn value_types_structurally_equal(&self, left: ValueType, right: ValueType) -> bool {
        if left == right {
            return true;
        }
        match (left, right) {
            (ValueType::Function(a), ValueType::Function(b)) => self.function_types_equal(a, b),
            (ValueType::Array(a), ValueType::Array(b)) => {
                match (self.array_elems.get(&a), self.array_elems.get(&b)) {
                    (Some(x), Some(y)) => self.value_types_structurally_equal(*x, *y),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Representación de `value_ref` cuando el contexto espera `expected`:
    /// idéntico → tal cual; `null` sobre nullable → `null`; subtipo con la
    /// misma representación LLVM → tal cual. `None` si son incompatibles.
    pub(in crate::codegen::llvm) fn value_repr_for_expected_type(
        &self,
        expected: ValueType,
        value_ref: &ValueRef,
    ) -> Option<String> {
        if value_ref.value_type == expected {
            return Some(value_ref.repr.clone());
        }

        if value_ref.value_type == ValueType::Null && Self::is_nullable_value_type(expected) {
            return Some("null".to_string());
        }

        if self.are_compatible_value_types(expected, value_ref.value_type)
            && expected.llvm_type() == value_ref.value_type.llvm_type()
        {
            return Some(value_ref.repr.clone());
        }

        None
    }

    /// Subtipado nominal de structs recorriendo `type_parents` (la jerarquía
    /// semántica completa, incluidas interfaces). `Object` es raíz universal.
    pub(in crate::codegen::llvm) fn is_subtype_struct(&self, child: u32, parent: u32) -> bool {
        if child == parent {
            return true;
        }

        if self
            .type_ids
            .get("Object")
            .is_some_and(|object_id| *object_id == parent)
        {
            return true;
        }

        let mut cursor = self.type_parents.get(&child).copied();
        while let Some(parent_id) = cursor {
            if parent_id == parent {
                return true;
            }
            cursor = self.type_parents.get(&parent_id).copied();
        }

        false
    }
}
