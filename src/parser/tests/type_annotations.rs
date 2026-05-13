use crate::lexer::Lexer;
use crate::parser::{
    Parser,
    expression::{Expr, Program, Statement},
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

fn parse_error_message(source: &str) -> String {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.lex();
    assert!(
        !lexer.has_errors(),
        "lexer produced errors: {:?}",
        lexer.errors()
    );

    let mut parser = Parser::new(source);
    let program = parser.parse_program(tokens);
    assert!(program.is_none(), "program should fail parsing");
    assert!(parser.has_errors(), "parser should report syntax errors");

    parser.errors()[0].message.clone()
}

#[test]
fn parses_typed_let_in_binding() {
    let program = parse_program("let x: Number = 42 in print(x);");
    assert_eq!(program.statements.len(), 1);

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    let Expr::LetIn(let_in) = value else {
        panic!("expected let-in expression");
    };
    assert_eq!(let_in.bindings.len(), 1);
    assert_eq!(let_in.bindings[0].name, "x");

    let annotation = let_in.bindings[0]
        .type_annotation
        .as_ref()
        .expect("type annotation should be present");
    assert_eq!(annotation.name, "Number");
}

#[test]
fn parses_typed_let_statement() {
    let program = parse_program(r#"let message: String = "hello";"#);
    assert_eq!(program.statements.len(), 1);

    let Statement::Let {
        name,
        type_annotation,
        ..
    } = &program.statements[0]
    else {
        panic!("expected let statement");
    };

    assert_eq!(name, "message");
    let annotation = type_annotation
        .as_ref()
        .expect("type annotation should be present");
    assert_eq!(annotation.name, "String");
}

#[test]
fn reports_error_when_type_annotation_name_is_missing() {
    let message = parse_error_message("let x: = 42 in x;");
    assert!(message.contains("Unexpected token =."));
    assert!(message.contains("identifier"));
}

#[test]
fn reports_error_when_assign_is_missing_after_type_annotation() {
    let message = parse_error_message("let x: Number 42 in x;");
    assert!(message.contains("Unexpected token number(42)."));
    assert!(message.contains("="));
}
