//! Núcleo del compilador HULK.
//!
//! Expone las fases del pipeline (`lexer → parser → semantic → codegen`) y el
//! orquestador (`compiler`) como una librería compartida por los binarios
//! `Hulk-Compiler` (CLI) y `gui`. Centralizar los módulos aquí evita que cada
//! binario recompile el compilador con `#[path]`.

pub mod codegen;
pub mod compiler;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod semantic;