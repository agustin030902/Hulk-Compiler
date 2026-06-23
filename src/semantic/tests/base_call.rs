use super::*;

#[test]
fn allows_base_call_in_subtype_method() {
    let source = r#"
type Animal(name: String) {
    name = name;
    speak() => self.name;
}

type Dog(name: String) inherits Animal(name) {
    speak() => base() @ " barks";
}

let d = new Dog("Rex");
print(d.speak());
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_base_call_outside_method() {
    let source = r#"
type Animal(name: String) {
    name = name;
}

let x = base();
print(x);
"#;

    let errors = analyze_source(source);
    assert!(
        !errors.is_empty(),
        "expected error for base() outside method"
    );
    assert!(
        errors.iter().any(|e| e.message.contains("base()")),
        "expected error about base(), got: {:?}",
        errors
    );
}

#[test]
fn rejects_base_call_with_wrong_arg_count() {
    let source = r#"
type Animal(name: String) {
    name = name;
    greet(x: String) => self.name;
}

type Dog(name: String) inherits Animal(name) {
    greet(x: String) => base(x, "extra");
}

let d = new Dog("Rex");
print(d.greet("hello"));
"#;

    let errors = analyze_source(source);
    assert!(
        !errors.is_empty(),
        "expected error for base() with wrong arg count"
    );
    assert!(
        errors.iter().any(|e| e.message.contains("base()") && e.message.contains("argument")),
        "expected argument count error, got: {:?}",
        errors
    );
}

#[test]
fn rejects_base_call_with_wrong_arg_type() {
    let source = r#"
type Animal(name: String) {
    name = name;
    greet(x: String) => self.name;
}

type Dog(name: String) inherits Animal(name) {
    greet(x: String) => base(1);
}

let d = new Dog("Rex");
print(d.greet("hello"));
"#;

    let errors = analyze_source(source);
    assert!(
        !errors.is_empty(),
        "expected error for base() with wrong arg type"
    );
    assert!(
        errors.iter().any(|e| e.message.contains("Base call argument")),
        "expected arg type error, got: {:?}",
        errors
    );
}

#[test]
fn rejects_base_call_without_parent() {
    let source = r#"
type Animal(name: String) {
    name = name;
    speak() => base();
}

let a = new Animal("Rex");
print(a.speak());
"#;

    let errors = analyze_source(source);
    assert!(
        !errors.is_empty(),
        "expected error for base() without parent"
    );
    assert!(
        errors.iter().any(|e| e.message.contains("parent")),
        "expected parent error, got: {:?}",
        errors
    );
}
