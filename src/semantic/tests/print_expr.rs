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
fn print_expression_returns_unit_type() {
    let source = r#"
let side = print(5) in { side; 42 };
"#;
    let errors = analyze(source);
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
}

#[test]
fn rejects_printing_unit_value() {
    let source = r#"
let side = print(5) in print(side);
"#;
    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].message,
        "Function 'print' expects a non-Unit argument, but got Unit."
    );
}
