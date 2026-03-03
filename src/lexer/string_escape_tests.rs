use super::{Lexer, TokenKind};

#[test]
fn lexes_escaped_double_quote_in_string_literal() {
    let source = r#"print("The message is \"Hello World\"");"#.to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::String("The message is \"Hello World\"".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}

#[test]
fn lexes_newline_and_tab_escape_sequences_in_string_literal() {
    let source = r#"print("Line1\nLine2\tDone");"#.to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::String("Line1\nLine2\tDone".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}
