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
fn allows_while_expression_with_boolean_condition() {
    let source = r#"
let i = 0;
let loop_result = while (i < 3) {
    i = i + 1;
};
loop_result;
"#;

    let errors = analyze(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_non_boolean_while_condition() {
    let source = r#"
while (1) {
    print(1);
};
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "While condition expects Boolean, but got Number."
    );
}

#[test]
fn rejects_unit_in_arithmetic_expression() {
    let source = r#"
let side = print(5);
let invalid = side + 1;
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Operator '+' expects Number and Number, but got Unit and Number."
    );
}
