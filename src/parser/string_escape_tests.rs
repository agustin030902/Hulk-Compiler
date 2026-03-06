use crate::lexer::{Lexer, Token, TokenKind};

use super::{
    Parser,
    expression::{Expr, Literal, Program, Statement},
};

fn parse_program(source: &str) -> Program {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.lex();
    assert!(
        !lexer.has_errors(),
        "lexer produced errors: {:?}",
        lexer.errors()
    );

    let mut parser = Parser::new(source);
    let program = parser.parse_program(tokens);
    assert!(
        !parser.has_errors(),
        "parser produced errors: {:?}",
        parser.errors()
    );

    program.expect("parser did not produce a program")
}

fn parse_program_from_tokens(source: &str, tokens: Vec<Token>) -> Program {
    let mut parser = Parser::new(source);
    let program = parser.parse_program(tokens);
    assert!(
        !parser.has_errors(),
        "parser produced errors: {:?}",
        parser.errors()
    );

    program.expect("parser did not produce a program")
}

fn token(kind: TokenKind, start: usize, end: usize) -> Token {
    let value = match &kind {
        TokenKind::Identifier(v)
        | TokenKind::Number(v)
        | TokenKind::String(v)
        | TokenKind::Boolean(v) => v.clone(),
        TokenKind::Let => "let".to_string(),
        TokenKind::Print => "print".to_string(),
        TokenKind::Pi => "PI".to_string(),
        TokenKind::E => "E".to_string(),
        TokenKind::Sin => "sin".to_string(),
        TokenKind::Cos => "cos".to_string(),
        TokenKind::Sqrt => "sqrt".to_string(),
        TokenKind::Exp => "exp".to_string(),
        TokenKind::Log => "log".to_string(),
        TokenKind::Assign => "=".to_string(),
        TokenKind::Add => "+".to_string(),
        TokenKind::Power => "^".to_string(),
        TokenKind::Concat => "@".to_string(),
        TokenKind::Minus => "-".to_string(),
        TokenKind::Multiply => "*".to_string(),
        TokenKind::Divide => "/".to_string(),
        TokenKind::EqualEqual => "==".to_string(),
        TokenKind::NotEqual => "!=".to_string(),
        TokenKind::Less => "<".to_string(),
        TokenKind::Greater => ">".to_string(),
        TokenKind::LessEqual => "<=".to_string(),
        TokenKind::GreaterEqual => ">=".to_string(),
        TokenKind::And => "&&".to_string(),
        TokenKind::Or => "||".to_string(),
        TokenKind::Not => "!".to_string(),
        TokenKind::Comma => ",".to_string(),
        TokenKind::Semicolon => ";".to_string(),
        TokenKind::LeftParen => "(".to_string(),
        TokenKind::RightParen => ")".to_string(),
        TokenKind::Unknown => "unknown".to_string(),
        TokenKind::EOF => String::new(),
    };

    Token {
        kind,
        value,
        line: 1,
        column: 1,
        start,
        end,
    }
}

#[test]
fn parses_escaped_double_quote_in_string_literal() {
    let program = parse_program(r#"print("The message is \"Hello World\"");"#);

    assert_eq!(program.statements.len(), 1);
    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };

    assert!(matches!(
        value,
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "The message is \"Hello World\""
    ));
}

#[test]
fn parses_newline_and_tab_escape_sequences_in_string_literal() {
    let program = parse_program(r#"print("Line1\nLine2\tDone");"#);

    assert_eq!(program.statements.len(), 1);
    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };

    assert!(matches!(
        value,
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "Line1\nLine2\tDone"
    ));
}

#[test]
fn parser_normalizes_escaped_sequences_from_string_tokens() {
    let source = r#"print("Line1\nLine2\tDone");"#;
    let tokens = vec![
        token(TokenKind::Print, 0, 5),
        token(TokenKind::LeftParen, 5, 6),
        token(TokenKind::String("Line1\\nLine2\\tDone".to_string()), 6, 24),
        token(TokenKind::RightParen, 24, 25),
        token(TokenKind::Semicolon, 25, 26),
        token(TokenKind::EOF, 26, 26),
    ];
    let program = parse_program_from_tokens(source, tokens);

    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };
    assert!(matches!(
        value,
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "Line1\nLine2\tDone"
    ));
}

#[test]
fn parser_normalizes_escaped_quotes_inside_concat_operands() {
    let source = r#"print("A \"quoted\" value" @ "!");"#;
    let tokens = vec![
        token(TokenKind::Print, 0, 5),
        token(TokenKind::LeftParen, 5, 6),
        token(
            TokenKind::String("A \\\"quoted\\\" value".to_string()),
            6,
            25,
        ),
        token(TokenKind::Concat, 26, 27),
        token(TokenKind::String("!".to_string()), 28, 31),
        token(TokenKind::RightParen, 31, 32),
        token(TokenKind::Semicolon, 32, 33),
        token(TokenKind::EOF, 33, 33),
    ];
    let program = parse_program_from_tokens(source, tokens);

    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };
    let Expr::Binary(binary) = value else {
        panic!("expected binary expression");
    };

    assert!(matches!(
        binary.left.as_ref(),
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "A \"quoted\" value"
    ));
}
