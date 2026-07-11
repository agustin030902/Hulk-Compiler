//! Emisión de arreglos.
//!
//! Representación en runtime: bloque de heap `[i64 longitud][elem0][elem1]…`
//! donde cada elemento ocupa 8 bytes (double o puntero). `new T[n]` reserva
//! con `calloc`, de modo que los Number arrancan en 0 y los arreglos anidados
//! en null. `a.size()` lee la cabecera de longitud.

use crate::parser::expression::{ArrayLiteralExpr, Expr, IndexExpr, NewArrayExpr};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType, VariableInfo};

impl LlvmBackend {
    /// Busca (por estructura) el ValueType de arreglo cuyo elemento es `elem`.
    /// Todos los tipos de arreglo del programa fueron internados durante el
    /// análisis semántico, así que la búsqueda inversa siempre encuentra el id.
    pub(in crate::codegen::llvm) fn array_type_for_elem(
        &self,
        elem: ValueType,
    ) -> Option<ValueType> {
        self.array_elems
            .iter()
            .find(|(_, value)| **value == elem)
            .map(|(array_id, _)| ValueType::Array(*array_id))
    }

    pub(in crate::codegen::llvm) fn array_elem_type(&self, array_id: u32) -> Option<ValueType> {
        self.array_elems.get(&array_id).copied()
    }

    /// Resuelve el nombre de tipo de elemento de `new T[n]` (posiblemente con
    /// sufijos `[]`) al ValueType correspondiente.
    pub(in crate::codegen::llvm) fn resolve_elem_type_name(
        &mut self,
        name: &str,
    ) -> Option<ValueType> {
        let mut base = name;
        let mut dims = 0usize;
        while let Some(stripped) = base.strip_suffix("[]") {
            base = stripped;
            dims += 1;
        }

        let mut current = match base {
            "Number" => ValueType::Double,
            "Boolean" => ValueType::Bool,
            "String" => ValueType::StringPtr,
            _ => {
                let Some(type_id) = self.type_ids.get(base).copied() else {
                    self.semantic_error(format!(
                        "Unknown element type '{}' in array construction.",
                        base
                    ));
                    return None;
                };
                ValueType::Struct(type_id)
            }
        };

        for _ in 0..dims {
            let Some(array_type) = self.array_type_for_elem(current) else {
                self.semantic_error(format!(
                    "Array type for element '{}' was not registered during analysis.",
                    name
                ));
                return None;
            };
            current = array_type;
        }
        Some(current)
    }

    /// Dirección del elemento `idx` (double) del arreglo `arr_repr`.
    fn emit_elem_ptr(&mut self, arr_repr: &str, idx_i64: &str, elem: ValueType) -> String {
        let offset = self.next_temp();
        self.emit_body(format!("{offset} = mul i64 {idx_i64}, 8"));
        let with_header = self.next_temp();
        self.emit_body(format!("{with_header} = add i64 {offset}, 8"));
        let raw_ptr = self.next_temp();
        self.emit_body(format!(
            "{raw_ptr} = getelementptr i8, i8* {arr_repr}, i64 {with_header}"
        ));
        let typed_ptr = self.next_temp();
        self.emit_body(format!(
            "{typed_ptr} = bitcast i8* {raw_ptr} to {}*",
            elem.llvm_type()
        ));
        typed_ptr
    }

    fn emit_index_to_i64(&mut self, index: &ValueRef) -> Option<String> {
        if index.value_type != ValueType::Double {
            self.semantic_error(format!(
                "Array index must be a Number, but got {}.",
                self.type_name_for_value_type(index.value_type)
            ));
            return None;
        }
        let as_i64 = self.next_temp();
        self.emit_body(format!("{as_i64} = fptosi double {} to i64", index.repr));
        Some(as_i64)
    }

    /// Reserva `[i64 len][len x 8 bytes]` inicializado a cero y devuelve
    /// (repr del arreglo, repr de la longitud i64).
    fn emit_array_alloc(&mut self, len_i64: &str) -> String {
        let payload = self.next_temp();
        self.emit_body(format!("{payload} = mul i64 {len_i64}, 8"));
        let total = self.next_temp();
        self.emit_body(format!("{total} = add i64 {payload}, 8"));
        let arr = self.next_temp();
        self.emit_body(format!("{arr} = call i8* @calloc(i64 {total}, i64 1)"));
        let len_ptr = self.next_temp();
        self.emit_body(format!("{len_ptr} = bitcast i8* {arr} to i64*"));
        self.emit_body(format!("store i64 {len_i64}, i64* {len_ptr}"));
        arr
    }

