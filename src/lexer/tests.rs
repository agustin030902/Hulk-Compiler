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

#[test]
fn lexes_comparison_and_logical_operators() {
    let source = r#"print(!(x <= 10) || true && y != 0);"#.to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::Not,
            TokenKind::LeftParen,
            TokenKind::Identifier("x".to_string()),
            TokenKind::LessEqual,
            TokenKind::Number("10".to_string()),
            TokenKind::RightParen,
            TokenKind::Or,
            TokenKind::Boolean("true".to_string()),
            TokenKind::And,
            TokenKind::Identifier("y".to_string()),
            TokenKind::NotEqual,
            TokenKind::Number("0".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}

#[test]
fn lexes_reassignment_statement() {
    let source = r#"let x = 45; x = true;"#.to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Let,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Assign,
            TokenKind::Number("45".to_string()),
            TokenKind::Semicolon,
            TokenKind::Identifier("x".to_string()),
            TokenKind::Assign,
            TokenKind::Boolean("true".to_string()),
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}

#[test]
fn lexes_builtin_math_functions_and_constants() {
    let source = r#"print(sin(PI) + cos(E) + sqrt(9) + exp(1) + log(4, 64));"#.to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::Sin,
            TokenKind::LeftParen,
            TokenKind::Pi,
            TokenKind::RightParen,
            TokenKind::Add,
            TokenKind::Cos,
            TokenKind::LeftParen,
            TokenKind::E,
            TokenKind::RightParen,
            TokenKind::Add,
            TokenKind::Sqrt,
            TokenKind::LeftParen,
            TokenKind::Number("9".to_string()),
            TokenKind::RightParen,
            TokenKind::Add,
            TokenKind::Exp,
            TokenKind::LeftParen,
            TokenKind::Number("1".to_string()),
            TokenKind::RightParen,
            TokenKind::Add,
            TokenKind::Log,
            TokenKind::LeftParen,
            TokenKind::Number("4".to_string()),
            TokenKind::Comma,
            TokenKind::Number("64".to_string()),
            TokenKind::RightParen,
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}

#[test]
fn lexes_power_operator_with_right_associative_shape() {
    let source = r#"print(2 ^ 3 ^ 2);"#.to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::Number("2".to_string()),
            TokenKind::Power,
            TokenKind::Number("3".to_string()),
            TokenKind::Power,
            TokenKind::Number("2".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}
