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
fn allows_for_loop_with_user_iterable_type() {
    let source = r#"
type Countdown(count: Number) {
    count = count;
    next() => { self.count := self.count - 1; self.count > 0; };
    current() => self.count;
    iter() => self;
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
fn allows_for_loop_with_user_enumerable_type() {
    let source = r#"
type MyCounter(n: Number) {
    n = n;
    iter() => new CounterIter(self.n);
}

type CounterIter(current: Number) {
    current = current;
    next() => { self.current := self.current + 1; self.current <= 10; };
    current() => self.current;
}

for (x in new MyCounter(1)) {
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
fn rejects_for_loop_with_boolean() {
    let source = r#"
for (x in true) {
    print(x);
};
"#;
    let errors = analyze(source);
    assert!(!errors.is_empty(), "expected semantic errors for non-iterable");
}

#[test]
fn rejects_for_loop_with_number() {
    let source = r#"
for (x in 42) {
    print(x);
};
"#;
    let errors = analyze(source);
    assert!(!errors.is_empty(), "expected semantic errors for non-iterable");
}

#[test]
fn rejects_enumerable_with_iter_returning_non_struct() {
    let source = r#"
type BadEnumerable() {
    iter() => 42;
}
for (x in new BadEnumerable()) {
    print(x);
};
"#;
    let errors = analyze(source);
    assert!(
        !errors.is_empty(),
        "expected error: iter() must return an Iterable type"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("iter()' must return an Iterable")),
        "expected 'iter() must return an Iterable' error, got: {:?}",
        errors
    );
}

#[test]
fn rejects_enumerable_iterator_missing_next() {
    let source = r#"
type MyEnum() {
    iter() => new BadIter(0);
}

type BadIter(current: Number) {
    current = current;
    current() => self.current;
}

for (x in new MyEnum()) {
    print(x);
};
"#;
    let errors = analyze(source);
    assert!(
        !errors.is_empty(),
        "expected error: iterator missing next()"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("missing 'next()'")),
        "expected 'missing next()' error, got: {:?}",
        errors
    );
}

#[test]
fn rejects_enumerable_iterator_next_not_boolean() {
    let source = r#"
type MyEnum() {
    iter() => new BadIter2(0);
}

type BadIter2(current: Number) {
    current = current;
    next() => self.current;
    current() => self.current;
}

for (x in new MyEnum()) {
    print(x);
};
"#;
    let errors = analyze(source);
    assert!(
        !errors.is_empty(),
        "expected error: iterator next() must return Boolean"
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
fn rejects_enumerable_iter_taking_parameters() {
    let source = r#"
type MyEnum() {
    iter(n: Number) => new BadIter3();
}

type BadIter3(current: Number) {
    current = current;
    next() => true;
    current() => self.current;
}

for (x in new MyEnum()) {
    print(x);
};
"#;
    let errors = analyze(source);
    assert!(
        !errors.is_empty(),
        "expected error: iter() must take no parameters"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("iter()' must take no parameters")),
        "expected 'iter() must take no parameters' error, got: {:?}",
        errors
    );
}
