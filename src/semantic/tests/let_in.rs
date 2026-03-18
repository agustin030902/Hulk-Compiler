use crate::{lexer::Lexer, parser::Parser, semantic::SemanticAnalyzer};

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
fn allows_let_in_scoped_bindings() {
    let source = r#"
let x = 1;
let y = let x = 9, z = 1 in x + z;
print(x);
print(y);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn let_in_body_type_flows() {
    let source = r#"
let result = let a = 2 in a * 3;
print(result);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn reports_type_error_inside_let_in() {
    let source = r#"
let bad = let a = true in a + 1;
print(bad);
"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0]
            .message
            .contains("Operator '+' expects Number and Number")
    );
}
