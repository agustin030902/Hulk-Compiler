use crate::{
    error::ErrorCategory,
    lexer::Lexer,
    parser::Parser,
};

use super::SemanticAnalyzer;

fn analyze_source(source: &str) -> Vec<crate::error::CompilerError> {
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

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(
        &program.expect("parser did not produce program for semantic analysis"),
        source,
    )
}

#[test]
fn allows_concat_between_string_and_number_and_string() {
    let source = r#"
let a = "hello " @ 1;
let b = 2 @ "world";
let c = "foo" @ "bar";
let d = a @ "!";
print(d);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_concat_between_number_and_number_with_specific_error() {
    let source = r#"print(1 @ 2);"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Operator '@' expects (String, String), (String, Number), or (Number, String), but got Number and Number."
    );
}

#[test]
fn rejects_concat_between_boolean_and_string_with_specific_error() {
    let source = r#"print(true @ "x");"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Operator '@' expects (String, String), (String, Number), or (Number, String), but got Boolean and String."
    );
}