    pub(in crate::codegen::llvm) fn emit_array_literal(
        &mut self,
        literal: &ArrayLiteralExpr,
    ) -> Option<ValueRef> {
        let mut elements = Vec::with_capacity(literal.elements.len());
        let mut elem_type: Option<ValueType> = None;

        for element in &literal.elements {
            let value = self.emit_expr(element)?;
            match elem_type {
                None => elem_type = Some(value.value_type),
                Some(expected) if expected != value.value_type => {
                    if !self.are_compatible_value_types(expected, value.value_type) {
                        self.semantic_error(format!(
                            "Array literal elements must share one type: expected {}, but got {}.",
                            self.type_name_for_value_type(expected),
                            self.type_name_for_value_type(value.value_type)
                        ));
                        return None;
                    }
                }
                Some(_) => {}
            }
            elements.push(value);
        }

        let elem_type = elem_type.unwrap_or(ValueType::Double);
        let Some(array_type) = self.array_type_for_elem(elem_type) else {
            self.semantic_error(
                "Array literal type was not registered during analysis.".to_string(),
            );
            return None;
        };

        let arr = self.emit_array_alloc(&elements.len().to_string());
        for (index, value) in elements.iter().enumerate() {
            let stored = self
                .value_repr_for_expected_type(elem_type, value)
                .unwrap_or_else(|| value.repr.clone());
            let elem_ptr = self.emit_elem_ptr(&arr, &index.to_string(), elem_type);
            self.emit_body(format!(
                "store {} {}, {}* {}",
                elem_type.llvm_type(),
                stored,
                elem_type.llvm_type(),
                elem_ptr
            ));
        }

        Some(ValueRef {
            value_type: array_type,
            repr: arr,
        })
    }

    pub(in crate::codegen::llvm) fn emit_new_array(
        &mut self,
        new_array: &NewArrayExpr,
    ) -> Option<ValueRef> {
        let elem_type = self.resolve_elem_type_name(&new_array.elem_type_name)?;
        let Some(array_type) = self.array_type_for_elem(elem_type) else {
            self.semantic_error(format!(
                "Array type for element '{}' was not registered during analysis.",
                new_array.elem_type_name
            ));
            return None;
        };

        let size = self.emit_expr(&new_array.size)?;
        let len_i64 = self.emit_index_to_i64(&size)?;
        let arr = self.emit_array_alloc(&len_i64);

        if let Some(init) = &new_array.init {
            // for (i = 0; i < len; i++) arr[i] = init(i)
            let counter_ptr = self.next_temp();
            self.emit_body(format!("{counter_ptr} = alloca i64"));
            self.emit_body(format!("store i64 0, i64* {counter_ptr}"));

            let cond_label = self.next_label("arrinit.cond");
            let body_label = self.next_label("arrinit.body");
            let end_label = self.next_label("arrinit.end");

            self.emit_body(format!("br label %{cond_label}"));
            self.emit_body(format!("{cond_label}:"));
            let counter = self.next_temp();
            self.emit_body(format!("{counter} = load i64, i64* {counter_ptr}"));
            let cmp = self.next_temp();
            self.emit_body(format!("{cmp} = icmp slt i64 {counter}, {len_i64}"));
            self.emit_body(format!("br i1 {cmp}, label %{body_label}, label %{end_label}"));

            self.emit_body(format!("{body_label}:"));
            let counter_in_body = self.next_temp();
            self.emit_body(format!("{counter_in_body} = load i64, i64* {counter_ptr}"));
            let idx_double = self.next_temp();
            self.emit_body(format!(
                "{idx_double} = sitofp i64 {counter_in_body} to double"
            ));

            // La variable de índice se liga como Number en un scope propio.
            let var_ptr = self.next_temp();
            self.emit_body(format!("{var_ptr} = alloca double"));
            self.emit_body(format!("store double {idx_double}, double* {var_ptr}"));
            self.push_scope();
            self.bind_current_scope(
                init.var_name.clone(),
                VariableInfo {
                    ptr_name: var_ptr,
                    value_type: ValueType::Double,
                },
            );
            let body_value = self.emit_expr(&init.body);
            self.pop_scope();
            let body_value = body_value?;

            let stored = self
                .value_repr_for_expected_type(elem_type, &body_value)
                .unwrap_or_else(|| body_value.repr.clone());
            let elem_ptr = self.emit_elem_ptr(&arr, &counter_in_body, elem_type);
            self.emit_body(format!(
                "store {} {}, {}* {}",
                elem_type.llvm_type(),
                stored,
                elem_type.llvm_type(),
                elem_ptr
            ));

            let next = self.next_temp();
            self.emit_body(format!("{next} = add i64 {counter_in_body}, 1"));
            self.emit_body(format!("store i64 {next}, i64* {counter_ptr}"));
            self.emit_body(format!("br label %{cond_label}"));
            self.emit_body(format!("{end_label}:"));
        }

        Some(ValueRef {
            value_type: array_type,
            repr: arr,
        })
    }

