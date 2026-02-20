#[derive(Debug, Clone)]
pub struct Error {
    category: ErrorCategory,
    message: String,
    line: i32,
    column: i32,
}

pub enum ErrorCategory {
    SyntaxError,
    TypeError,
    SemanticError,
}