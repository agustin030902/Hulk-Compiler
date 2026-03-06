#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Number(String),
    String(String),
    Boolean(String),
    Let,
    Print,
    Pi,
    E,
    Sin,
    Cos,
    Sqrt,
    Exp,
    Log,
    Assign,
    Add,
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
