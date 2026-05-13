use crate::lexer::{Lexer, TokenKind};

#[test]
fn lexes_typed_let_in_binding_tokens() {
    let source = r#"let x: Number = 42 in print(x);"#.to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Let,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Colon,
            TokenKind::Identifier("Number".to_string()),
            TokenKind::Assign,
            TokenKind::Number("42".to_string()),
            TokenKind::In,
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::Identifier("x".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}

#[test]
fn keeps_destructive_assign_as_single_token_when_colon_exists() {
    let source = r#"x := x + 1;"#.to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Identifier("x".to_string()),
            TokenKind::DestructiveAssign,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Add,
            TokenKind::Number("1".to_string()),
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}
