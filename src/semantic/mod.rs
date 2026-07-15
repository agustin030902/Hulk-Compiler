//! # Análisis semántico
//!
//! Verificación de tipos y símbolos en **dos pasadas** orquestadas por
//! [`SemanticAnalyzer`]:
//!
//! 1. **Inferencia de firmas** (`SignatureInferencePass`): fixpoint iterativo
//!    (hasta 8 pasadas) que propaga tipos entre funciones mutuamente
//!    recursivas antes de verificar cuerpos.
//! 2. **Recolección + verificación**: el `SymbolCollector` registra tipos,
//!    interfaces, funciones y métodos (detectando redeclaraciones y ciclos de
//!    herencia) y el `TypeChecker` verifica cada expresión con un módulo por
//!    variante del AST.
//!
//! Piezas destacadas del sistema de tipos:
//!
//! - **Interfaces estructurales** con varianza: covariante en retornos,
//!   contravariante en parámetros.
//! - **Iteración dual**: `Iterable` (el tipo es su propio iterador, con
//!   `next()/current()`) y `Enumerable` (fábrica con `iter()`), análogo a
//!   `Iterator`/`IntoIterator` de Rust.
//! - **Splat notation** `T*`: sintetiza una interfaz `Iterable_T` que refina
//!   `current(): T` por cada tipo base usado.
//! - **Tipos internados estructuralmente**: los arreglos (`TypeInfo::Array`) y
//!   las firmas de función comparten `TypeId` cuando tienen la misma forma,
//!   así que la igualdad de `Number[]` o `(Number) -> Number` es estructural.
//! - **Nulabilidad selectiva**: `Null` solo es asignable a tipos-puntero
//!   (`String`, structs, funciones, arreglos), nunca a `Number`/`Boolean`.

mod analyzer;
pub mod builtins;
mod helper;
pub(in crate::semantic) mod pipeline;
#[cfg(test)]
mod tests;

pub use analyzer::SemanticAnalyzer;
pub use helper::{SemanticType, TypeId, TypeInfo};
