use crate::lexer::Lexer;
use crate::parser::{
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

#[test]
fn parses_destructive_assignment_expression() {
    let program = parse_program("let x = 0; x := 1;");
    let Statement::Expr { value, .. } = &program.statements[1] else {
        panic!("expected expression statement");
    };

    let Expr::DestructiveAssign(assign) = value else {
        panic!("expected destructive assignment expression");
    };
    assert_eq!(assign.name, "x");
    assert!(matches!(
        assign.value.as_ref(),
        Expr::Literal {
            value: Literal::Integer(1),
            ..
        }
    ));
}

#[test]
fn destructive_assignment_is_right_associative() {
    let program = parse_program("a := b := 3;");
    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    let Expr::DestructiveAssign(outer) = value else {
        panic!("expected outer destructive assignment");
    };
    assert_eq!(outer.name, "a");

    let Expr::DestructiveAssign(inner) = outer.value.as_ref() else {
        panic!("expected inner destructive assignment");
    };
    assert_eq!(inner.name, "b");
}
