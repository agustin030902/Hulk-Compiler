mod conditionals;
mod identifier_rules;
mod let_in;
mod string_escape;
mod type_annotations;
mod types;
mod while_expr;

use super::{Lexer, TokenKind};

#[test]
fn lexes_function_declaration_and_user_call_tokens() {
    let source =
        r#"function fact(n) => if (n == 0) 1 else n * fact(n - 1); print(fact(5));"#.to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Function,
            TokenKind::Identifier("fact".to_string()),
            TokenKind::LeftParen,
            TokenKind::Identifier("n".to_string()),
            TokenKind::RightParen,
            TokenKind::Arrow,
            TokenKind::If,
            TokenKind::LeftParen,
            TokenKind::Identifier("n".to_string()),
            TokenKind::EqualEqual,
            TokenKind::Number("0".to_string()),
            TokenKind::RightParen,
            TokenKind::Number("1".to_string()),
            TokenKind::Else,
            TokenKind::Identifier("n".to_string()),
            TokenKind::Multiply,
            TokenKind::Identifier("fact".to_string()),
            TokenKind::LeftParen,
            TokenKind::Identifier("n".to_string()),
            TokenKind::Minus,
            TokenKind::Number("1".to_string()),
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::Identifier("fact".to_string()),
            TokenKind::LeftParen,
            TokenKind::Number("5".to_string()),
            TokenKind::RightParen,
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}

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

#[test]
fn lexes_rand_builtin_without_arguments() {
    let source = r#"print(rand());"#.to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Print,
            TokenKind::LeftParen,
            TokenKind::Rand,
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::RightParen,
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}

#[test]
fn lexes_expression_statement_literal() {
    let source = "42;".to_string();
    let mut lexer = Lexer::new(source);

    let tokens = lexer.lex();
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

    assert!(!lexer.has_errors());
    assert_eq!(
        kinds,
        vec![
            TokenKind::Number("42".to_string()),
            TokenKind::Semicolon,
            TokenKind::EOF,
        ]
    );
}

#[test]
fn lexes_block_tokens_without_trailing_semicolon() {
    let source = "let x = { let y = 1; y }".to_string();
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
            TokenKind::LeftBrace,
            TokenKind::Let,
            TokenKind::Identifier("y".to_string()),
            TokenKind::Assign,
            TokenKind::Number("1".to_string()),
            TokenKind::Semicolon,
            TokenKind::Identifier("y".to_string()),
            TokenKind::RightBrace,
            TokenKind::EOF,
        ]
    );
}
