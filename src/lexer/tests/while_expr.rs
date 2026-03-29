use crate::lexer::{Lexer, TokenKind};

#[test]
fn lexes_while_expression_tokens() {
    let source = "while (x < 3) { print(x); x = x + 1; }".to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::While,
            TokenKind::LeftParen,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Less,
            TokenKind::Number("3".to_string()),
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::Identifier("x".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Assign,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Add,
            TokenKind::Number("1".to_string()),
            TokenKind::Semicolon,
            TokenKind::RightBrace,
            TokenKind::EOF,
        ]
    );
}
