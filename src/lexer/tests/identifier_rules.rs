use crate::lexer::{Lexer, TokenKind};

#[test]
fn accepts_identifiers_starting_with_letter_and_underscores_inside() {
    let source = "lowercase TitleCase snake_case camelCase x0 x_0".to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Identifier("lowercase".to_string()),
            TokenKind::Identifier("TitleCase".to_string()),
            TokenKind::Identifier("snake_case".to_string()),
            TokenKind::Identifier("camelCase".to_string()),
            TokenKind::Identifier("x0".to_string()),
            TokenKind::Identifier("x_0".to_string()),
            TokenKind::EOF
        ]
    );
}

#[test]
fn rejects_identifiers_starting_with_underscore_or_digit() {
    let source = "_x 8ball".to_string();
    let mut lexer = Lexer::new(source);

    let _tokens = lexer.lex();
    assert!(
        lexer.has_errors(),
        "should report lexical errors for invalid identifiers"
    );
    let errors = lexer.errors();
    assert_eq!(
        errors.len(),
        2,
        "expected two lexical errors, got {:?}",
        errors
    );
}

#[test]
fn rejects_identifiers_with_invalid_chars() {
    let source = "x$y some?method".to_string();
    let mut lexer = Lexer::new(source);

    let _tokens = lexer.lex();
    assert!(lexer.has_errors());
    assert_eq!(lexer.errors().len(), 2);
}
