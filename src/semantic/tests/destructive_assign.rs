use crate::{lexer::Lexer, parser::Parser, semantic::SemanticAnalyzer};

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
fn allows_destructive_assignment_of_same_type() {
    let source = r#"
let a = 0;
a := 1;
print(a);
"#;

    let errors = analyze(source);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn reports_undefined_variable_on_destructive_assignment() {
    let source = r#"
x := 2;
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0]
            .message
            .contains("Variable 'x' is assigned before declaration"),
        "unexpected error message: {:?}",
        errors[0]
    );
}

#[test]
fn rejects_type_change_on_destructive_assignment() {
    let source = r#"
let a = 1;
a := true;
"#;

    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].message.contains("requires the same type"),
        "unexpected error message: {:?}",
        errors[0]
    );
}