    pub(in crate::codegen::llvm) fn emit_index_expr(
        &mut self,
        index_expr: &IndexExpr,
    ) -> Option<ValueRef> {
        let object = self.emit_expr(&index_expr.object)?;
        let ValueType::Array(array_id) = object.value_type else {
            self.semantic_error(format!(
                "Indexing requires an array, but got {}.",
                self.type_name_for_value_type(object.value_type)
            ));
            return None;
        };
        let Some(elem_type) = self.array_elem_type(array_id) else {
            self.semantic_error("Array element type is unknown at code generation.".to_string());
            return None;
        };

        let index = self.emit_expr(&index_expr.index)?;
        let idx_i64 = self.emit_index_to_i64(&index)?;
        let elem_ptr = self.emit_elem_ptr(&object.repr, &idx_i64, elem_type);
        let value = self.next_temp();
        self.emit_body(format!(
            "{value} = load {}, {}* {}",
            elem_type.llvm_type(),
            elem_type.llvm_type(),
            elem_ptr
        ));

        Some(ValueRef {
            value_type: elem_type,
            repr: value,
        })
    }

    pub(in crate::codegen::llvm) fn emit_index_assign(
        &mut self,
        object: &Expr,
        index: &Expr,
        value: &Expr,
    ) -> Option<ValueRef> {
        let object_ref = self.emit_expr(object)?;
        let ValueType::Array(array_id) = object_ref.value_type else {
            self.semantic_error(format!(
                "Indexed assignment requires an array, but got {}.",
                self.type_name_for_value_type(object_ref.value_type)
            ));
            return None;
        };
        let Some(elem_type) = self.array_elem_type(array_id) else {
            self.semantic_error("Array element type is unknown at code generation.".to_string());
            return None;
        };

        let index_ref = self.emit_expr(index)?;
        let idx_i64 = self.emit_index_to_i64(&index_ref)?;

        let value_ref = self.emit_expr(value)?;
        if !self.are_compatible_value_types(elem_type, value_ref.value_type) {
            self.semantic_error(format!(
                "Indexed assignment ':=' requires element type {}, but expression is {}.",
                self.type_name_for_value_type(elem_type),
                self.type_name_for_value_type(value_ref.value_type)
            ));
            return None;
        }
        let stored = self
            .value_repr_for_expected_type(elem_type, &value_ref)
            .unwrap_or_else(|| value_ref.repr.clone());

        let elem_ptr = self.emit_elem_ptr(&object_ref.repr, &idx_i64, elem_type);
        self.emit_body(format!(
            "store {} {}, {}* {}",
            elem_type.llvm_type(),
            stored,
            elem_type.llvm_type(),
            elem_ptr
        ));

        Some(ValueRef {
            value_type: elem_type,
            repr: value_ref.repr,
        })
    }

    /// `arr.size()`: lee la cabecera i64 y la devuelve como Number.
    pub(in crate::codegen::llvm) fn emit_array_size(&mut self, array: &ValueRef) -> ValueRef {
        let len_ptr = self.next_temp();
        self.emit_body(format!("{len_ptr} = bitcast i8* {} to i64*", array.repr));
        let len = self.next_temp();
        self.emit_body(format!("{len} = load i64, i64* {len_ptr}"));
        let as_double = self.next_temp();
        self.emit_body(format!("{as_double} = sitofp i64 {len} to double"));
        ValueRef {
            value_type: ValueType::Double,
            repr: as_double,
        }
    }
}
