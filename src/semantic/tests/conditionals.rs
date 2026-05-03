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
fn allows_if_else_expression_with_matching_branch_types() {
    let source = r#"
let value = if (true) 1 else 2;
print(value);
"#;

    let errors = analyze(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn allows_if_elif_else_expression_with_matching_branch_types() {
    let source = r#"
let x = 0;
let label = if (x > 0) "positive" elif (x == 0) "zero" else "negative";
print(label);
"#;

    let errors = analyze(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn allows_if_expression_with_block_branches() {
    let source = r#"
let x = 1;
let label = if (x > 0) {
    print(x);
    "positive"
} else {
    "not positive"
};
print(label);
"#;

    let errors = analyze(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_non_boolean_if_condition() {
    let source = r#"
let value = if (1) "yes" else "no";
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "If condition expects Boolean, but got Number."
    );
}

#[test]
fn rejects_non_boolean_elif_condition() {
    let source = r#"
let value = if (false) "no" elif (1) "maybe" else "yes";
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "If condition expects Boolean, but got Number."
    );
}

#[test]
fn rejects_if_branches_with_mismatched_types() {
    let source = r#"
let value = if (true) 1 else "no";
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "If branches must return the same type, but got Number and String."
    );
}

#[test]
fn rejects_elif_branch_with_mismatched_type() {
    let source = r#"
let value = if (false) 1 elif (true) "yes" else 2;
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "If branches must return the same type, but got Number and String."
    );
}
