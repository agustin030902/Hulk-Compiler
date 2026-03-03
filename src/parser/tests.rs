use crate::lexer::Lexer;

use super::{
    Parser,
    expression::{BinaryOp, Expr, Literal, Program, Statement},
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

#[test]
fn parses_string_concat_with_number() {
    let program = parse_program(r#"print("The meaning of life is " @ 42);"#);

    assert_eq!(program.statements.len(), 1);
    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };

    let Expr::Binary(binary) = value else {
        panic!("expected binary expression");
    };

    assert!(matches!(binary.op, BinaryOp::Concat));
    assert!(matches!(
        binary.left.as_ref(),
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "The meaning of life is "
    ));
    assert!(matches!(
        binary.right.as_ref(),
        Expr::Literal {
            value: Literal::Integer(42),
            ..
        }
    ));
}

#[test]
fn parses_concat_as_left_associative() {
    let program = parse_program(r#"print("a" @ 1 @ "b");"#);

    assert_eq!(program.statements.len(), 1);
    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };

    let Expr::Binary(outer) = value else {
        panic!("expected outer binary expression");
    };
    assert!(matches!(outer.op, BinaryOp::Concat));

    assert!(matches!(
        outer.right.as_ref(),
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "b"
    ));

    let Expr::Binary(inner) = outer.left.as_ref() else {
        panic!("expected inner binary expression");
    };
    assert!(matches!(inner.op, BinaryOp::Concat));
    assert!(matches!(
        inner.left.as_ref(),
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "a"
    ));
    assert!(matches!(
        inner.right.as_ref(),
        Expr::Literal {
            value: Literal::Integer(1),
            ..
        }
    ));
}
