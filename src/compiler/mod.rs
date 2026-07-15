//! # Orquestación del pipeline
//!
//! [`Compiler`] encadena las cuatro fases con política **fail-fast**:
//!
//! ```text
//! lex ─▶ parse ─▶ expandir macros ─▶ análisis semántico ─▶ codegen LLVM
//! ```
//!
//! En cuanto una fase produce errores, se interrumpe el flujo y se escribe un
//! reporte de diagnóstico en `output_path` en lugar del IR. El resultado
//! completo (tokens, AST, IR, errores) queda disponible en [`CompileReport`]
//! para consumidores como la GUI, que muestran cada artefacto por separado.

use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(test)]
mod tests;

use crate::{
    codegen::{CodegenBackend, llvm::LlvmBackend},
    error::{CompilerError, ErrorCategory},
    lexer::{Lexer, Token},
    parser::{Parser, Program},
    semantic::SemanticAnalyzer,
};

/// Opciones de compilación.
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Ruta donde se escribe el LLVM IR generado (o el reporte de
    /// diagnóstico si la compilación falla). Por defecto: `output`.
    pub output_path: PathBuf,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("output"),
        }
    }
}

/// Qué terminó escrito en `output_path`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    /// Compilación exitosa: el archivo contiene el módulo LLVM IR.
    LlvmIr,
    /// Hubo errores: el archivo contiene el reporte de diagnóstico.
    Diagnostics,
}

/// Resultado completo de una compilación: expone los artefactos de cada fase
/// aunque el pipeline se haya interrumpido, para que los consumidores (CLI,
/// GUI) puedan inspeccionarlos por separado.
#[derive(Debug)]
pub struct CompileReport {
    /// Tokens producidos por el lexer (incluye `Unknown` de recuperación).
    pub tokens: Vec<Token>,
    /// AST tras el parseo y la expansión de macros, si el parseo tuvo éxito.
    pub ast: Option<Program>,
    /// Módulo LLVM IR como texto, solo si toda la compilación tuvo éxito.
    pub llvm_ir: Option<String>,
    /// Ruta donde quedó escrito el resultado (IR o diagnóstico).
    pub output_path: Option<PathBuf>,
    /// Naturaleza de lo escrito en `output_path`.
    pub output_kind: Option<OutputKind>,
    /// Diagnósticos acumulados; vacío si la compilación fue exitosa.
    pub errors: Vec<CompilerError>,
}

