use crate::lexer::{Lexer, TokenKind};

#[test]
fn lexes_let_in_single_binding() {
    let source = "let x = 7 in x;";
    let mut lexer = Lexer::new(source.to_string());

    let tokens: Vec<TokenKind> = lexer.lex().into_iter().map(|t| t.kind).collect();
    assert!(!lexer.has_errors());
    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Assign,
            TokenKind::Number("7".to_string()),
            TokenKind::In,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}

#[test]
fn lexes_let_in_multiple_bindings() {
    let source = "let a = 9, b = 5, c = true in print(a+b);";
    let mut lexer = Lexer::new(source.to_string());

    let tokens: Vec<TokenKind> = lexer.lex().into_iter().map(|t| t.kind).collect();
    assert!(!lexer.has_errors());
    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Identifier("a".to_string()),
            TokenKind::Assign,
            TokenKind::Number("9".to_string()),
            TokenKind::Comma,
            TokenKind::Identifier("b".to_string()),
            TokenKind::Assign,
            TokenKind::Number("5".to_string()),
            TokenKind::Comma,
            TokenKind::Identifier("c".to_string()),
            TokenKind::Assign,
            TokenKind::Boolean("true".to_string()),
            TokenKind::In,
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::Identifier("a".to_string()),
            TokenKind::Add,
            TokenKind::Identifier("b".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}
