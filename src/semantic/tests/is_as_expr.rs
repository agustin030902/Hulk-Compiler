use crate::{error::ErrorCategory, lexer::Lexer, parser::Parser};

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
fn allows_is_expression_with_struct_types() {
    let source = r#"
    type Animal { name: String = "unknown"; }
    type Bird inherits Animal { }
    let x = new Bird();
    print(x is Animal);
    "#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn allows_as_expression_with_struct_types() {
    let source = r#"
    type Animal { name: String = "unknown"; }
    type Bird inherits Animal { }
    let x = new Bird();
    let y = x as Animal;
    "#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_is_with_unknown_type() {
    let source = r#"
    type Bird { }
    let x = new Bird();
    print(x is NonExistent);
    "#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Semantic);
    assert!(errors[0].message.contains("Unknown type 'NonExistent'"));
}

#[test]
fn rejects_as_with_unknown_type() {
    let source = r#"
    type Bird { }
    let x = new Bird();
    let y = x as NonExistent;
    "#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Semantic);
    assert!(errors[0].message.contains("Unknown type 'NonExistent'"));
}

#[test]
fn allows_is_with_self_type() {
    let source = r#"
    type Bird { }
    let x = new Bird();
    print(x is Bird);
    "#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn allows_as_with_self_type() {
    let source = r#"
    type Bird { }
    let x = new Bird();
    let y = x as Bird;
    "#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}
