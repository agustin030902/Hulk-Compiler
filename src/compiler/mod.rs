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

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub output_path: PathBuf,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("output"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    LlvmIr,
    Diagnostics,
}

#[derive(Debug)]
pub struct CompileReport {
    pub tokens: Vec<Token>,
    pub ast: Option<Program>,
    pub llvm_ir: Option<String>,
    pub output_path: Option<PathBuf>,
    pub output_kind: Option<OutputKind>,
    pub errors: Vec<CompilerError>,
}

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
    
        let program = match ast {
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
    
        let semantic_errors = self.semantic_analyzer.analyze(&program, source);
        if !semantic_errors.is_empty() {
            return self.finalize_diagnostics(tokens, Some(program), semantic_errors, options);
        }
    
        // =========================
        // CODEGEN (AQUÍ ES DONDE VA)
        // =========================
        match self.llvm_backend.generate(&program) {
            Ok(llvm_ir) => {
                // 👇 AQUÍ LO PONES (DEBUG)
                #[cfg(debug_assertions)]
                {
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