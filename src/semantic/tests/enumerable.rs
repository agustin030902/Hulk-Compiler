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
