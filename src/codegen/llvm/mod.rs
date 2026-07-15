//! Backend LLVM: [`LlvmBackend`] implementa
//! [`CodegenBackend`](crate::codegen::CodegenBackend) emitiendo el módulo IR
//! como texto en tres buffers (globals, funciones, cuerpo de `main`) que se
//! ensamblan al final junto al preámbulo de declares (`printf`, `malloc`,
//! `calloc`, funciones matemáticas de libm…).

mod backend;
mod helper;
#[cfg(test)]
mod tests;

pub use backend::LlvmBackend;
