use crate::lexer::Lexer;
use crate::parser::Parser;

fn parse_program(source: &str) -> crate::parser::expression::Program {
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
fn parses_function_with_splat_parameter_annotation() {
    let program = parse_program("function sum(numbers: Number*): Number => 0;");
    assert_eq!(program.functions.len(), 1);

    let function = &program.functions[0];
    assert_eq!(function.name, "sum");
    assert_eq!(function.params.len(), 1);

    let param = &function.params[0];
    assert_eq!(param.name, "numbers");
    let annotation = param
        .type_annotation
        .as_ref()
        .expect("parameter annotation should be present");
    assert_eq!(annotation.name, "Number");
    assert!(annotation.is_splat, "should be a splat annotation");
}

#[test]
fn parses_function_with_splat_return_annotation() {
    let program = parse_program("function get_iter(): Number* => null;");
    assert_eq!(program.functions.len(), 1);

    let function = &program.functions[0];
    let return_annotation = function
        .return_type_annotation
        .as_ref()
        .expect("return annotation should be present");
    assert_eq!(return_annotation.name, "Number");
    assert!(
        return_annotation.is_splat,
        "return should be a splat annotation"
    );
}

#[test]
fn parses_let_binding_with_splat_annotation() {
    let program = parse_program("let items: String* = null;");
    assert_eq!(program.statements.len(), 1);

    if let crate::parser::expression::Statement::Let {
        type_annotation, ..
    } = &program.statements[0]
    {
        let annotation = type_annotation
            .as_ref()
            .expect("let binding should have annotation");
        assert_eq!(annotation.name, "String");
        assert!(annotation.is_splat, "should be a splat annotation");
    } else {
        panic!("expected let statement");
    }
}

#[test]
fn parses_non_splat_annotation_without_star() {
    let program = parse_program("function f(x: Number): Number => x;");
    let function = &program.functions[0];
    let param = &function.params[0];
    let annotation = param
        .type_annotation
        .as_ref()
        .expect("annotation should be present");
    assert_eq!(annotation.name, "Number");
    assert!(!annotation.is_splat, "should NOT be a splat annotation");
}
