//! # Diagnóstico unificado
//!
//! Todas las fases reportan errores con el mismo tipo, [`CompilerError`], que
//! conserva la [categoría](ErrorCategory), el mensaje y la ubicación. La
//! categoría determina tanto el prefijo del mensaje en `stderr`
//! (`LEXICAL`/`SYNTACTIC`/`SEMANTIC`) como el [código de
//! salida](CompilerError::exit_code) de la CLI.

use std::fmt;

/// Fase del compilador a la que pertenece un error.
///
/// `Type` y `Semantic` se distinguen internamente, pero de cara al usuario
/// ambas se reportan como `SEMANTIC` y comparten exit code `3`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Error de tokenización (exit code `1`).
    Lexical,
    /// Error del parser LR (exit code `2`).
    Syntax,
    /// Incompatibilidad de tipos (exit code `3`).
    Type,
    /// Otros errores semánticos: símbolos, aridad, herencia… (exit code `3`).
    Semantic,
}

/// Un diagnóstico del compilador con ubicación en el fuente.
///
/// Su implementación de [`fmt::Display`] produce el formato del contrato de
/// la CLI: `(línea,columna) CATEGORÍA: mensaje`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerError {
    pub category: ErrorCategory,
    pub message: String,
    /// Línea 1-indexada dentro del fuente.
    pub line: usize,
    /// Columna 1-indexada dentro de la línea.
    pub column: usize,
}

impl CompilerError {
    pub fn new(
        category: ErrorCategory,
        message: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            category,
            message: message.into(),
            line,
            column,
        }
    }

    /// Código de salida de la CLI para esta categoría de error:
    /// léxico → `1`, sintáctico → `2`, semántico/tipos → `3`.
    pub fn exit_code(&self) -> i32 {
        match self.category {
            ErrorCategory::Lexical => 1,
            ErrorCategory::Syntax => 2,
            ErrorCategory::Type | ErrorCategory::Semantic => 3,
        }
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let error_type = match self.category {
            ErrorCategory::Lexical => "LEXICAL",
            ErrorCategory::Syntax => "SYNTACTIC",
            ErrorCategory::Type | ErrorCategory::Semantic => "SEMANTIC",
        };

        write!(
            f,
            "({},{}) {}: {}",
            self.line,
            self.column,
            error_type,
            self.message
        )
    }
}

impl std::error::Error for CompilerError {}

/// Traduce un offset de bytes del fuente a coordenadas (línea, columna)
/// 1-indexadas; lo usan el parser y los checkers para ubicar diagnósticos.
pub fn offset_to_line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;

    for (idx, ch) in source.char_indices() {
        if idx >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}