//! Recolección de símbolos: primera pasada del análisis semántico.
//!
//! `SymbolCollector` es una fachada sin estado; cada responsabilidad vive en
//! su propio submódulo:
//!
//! - [`type_collector`] — registro de tipos, padres y parámetros de constructor.
//! - [`interface_collector`] — registro de interfaces y su herencia.
//! - [`signature_collector`] — firmas de funciones globales, métodos y métodos
//!   de interfaz (comparten el constructor de firmas).
//! - [`hierarchy`] — utilidades de jerarquía (ciclos, búsqueda en padres).
//! - [`splat_injector`] — síntesis de interfaces `Iterable_T` para `T*`.
//!
//! # Orden de las fases
//!
//! El orden en que `analyzer.rs` invoca estas pasadas **importa** y debe
//! preservarse:
//!
//! 1. `collect_types` — los nombres de tipo deben existir antes de resolver
//!    padres o anotaciones que los mencionen.
//! 2. `collect_interfaces` — pueden ser padres de splat y receptores de métodos.
//! 3. `inject_splat_interfaces` — debe correr antes de resolver anotaciones
//!    `T*` (crea los `Iterable_T` que esas anotaciones nombran).
//! 4. `collect_functions` / `collect_methods` / `collect_interface_methods` —
//!    resuelven anotaciones, así que necesitan todos los tipos ya registrados.

mod hierarchy;
mod interface_collector;
mod signature_collector;
mod splat_injector;
mod type_collector;

use super::super::helper::TypeId;

pub(in crate::semantic) struct SymbolCollector;

impl SymbolCollector {
    /// Clave canónica de un método en las tablas de símbolos/firmas:
    /// `type#<id>::<nombre>`. La usan también el type checker y la
    /// inferencia de firmas para localizar métodos.
    pub(in crate::semantic) fn method_symbol_key(receiver: TypeId, method_name: &str) -> String {
        format!("type#{}::{}", receiver.0, method_name)
    }
}
