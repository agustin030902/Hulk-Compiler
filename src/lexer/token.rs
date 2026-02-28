    #[derive(Clone, Debug, PartialEq)]
    pub enum TokenKind {
        Identifier(String),
        Number(String),
        String(String),
        Operator(String),
        Keyword(String),
        Boolean(String),
        Print,
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
        pub line: i32,
        pub column: i32,
    }

    impl Token {
        pub fn kind(&self) -> &TokenKind {
            &self.kind
        }
    }