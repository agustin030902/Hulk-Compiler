mod token;
#[cfg(test)]
mod string_escape_tests;
#[cfg(test)]
mod tests;

use logos::Logos;

use crate::error::{CompilerError, ErrorCategory};

pub use token::Token;
pub use token::TokenKind;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
enum LogosTokenKind {
    #[token("let")]
    Let,
    #[token("print")]
    Print,
    #[regex(r"true|false")]
    Boolean,
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
    #[regex(r"[0-9]+(?:\.[0-9]+)?")]
    Number,
    #[regex(r#""([^"\\]|\\.)*""#)]
    String,
    #[token("+")]
    Add,
    #[token("@")]
    Concat,
    #[token("-")]
    Minus,
    #[token("*")]
    Multiply,
    #[token("/")]
    Divide,
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token("=")]
    Assign,
}

impl LogosTokenKind {
    fn into_token(
        self,
        lexeme: &str,
        line: usize,
        column: usize,
        start: usize,
        end: usize,
    ) -> Token {
        let (kind, value) = match self {
            LogosTokenKind::Let => (TokenKind::Let, lexeme.to_string()),
            LogosTokenKind::Print => (TokenKind::Print, lexeme.to_string()),
            LogosTokenKind::Boolean => {
                let value = lexeme.to_string();
                (TokenKind::Boolean(value.clone()), value)
            }
            LogosTokenKind::Identifier => {
                let value = lexeme.to_string();
                (TokenKind::Identifier(value.clone()), value)
            }
            LogosTokenKind::Number => {
                let value = lexeme.to_string();
                (TokenKind::Number(value.clone()), value)
            }
            LogosTokenKind::String => {
                let raw_value = lexeme
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(lexeme);
                let value = unescape_string_contents(raw_value);
                (TokenKind::String(value.clone()), value)
            }
            LogosTokenKind::Add => (TokenKind::Add, lexeme.to_string()),
            LogosTokenKind::Concat => (TokenKind::Concat, lexeme.to_string()),
            LogosTokenKind::Minus => (TokenKind::Minus, lexeme.to_string()),
            LogosTokenKind::Multiply => (TokenKind::Multiply, lexeme.to_string()),
            LogosTokenKind::Divide => (TokenKind::Divide, lexeme.to_string()),
            LogosTokenKind::LeftParen => (TokenKind::LeftParen, lexeme.to_string()),
            LogosTokenKind::RightParen => (TokenKind::RightParen, lexeme.to_string()),
            LogosTokenKind::Comma => (TokenKind::Comma, lexeme.to_string()),
            LogosTokenKind::Semicolon => (TokenKind::Semicolon, lexeme.to_string()),
            LogosTokenKind::Assign => (TokenKind::Assign, lexeme.to_string()),
        };

        Token {
            kind,
            value,
            line,
            column,
            start,
            end,
        }
    }
}

pub struct Lexer {
    input: String,
    errors: Vec<CompilerError>,
}

impl Lexer {
    pub fn new(input: String) -> Self {
        Self {
            input,
            errors: Vec::new(),
        }
    }

    pub fn lex(&mut self) -> Vec<Token> {
        let input = self.input.as_str();
        let line_starts = compute_line_starts(input);
        let mut logos_lexer = LogosTokenKind::lexer(input);
        let mut tokens = Vec::new();
        self.errors.clear();

        while let Some(next) = logos_lexer.next() {
            let span = logos_lexer.span();
            let lexeme = &input[span.clone()];
            let (line, column) = line_column_at(span.start, &line_starts);

            match next {
                Ok(kind) => {
                    tokens.push(kind.into_token(lexeme, line, column, span.start, span.end))
                }
                Err(_) => {
                    self.errors.push(CompilerError::new(
                        ErrorCategory::Lexical,
                        format!("Unexpected character sequence: {}", lexeme),
                        line,
                        column,
                    ));
                    tokens.push(Token {
                        kind: TokenKind::Unknown,
                        value: lexeme.to_string(),
                        line,
                        column,
                        start: span.start,
                        end: span.end,
                    });
                }
            }
        }

        let (line, column) = line_column_at(input.len(), &line_starts);
        tokens.push(Token {
            kind: TokenKind::EOF,
            value: String::new(),
            line,
            column,
            start: input.len(),
            end: input.len(),
        });

        tokens
    }

    pub fn errors(&self) -> &[CompilerError] {
        &self.errors
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

fn compute_line_starts(input: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, ch) in input.char_indices() {
        if ch == '\n' {
            starts.push(index + ch.len_utf8());
        }
    }
    starts
}

fn line_column_at(offset: usize, line_starts: &[usize]) -> (usize, usize) {
    let line_index = line_starts
        .partition_point(|&line_start| line_start <= offset)
        .saturating_sub(1);
    let line_start = line_starts[line_index];
    let column = offset.saturating_sub(line_start);
    (line_index + 1, column + 1)
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
