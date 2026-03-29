#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Number(String),
    String(String),
    Boolean(String),
    Let,
    While,
    Print,
    Pi,
    E,
    Sin,
    Cos,
    Sqrt,
    Exp,
    Log,
    Rand,
    In,
    Assign,
    Add,
    Power,
    Concat,
    Minus,
    Multiply,
    Divide,
    EqualEqual,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    Not,
    DestructiveAssign,
    Comma,
    Semicolon,
    LeftBrace,
    RightBrace,
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
