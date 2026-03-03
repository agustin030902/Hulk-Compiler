use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexical,
    Syntax,
    Type,
    Semantic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerError {
    pub category: ErrorCategory,
    pub message: String,
    pub line: usize,
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
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} error at {}:{}: {}",
            self.category, self.line, self.column, self.message
        )
    }
}

impl std::error::Error for CompilerError {}

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
