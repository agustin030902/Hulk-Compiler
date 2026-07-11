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
fn rejects_type_with_current_but_no_next() {
    let source = r#"
type NoNext {
    current() => 42;
}
for (x in new NoNext()) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        errors.iter().any(|e| e.message.contains("next")),
        "expected error about missing next(), got: {:?}",
        errors
    );
}

#[test]
fn rejects_next_with_wrong_return_type() {
    let source = r#"
type BadNext {
    next() => 42;
    current() => 42;
}
for (x in new BadNext()) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        errors.iter().any(|e| e.message.contains("next")),
        "expected error about wrong next() signature, got: {:?}",
        errors
    );
}

#[test]
fn rejects_next_with_parameters() {
    let source = r#"
type NextWithArgs {
    next(x: Number) => true;
    current() => 42;
}
for (x in new NextWithArgs()) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        errors.iter().any(|e| e.message.contains("next")),
        "expected error about wrong next() signature, got: {:?}",
        errors
    );
}

#[test]
fn rejects_current_with_parameters() {
    let source = r#"
type CurrentWithArgs {
    next() => true;
    current(x: Number) => 42;
}
for (x in new CurrentWithArgs()) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        errors.iter().any(|e| e.message.contains("current")),
        "expected error about wrong current() signature, got: {:?}",
        errors
    );
}

#[test]
fn allows_user_enumerable_type() {
    let source = r#"
type MyIterator {
    val = 0;
    next() => { self.val := self.val + 1; self.val <= 3; };
    current() => self.val;
}
type MyEnumerable {
    iter() => new MyIterator();
}
for (x in new MyEnumerable()) {
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
fn rejects_iter_with_wrong_return_type() {
    let source = r#"
type BadEnumerable {
    iter() => 42;
}
for (x in new BadEnumerable()) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        !errors.is_empty(),
        "expected semantic error for bad iter() return type"
    );
}

#[test]
fn rejects_iter_with_parameters() {
    let source = r#"
type IterableWithArgs {
    iter(x: Number) => new IterableWithArgs();
}
for (x in new IterableWithArgs()) {
    print(x);
};
"#;

    let errors = analyze(source);
    assert!(
        errors.iter().any(|e| e.message.contains("iter")),
        "expected error about wrong iter() signature, got: {:?}",
        errors
    );
}
