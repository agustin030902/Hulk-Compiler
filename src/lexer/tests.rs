use super::{Lexer, TokenKind};

#[test]
fn lexes_concat_operator_between_string_and_number() {
    let source = r#"print("The meaning of life is " @ 42);"#.to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::String("The meaning of life is ".to_string()),
            TokenKind::Concat,
            TokenKind::Number("42".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}

#[test]
fn lexes_concat_operator_between_strings() {
    let source = r#"let message = "Hello, " @ "World";"#.to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Let,
            TokenKind::Identifier("message".to_string()),
            TokenKind::Assign,
            TokenKind::String("Hello, ".to_string()),
            TokenKind::Concat,
            TokenKind::String("World".to_string()),
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}
