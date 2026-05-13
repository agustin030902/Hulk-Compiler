use crate::lexer::Lexer;
use crate::parser::{
    Parser,
    expression::{BinaryOp, Expr, Program},
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
fn parses_function_with_typed_parameter_and_typed_return() {
    let program = parse_program("function tan(x: Number): Number => sin(x) / cos(x);");
    assert_eq!(program.functions.len(), 1);

    let function = &program.functions[0];
    assert_eq!(function.name, "tan");
    assert_eq!(function.params.len(), 1);

    assert_eq!(function.params[0].name, "x");
    let param_annotation = function.params[0]
        .type_annotation
        .as_ref()
        .expect("parameter annotation should be present");
    assert_eq!(param_annotation.name, "Number");

    let return_annotation = function
        .return_type_annotation
        .as_ref()
        .expect("return annotation should be present");
    assert_eq!(return_annotation.name, "Number");

    assert!(matches!(
        function.body,
        Expr::Binary(ref bin) if matches!(bin.op, BinaryOp::Div)
    ));
}

#[test]
fn parses_function_with_partial_parameter_annotations() {
    let program = parse_program("function pick(a, b: Number, c): Number => b;");
    assert_eq!(program.functions.len(), 1);

    let function = &program.functions[0];
    assert_eq!(function.params.len(), 3);

    assert_eq!(function.params[0].name, "a");
    assert!(function.params[0].type_annotation.is_none());

    assert_eq!(function.params[1].name, "b");
    let b_annotation = function.params[1]
        .type_annotation
        .as_ref()
        .expect("b annotation should be present");
    assert_eq!(b_annotation.name, "Number");

    assert_eq!(function.params[2].name, "c");
    assert!(function.params[2].type_annotation.is_none());

    let return_annotation = function
        .return_type_annotation
        .as_ref()
        .expect("return annotation should be present");
    assert_eq!(return_annotation.name, "Number");
}

#[test]
fn parses_block_function_with_typed_signature() {
    let program = parse_program(
        r#"
function banner(prefix: String): String {
    prefix @ "!";
}
"#,
    );
    assert_eq!(program.functions.len(), 1);

    let function = &program.functions[0];
    assert_eq!(function.name, "banner");
    assert_eq!(function.params.len(), 1);

    let param_annotation = function.params[0]
        .type_annotation
        .as_ref()
        .expect("parameter annotation should be present");
    assert_eq!(param_annotation.name, "String");

    let return_annotation = function
        .return_type_annotation
        .as_ref()
        .expect("return annotation should be present");
    assert_eq!(return_annotation.name, "String");
}

#[test]
fn reports_error_when_function_parameter_annotation_name_is_missing() {
    let message = parse_error_message("function f(x:) => x;");
    assert!(message.contains("Unexpected token )."));
    assert!(message.contains("identifier"));
}

#[test]
fn reports_error_when_function_return_annotation_name_is_missing() {
    let message = parse_error_message("function f(x): => x;");
    assert!(message.contains("Unexpected token =>."));
    assert!(message.contains("identifier"));
}

#[test]
fn reports_error_when_arrow_is_missing_after_typed_return() {
    let message = parse_error_message("function f(x): Number x;");
    assert!(message.contains("Unexpected token identifier(x)."));
    assert!(message.contains("=>"));
}
