#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Number(String),
    String(String),
    Boolean(String),
    Let,
    Print,
    Assign,
    Add,
    Minus,
    Multiply,
    Divide,
    Comma,
    Semicolon,
    LeftParen,
    RightParen,
    Unknown,
    EOF,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub line: usize,
    pub column: usize,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn kind(&self) -> &TokenKind {
        &self.kind
    }
}
