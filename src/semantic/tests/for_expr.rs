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
fn allows_for_loop_with_range() {
    let source = r#"
for (x in range(0, 5)) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn allows_for_loop_with_variable_iterable() {
    let source = r#"
let r = range(1, 11);
for (x in r) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn allows_for_loop_with_accumulation() {
    let source = r#"
let total = 0;
for (x in range(1, 6)) {
    total = total + x;
};
print(total);
"#;

    let errors = analyze(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn allows_user_type_as_iterable_with_next_and_current() {
    let source = r#"
type Countdown(count: Number) {
    count = count;
    next() => { self.count := self.count - 1; self.count > 0; };
    current() => self.count;
}
for (x in new Countdown(3)) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_for_loop_with_boolean_iterable() {
    let source = r#"
for (x in true) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(!errors.is_empty(), "expected semantic errors for non-iterable");
}

#[test]
fn allows_range_new_expression_with_two_args() {
    let source = r#"
let r = range(0, 10);
let n = r.current();
print(n);
"#;

    let errors = analyze(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}
