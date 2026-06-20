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

#[test]
fn rejects_type_with_only_current_no_next() {
    let source = r#"
type OnlyCurrent(val: Number) {
    val = val;
    current() => self.val;
}
for (x in new OnlyCurrent(5)) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        !errors.is_empty(),
        "expected error: type has current() but missing next()"
    );
    assert!(
        errors.iter().any(|e| e.message.contains("missing 'next()'")),
        "expected 'missing next()' error, got: {:?}",
        errors
    );
}

#[test]
fn rejects_type_with_next_returning_non_boolean() {
    let source = r#"
type BadNext(val: Number) {
    val = val;
    next() => self.val;
    current() => self.val;
}
for (x in new BadNext(5)) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        !errors.is_empty(),
        "expected error: next() must return Boolean"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("next()' must return Boolean")),
        "expected 'next() must return Boolean' error, got: {:?}",
        errors
    );
}

#[test]
fn rejects_type_with_next_taking_parameters() {
    let source = r#"
type BadNextParam(val: Number) {
    val = val;
    next(n: Number) => self.val > 0;
    current() => self.val;
}
for (x in new BadNextParam(5)) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        !errors.is_empty(),
        "expected error: next() must take no parameters"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("next()' must take no parameters")),
        "expected 'next() must take no parameters' error, got: {:?}",
        errors
    );
}

#[test]
fn rejects_type_with_current_taking_parameters() {
    let source = r#"
type BadCurrentParam(val: Number) {
    val = val;
    next() => self.val > 0;
    current(n: Number) => self.val;
}
for (x in new BadCurrentParam(5)) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        !errors.is_empty(),
        "expected error: current() must take no parameters"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("current()' must take no parameters")),
        "expected 'current() must take no parameters' error, got: {:?}",
        errors
    );
}
