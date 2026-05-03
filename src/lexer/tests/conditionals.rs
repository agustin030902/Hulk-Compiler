use crate::lexer::{Lexer, TokenKind};

#[test]
fn lexes_conditional_expression_tokens() {
    let source =
        r#"if (x > 0) print("positive") elif (x == 0) print("zero") else print("negative");"#
            .to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::If,
            TokenKind::LeftParen,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Greater,
            TokenKind::Number("0".to_string()),
            TokenKind::RightParen,
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::String("positive".to_string()),
            TokenKind::RightParen,
            TokenKind::Elif,
            TokenKind::LeftParen,
            TokenKind::Identifier("x".to_string()),
            TokenKind::EqualEqual,
            TokenKind::Number("0".to_string()),
            TokenKind::RightParen,
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::String("zero".to_string()),
            TokenKind::RightParen,
            TokenKind::Else,
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::String("negative".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}