/// Orquestador del pipeline de compilación (ver [documentación del
/// módulo](self)).
#[derive(Debug, Default)]
pub struct Compiler {
    semantic_analyzer: SemanticAnalyzer,
    llvm_backend: LlvmBackend,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            semantic_analyzer: SemanticAnalyzer::new(),
            llvm_backend: LlvmBackend::new(),
        }
    }

    /// Compila `source` de principio a fin y devuelve el [`CompileReport`]
    /// con los artefactos de todas las fases alcanzadas.
    pub fn compile(&mut self, source: &str, options: &CompileOptions) -> CompileReport {
        let mut lexer = Lexer::new(source.to_string());
        let tokens = lexer.lex();
    
        let lexer_errors = lexer.errors().to_vec();
        if !lexer_errors.is_empty() {
            return self.finalize_diagnostics(tokens, None, lexer_errors, options);
        }
    
        let mut parser = Parser::new(source);
        let ast = parser.parse_program(tokens.clone());
    
        let parser_errors = parser.errors().to_vec();
        if !parser_errors.is_empty() {
            return self.finalize_diagnostics(tokens, ast, parser_errors, options);
        }
    
        let mut program = match ast {
            Some(p) => p,
            None => {
                return self.finalize_diagnostics(
                    tokens,
                    None,
                    vec![CompilerError::new(
                        ErrorCategory::Syntax,
                        "Program could not be built after parsing.",
                        1,
                        1,
                    )],
                    options,
                );
            }
        };
    
        // Expansión de macros `define`: sustitución call-by-name a nivel de
        // AST, de modo que semántica y codegen solo ven HULK plano.
        let macro_errors = crate::parser::MacroExpander::expand_program(&mut program);
        if !macro_errors.is_empty() {
            return self.finalize_diagnostics(tokens, Some(program), macro_errors, options);
        }

        let semantic_errors = self.semantic_analyzer.analyze(&program, source);
        if !semantic_errors.is_empty() {
            return self.finalize_diagnostics(tokens, Some(program), semantic_errors, options);
        }
    
        // =========================
        // CODEGEN (AQUÍ ES DONDE VA)
        // =========================
        match self.llvm_backend.generate(&program) {
            Ok(llvm_ir) => {
                // Volcado del IR opt-in: activar con la env var HULK_DUMP_IR.
                // Antes se imprimía siempre en builds debug, contaminando la salida
                // de cada `cargo test` que compila end-to-end.
                if std::env::var_os("HULK_DUMP_IR").is_some() {
                    eprintln!("================ LLVM IR ================\n{}", llvm_ir);
                }

                self.finalize_ir(tokens, program, llvm_ir, options)
            }
    
            Err(codegen_errors) => {
                self.finalize_diagnostics(tokens, Some(program), codegen_errors, options)
            }
        }
    }

    fn emit_output(
        &self,
        path: &Path,
        contents: &str,
    ) -> Result<PathBuf, CompilerError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| {
                    CompilerError::new(
                        ErrorCategory::Semantic,
                        format!("Failed to create output dir: {}", e),
                        1,
                        1,
                    )
                })?;
            }
        }

        fs::write(path, contents).map_err(|e| {
            CompilerError::new(
                ErrorCategory::Semantic,
                format!("Failed to write output file '{}': {}", path.display(), e),
                1,
                1,
            )
        })?;

        Ok(path.to_path_buf())
    }

    fn finalize_ir(
        &self,
        tokens: Vec<Token>,
        program: Program,
        llvm_ir: String,
        options: &CompileOptions,
    ) -> CompileReport {
        let mut errors = Vec::new();

        let output_path = match self.emit_output(&options.output_path, &llvm_ir) {
            Ok(path) => Some(path),
            Err(e) => {
                errors.push(e);
                None
            }
        };

        CompileReport {
            tokens,
            ast: Some(program),
            llvm_ir: Some(llvm_ir),
            output_path,
            output_kind: if errors.is_empty() {
                Some(OutputKind::LlvmIr)
            } else {
                Some(OutputKind::Diagnostics)
            },
            errors,
        }
    }

    fn finalize_diagnostics(
        &self,
        tokens: Vec<Token>,
        ast: Option<Program>,
        mut errors: Vec<CompilerError>,
        options: &CompileOptions,
    ) -> CompileReport {
        let diagnostics = format_diagnostics_report(&errors);

        let output_path = match self.emit_output(&options.output_path, &diagnostics) {
            Ok(path) => Some(path),
            Err(e) => {
                errors.push(e);
                None
            }
        };

        CompileReport {
            tokens,
            ast,
            llvm_ir: None,
            output_path,
            output_kind: Some(OutputKind::Diagnostics),
            errors,
        }
    }
}

fn format_diagnostics_report(errors: &[CompilerError]) -> String {
    let mut report = String::from("Hulk Compiler Diagnostics\n");
    report.push_str("========================\n");

    if errors.is_empty() {
        report.push_str("No errors.\n");
        return report;
    }

    for (index, error) in errors.iter().enumerate() {
        report.push_str(&format!(
            "{}. [{:?}] [{}] line {}, column {}: {}\n",
            index + 1,
            error.category,
            phase_for_category(&error.category),
            error.line,
            error.column,
            error.message
        ));
    }

    report
}

fn phase_for_category(category: &ErrorCategory) -> &'static str {
    match category {
        ErrorCategory::Lexical => "Lexer",
        ErrorCategory::Syntax => "Parser",
        ErrorCategory::Type | ErrorCategory::Semantic => "Semantic",
    }
}