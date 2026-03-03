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
fn allows_concat_with_escaped_quotes_newline_and_tab() {
    let source = r#"
let header = "The message is \"Hello\"";
let details = "line1\nline2\tok";
let full = header @ " | " @ details @ " | id=" @ 42;
print(full);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_numeric_addition_with_escaped_string() {
    let source = r#"print("Line1\nLine2" + 1);"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Operator '+' expects Number and Number, but got String and Number."
    );
}

#[test]
fn rejects_concat_between_boolean_and_escaped_string() {
    let source = r#"print(true @ "The message is \"Hello\"");"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Operator '@' expects (String, String), (String, Number), or (Number, String), but got Boolean and String."
    );
}
