use crate::{error::ErrorCategory, lexer::Lexer, parser::Parser, semantic::SemanticAnalyzer};

fn analyze(source: &str) -> Vec<crate::error::CompilerError> {
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
    analyzer.analyze(&program.expect("program"), source)
}

#[test]
fn allows_function_with_typed_parameter_and_typed_return() {
    let source = r#"
function tan(x: Number): Number => sin(x) / cos(x);
print(tan(1));
"#;

    let errors = analyze(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn allows_function_with_partial_parameter_annotations() {
    let source = r#"
function sum_with_base(base, x: Number): Number => base + x;
print(sum_with_base(10, 2));
"#;

    let errors = analyze(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_call_argument_that_violates_parameter_annotation() {
    let source = r#"
function id(x: Number): Number => x;
print(id(true));
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Function 'id' argument #1 expects Number, but got Boolean."
    );
}

#[test]
fn rejects_function_body_when_parameter_annotation_is_used_inconsistently() {
    let source = r#"
function invalid(flag: Number): Boolean => !flag;
print(invalid(1));
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Unary '!' expects Boolean, but got Number."
    );
}

#[test]
fn rejects_function_body_return_type_that_does_not_conform_to_annotation() {
    let source = r#"
function invalid(): Number => "text";
print(invalid());
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Function 'invalid' return type conflict: Number vs String."
    );
}

#[test]
fn rejects_unknown_parameter_type_annotation() {
    let source = r#"
function f(x: Numeric) => x + 1;
print(f(1));
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Semantic);
    assert_eq!(
        errors[0].message,
        "Unknown type annotation 'Numeric'. Expected one of: Number, Boolean, String, Unit, Null, Enumerable, Iterable, Object, Range."
    );
}

#[test]
fn rejects_unknown_return_type_annotation() {
    let source = r#"
function f(x): Numeric => x + 1;
print(f(1));
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Semantic);
    assert_eq!(
        errors[0].message,
        "Unknown type annotation 'Numeric'. Expected one of: Number, Boolean, String, Unit, Null, Enumerable, Iterable, Object, Range."
    );
}

#[test]
fn return_annotation_can_constrain_parameter_inference() {
    let source = r#"
function identity(x): Number => x;
print(identity(42));
"#;

    let errors = analyze(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}
