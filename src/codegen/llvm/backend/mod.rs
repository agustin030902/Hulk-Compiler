//! Backend LLVM: estado del generador y orquestación de `generate`.
//!
//! El backend está organizado por responsabilidad:
//!
//! - [`emitter`] — buffers de texto y generadores de nombres frescos.
//! - [`scopes`] — pila de scopes y almacenamiento de variables (`alloca`).
//! - [`type_compat`] — subtipado, nulabilidad e igualdad estructural.
//! - [`functions`] — carga de la metadata semántica (firmas, layouts, mapas).
//! - [`layout`] — layout de structs en memoria.
//! - [`type_lowering`] — mapeo `SemanticType` → `ValueType`.
//! - [`runtime_globals`] — `@hulk_type_parents` / `@hulk_is_subtype`.
//! - [`emit`] — emisión del programa (un módulo por variante de expresión).

mod emit;
mod emitter;
mod functions;
mod layout;
mod runtime_globals;
mod scopes;
mod type_compat;
mod type_lowering;

use std::collections::HashMap;

use crate::{
    codegen::CodegenBackend,
    error::{CompilerError, ErrorCategory},
    parser::expression::{Program, TypeDecl},
};

use super::helper::state::{ValueType, VariableInfo};
use functions::FunctionInfo;
use layout::StructLayout;

#[derive(Debug, Default)]
pub struct LlvmBackend {
    pub(super) body_lines: Vec<String>,
    pub(super) function_lines: Vec<String>,
    pub(super) global_lines: Vec<String>,
    pub(super) errors: Vec<CompilerError>,
    pub(super) scopes: Vec<HashMap<String, VariableInfo>>,
    pub(super) functions: HashMap<String, FunctionInfo>,
    pub(super) type_ids: HashMap<String, u32>,
    pub(super) type_decls: HashMap<String, TypeDecl>,
    pub(super) struct_layouts: HashMap<u32, StructLayout>,
    pub(super) method_dispatch: HashMap<(u32, String), String>,
    // Jerarquía completa (tipos e interfaces, incluidas las splat sintetizadas)
    // extraída del TypeTable semántico; type_decls solo cubre tipos del AST.
    pub(super) type_parents: HashMap<u32, u32>,
    // TypeId semántico del arreglo → ValueType de sus elementos.
    pub(super) array_elems: HashMap<u32, ValueType>,
    // TypeId semántico de firma función → (params, retorno) para closures.
    pub(super) function_types: HashMap<u32, (Vec<ValueType>, ValueType)>,
    pub(super) lambda_counter: usize,
    pub(super) interface_real_types: HashMap<String, u32>,
    pub(super) param_real_types: HashMap<String, u32>,
    pub(super) temp_counter: usize,
    pub(super) label_counter: usize,
    pub(super) string_counter: usize,
    pub(crate) current_block: String,
    pub(super) current_type_id: Option<u32>,
    pub(super) current_method_name: Option<String>,
    pub(super) current_self_ref: Option<VariableInfo>,
}

impl LlvmBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub(super) fn reset(&mut self) {
        self.body_lines.clear();
        self.function_lines.clear();
        self.global_lines.clear();
        self.errors.clear();
        self.scopes.clear();
        self.functions.clear();
        self.type_ids.clear();
        self.type_decls.clear();
        self.struct_layouts.clear();
        self.method_dispatch.clear();
        self.type_parents.clear();
        self.array_elems.clear();
        self.function_types.clear();
        self.lambda_counter = 0;
        self.interface_real_types.clear();
        self.param_real_types.clear();
        self.temp_counter = 0;
        self.label_counter = 0;
        self.string_counter = 0;
        self.current_type_id = None;
        self.current_method_name = None;
        self.current_self_ref = None;
        self.push_scope();
    }

    pub(in crate::codegen::llvm) fn semantic_error(&mut self, message: impl Into<String>) {
        self.errors
            .push(CompilerError::new(ErrorCategory::Semantic, message, 1, 1));
    }
}

impl CodegenBackend for LlvmBackend {
    fn generate(
        &mut self,
        program: &Program,
        analyzer: &crate::semantic::SemanticAnalyzer,
    ) -> Result<String, Vec<CompilerError>> {
        self.reset();

        if !self.load_function_signatures(program, analyzer) {
            return Err(self.errors.clone());
        }

        self.emit_program(program);

        if self.errors.is_empty() {
            Ok(self.compose_module())
        } else {
            Err(self.errors.clone())
        }
    }
}
