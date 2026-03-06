#[cfg(test)]
mod string_escape_tests;
#[cfg(test)]
mod tests;
mod token;

use logos::Logos;

use crate::error::{CompilerError, ErrorCategory};

pub use token::Token;
pub use token::TokenKind;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
enum LogosTokenKind {
    #[token("let", priority = 3)]
    Let,
    #[token("print", priority = 3)]
    Print,
    #[token("PI", priority = 3)]
    Pi,
    #[token("E", priority = 3)]
    E,
    #[token("sin", priority = 3)]
    Sin,
    #[token("cos", priority = 3)]
    Cos,
    #[token("sqrt", priority = 3)]
    Sqrt,
    #[token("exp", priority = 3)]
    Exp,
    #[token("log", priority = 3)]
    Log,
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
    #[token("^")]
    Power,
    #[token("@")]
    Concat,
    #[token("-")]
    Minus,
    #[token("*")]
    Multiply,
    #[token("/")]
    Divide,
    #[token("==")]
    EqualEqual,
    #[token("!=")]
    NotEqual,
    #[token("<=")]
    LessEqual,
    #[token(">=")]
    GreaterEqual,
    #[token("<")]
    Less,
    #[token(">")]
    Greater,
    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("!")]
    Not,
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
            LogosTokenKind::Pi => (TokenKind::Pi, lexeme.to_string()),
            LogosTokenKind::E => (TokenKind::E, lexeme.to_string()),
            LogosTokenKind::Sin => (TokenKind::Sin, lexeme.to_string()),
            LogosTokenKind::Cos => (TokenKind::Cos, lexeme.to_string()),
            LogosTokenKind::Sqrt => (TokenKind::Sqrt, lexeme.to_string()),
            LogosTokenKind::Exp => (TokenKind::Exp, lexeme.to_string()),
            LogosTokenKind::Log => (TokenKind::Log, lexeme.to_string()),
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
            LogosTokenKind::Power => (TokenKind::Power, lexeme.to_string()),
            LogosTokenKind::Concat => (TokenKind::Concat, lexeme.to_string()),
            LogosTokenKind::Minus => (TokenKind::Minus, lexeme.to_string()),
            LogosTokenKind::Multiply => (TokenKind::Multiply, lexeme.to_string()),
            LogosTokenKind::Divide => (TokenKind::Divide, lexeme.to_string()),
            LogosTokenKind::EqualEqual => (TokenKind::EqualEqual, lexeme.to_string()),
            LogosTokenKind::NotEqual => (TokenKind::NotEqual, lexeme.to_string()),
            LogosTokenKind::Less => (TokenKind::Less, lexeme.to_string()),
            LogosTokenKind::Greater => (TokenKind::Greater, lexeme.to_string()),
            LogosTokenKind::LessEqual => (TokenKind::LessEqual, lexeme.to_string()),
            LogosTokenKind::GreaterEqual => (TokenKind::GreaterEqual, lexeme.to_string()),
            LogosTokenKind::And => (TokenKind::And, lexeme.to_string()),
            LogosTokenKind::Or => (TokenKind::Or, lexeme.to_string()),
            LogosTokenKind::Not => (TokenKind::Not, lexeme.to_string()),
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
        let mut logos_lexer = LogosTokenKind::lexer(input);
        let mut tokens = Vec::new();
        self.errors.clear();
        let mut line = 1usize;
        let mut column = 1usize;
        let mut cursor = 0usize;

        while let Some(next) = logos_lexer.next() {
            let span = logos_lexer.span();
            let lexeme = &input[span.clone()];
            advance_position(input, cursor, span.start, &mut line, &mut column);
            let token_line = line;
            let token_column = column;

            match next {
                Ok(kind) => {
                    tokens.push(kind.into_token(
                        lexeme,
                        token_line,
                        token_column,
                        span.start,
                        span.end,
                    ))
                }
                Err(_) => {
                    self.errors.push(CompilerError::new(
                        ErrorCategory::Lexical,
                        format!("Unexpected character sequence: {}", lexeme),
                        token_line,
                        token_column,
                    ));
                    tokens.push(Token {
                        kind: TokenKind::Unknown,
                        value: lexeme.to_string(),
                        line: token_line,
                        column: token_column,
                        start: span.start,
                        end: span.end,
                    });
                }
            }

            advance_position(input, span.start, span.end, &mut line, &mut column);
            cursor = span.end;
        }

        advance_position(input, cursor, input.len(), &mut line, &mut column);
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

fn advance_position(input: &str, from: usize, to: usize, line: &mut usize, column: &mut usize) {
    for byte in &input.as_bytes()[from..to] {
        if *byte == b'\n' {
            *line += 1;
            *column = 1;
        } else {
            *column += 1;
        }
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
