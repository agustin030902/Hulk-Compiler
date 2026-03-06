use lalrpop_util::{ParseError, lalrpop_mod};

use crate::{
    error::{CompilerError, ErrorCategory, offset_to_line_column},
    lexer::{Token, TokenKind},
};

pub mod expression;
#[cfg(test)]
mod string_escape_tests;
#[cfg(test)]
mod tests;

pub use expression::Program;

lalrpop_mod!(
    #[allow(clippy::all)]
    pub grammar,
    "/parser/grammar.rs"
);

#[derive(Debug)]
pub struct Parser<'input> {
    source: &'input str,
    errors: Vec<CompilerError>,
}

impl<'input> Parser<'input> {
    pub fn new(source: &'input str) -> Self {
        Self {
            source,
            errors: Vec::new(),
        }
    }

    pub fn parse_program(&mut self, tokens: Vec<Token>) -> Option<Program> {
        self.errors.clear();

        let spanned = tokens.into_iter().map(|token| {
            let kind = normalize_token_kind(token.kind);
            Ok((token.start, kind, token.end))
        });

        match grammar::ProgramParser::new().parse(spanned) {
            Ok(program) => Some(program),
            Err(err) => {
                self.errors.push(parse_error_to_compiler(err, self.source));
                None
            }
        }
    }

    pub fn errors(&self) -> &[CompilerError] {
        &self.errors
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

fn parse_error_to_compiler(
    error: ParseError<usize, TokenKind, String>,
    source: &str,
) -> CompilerError {
    match error {
        ParseError::InvalidToken { location } => {
            let (line, column) = offset_to_line_column(source, location);
            CompilerError::new(ErrorCategory::Syntax, "Invalid token", line, column)
        }
        ParseError::UnrecognizedEof { location, expected } => {
            let (line, column) = offset_to_line_column(source, location);
            let expected_msg = format_expected(&expected);
            CompilerError::new(
                ErrorCategory::Syntax,
                format!("Unexpected end of input. Expected one of: {}", expected_msg),
                line,
                column,
            )
        }
        ParseError::UnrecognizedToken { token, expected } => {
            let (start, token_kind, _) = token;
            let (line, column) = offset_to_line_column(source, start);
            let expected_msg = format_expected(&expected);
            CompilerError::new(
                ErrorCategory::Syntax,
                format!(
                    "Unexpected token {}. Expected one of: {}",
                    token_label(&token_kind),
                    expected_msg
                ),
                line,
                column,
            )
        }
        ParseError::ExtraToken { token } => {
            let (start, token_kind, _) = token;
            let (line, column) = offset_to_line_column(source, start);
            CompilerError::new(
                ErrorCategory::Syntax,
                format!("Extra token {}", token_label(&token_kind)),
                line,
                column,
            )
        }
        ParseError::User { error } => CompilerError::new(ErrorCategory::Syntax, error, 1, 1),
    }
}

fn format_expected(expected: &[String]) -> String {
    if expected.is_empty() {
        return "no alternatives".to_string();
    }

    expected.join(", ")
}

fn token_label(token: &TokenKind) -> String {
    match token {
        TokenKind::Identifier(v) => format!("identifier({})", v),
        TokenKind::Number(v) => format!("number({})", v),
        TokenKind::String(v) => format!("string({})", v),
        TokenKind::Boolean(v) => format!("boolean({})", v),
        TokenKind::Let => "let".to_string(),
        TokenKind::Print => "print".to_string(),
        TokenKind::Pi => "PI".to_string(),
        TokenKind::E => "E".to_string(),
        TokenKind::Sin => "sin".to_string(),
        TokenKind::Cos => "cos".to_string(),
        TokenKind::Sqrt => "sqrt".to_string(),
        TokenKind::Exp => "exp".to_string(),
        TokenKind::Log => "log".to_string(),
        TokenKind::Assign => "=".to_string(),
        TokenKind::Add => "+".to_string(),
        TokenKind::Concat => "@".to_string(),
        TokenKind::Minus => "-".to_string(),
        TokenKind::Multiply => "*".to_string(),
        TokenKind::Divide => "/".to_string(),
        TokenKind::EqualEqual => "==".to_string(),
        TokenKind::NotEqual => "!=".to_string(),
        TokenKind::Less => "<".to_string(),
        TokenKind::Greater => ">".to_string(),
        TokenKind::LessEqual => "<=".to_string(),
        TokenKind::GreaterEqual => ">=".to_string(),
        TokenKind::And => "&&".to_string(),
        TokenKind::Or => "||".to_string(),
        TokenKind::Not => "!".to_string(),
        TokenKind::Comma => ",".to_string(),
        TokenKind::Semicolon => ";".to_string(),
        TokenKind::LeftParen => "(".to_string(),
        TokenKind::RightParen => ")".to_string(),
        TokenKind::Unknown => "unknown".to_string(),
        TokenKind::EOF => "EOF".to_string(),
    }
}

fn normalize_token_kind(kind: TokenKind) -> TokenKind {
    match kind {
        TokenKind::String(raw) => TokenKind::String(unescape_string_contents(&raw)),
        other => other,
    }
}

fn unescape_string_contents(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        match chars.next() {
            Some('"') => result.push('"'),
            Some('n') => result.push('\n'),
            Some('t') => result.push('\t'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }

    result
}
