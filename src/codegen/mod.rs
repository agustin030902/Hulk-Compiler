//! # Generación de código
//!
//! Emisión de **LLVM IR en formato texto** (sin `inkwell`): el backend
//! mantiene buffers de líneas separados para globals, funciones y el cuerpo de
//! `main`, y los ensambla en un módulo `.ll` que `clang` valida y compila.
//! Esta elección evita el acople a una versión concreta de LLVM y hace el IR
//! trivialmente inspeccionable.
//!
//! Convenciones de representación en runtime:
//!
//! - **Números**: `double` (f64) unificado; booleanos como `i1`.
//! - **Objetos**: bloque de heap `[type_id i64][campos del padre][propios]`;
//!   el subtipado en runtime (`is`) consulta la tabla global
//!   `@hulk_type_parents` con `@hulk_is_subtype`.
//! - **Dispatch dinámico**: cascada de comparaciones por type-tag con búsqueda
//!   del método en la jerarquía completa (incluye implementaciones heredadas).
//! - **Arreglos**: `[i64 longitud][elem0][elem1]…` con 8 bytes por elemento;
//!   `new T[n]` reserva con `calloc` (ceros / null).
//! - **Closures**: `[fnptr][captura0][captura1]…`; cada lambda se eleva a una
//!   función `@hulk_lambda_N(i8* %__env, …)` y captura por valor.

use crate::{error::CompilerError, parser::expression::Program};

pub mod llvm;

/// Contrato de un backend de generación: recibe el AST verificado y produce
/// el módulo compilado como texto, o los diagnósticos que lo impidieron.
pub trait CodegenBackend {
    fn generate(&mut self, program: &Program) -> Result<String, Vec<CompilerError>>;
}
