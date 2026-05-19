use crate::lexer::{Lexer, TokenKind};

#[test]
fn lexes_type_declaration_and_member_access_tokens() {
    let source = r#"
type Point(x: Number, y: Number) {
    x = x;
    y = y;
    norm() => sqrt(self.x ^ 2 + self.y ^ 2);
}
"#
    .to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Type,
            TokenKind::Identifier("Point".to_string()),
            TokenKind::LeftParen,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Colon,
            TokenKind::Identifier("Number".to_string()),
            TokenKind::Comma,
            TokenKind::Identifier("y".to_string()),
            TokenKind::Colon,
            TokenKind::Identifier("Number".to_string()),
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Assign,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Semicolon,
            TokenKind::Identifier("y".to_string()),
            TokenKind::Assign,
            TokenKind::Identifier("y".to_string()),
            TokenKind::Semicolon,
            TokenKind::Identifier("norm".to_string()),
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::Arrow,
            TokenKind::Sqrt,
            TokenKind::LeftParen,
            TokenKind::Identifier("self".to_string()),
            TokenKind::Dot,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Power,
            TokenKind::Number("2".to_string()),
            TokenKind::Add,
            TokenKind::Identifier("self".to_string()),
            TokenKind::Dot,
            TokenKind::Identifier("y".to_string()),
            TokenKind::Power,
            TokenKind::Number("2".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::RightBrace,
            TokenKind::EOF,
        ]
    );
}

#[test]
fn lexes_new_expression_and_method_call_tokens() {
    let source = "let p = new Point(1, 2); let q = p.add(p);".to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Let,
            TokenKind::Identifier("p".to_string()),
            TokenKind::Assign,
            TokenKind::New,
            TokenKind::Identifier("Point".to_string()),
            TokenKind::LeftParen,
            TokenKind::Number("1".to_string()),
            TokenKind::Comma,
            TokenKind::Number("2".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::Let,
            TokenKind::Identifier("q".to_string()),
            TokenKind::Assign,
            TokenKind::Identifier("p".to_string()),
            TokenKind::Dot,
            TokenKind::Identifier("add".to_string()),
            TokenKind::LeftParen,
            TokenKind::Identifier("p".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}
