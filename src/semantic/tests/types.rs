use crate::error::ErrorCategory;

use super::analyze_source;

#[test]
fn accepts_type_declaration_instantiation_and_method_calls() {
    let source = r#"
type Point(x: Number, y: Number) {
    x = x;
    y = y;

    norm() => sqrt(self.x ^ 2 + self.y ^ 2);
    add(other: Point) => new Point(self.x + other.x, self.y + other.y);
    describe() => "(" @ self.x @ ", " @ self.y @ ")";
}

let p1 = new Point(3, 4);
let p2 = new Point(1, 2);
let p3 = p1.add(p2);
print(p1.describe() @ " norm=" @ p1.norm());
print(p2.describe() @ " norm=" @ p2.norm());
print(p3.describe() @ " norm=" @ p3.norm());
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_private_attribute_access_outside_type() {
    let source = r#"
type Point(x: Number) {
    x = x;
    getX() => self.x;
}

let p = new Point(1);
print(p.x);
"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Semantic);
    assert_eq!(
        errors[0].message,
        "Attribute 'x' is private and cannot be accessed from this context."
    );
}

#[test]
fn rejects_self_as_destructive_assign_target_in_method() {
    let source = r#"
type A() {
    id() => 0;
    bad() => self := new A();
}
"#;

    let errors = analyze_source(source);
    assert!(
        errors.iter().any(|error| {
            error.category == ErrorCategory::Semantic
                && error.message == "`self` is not a valid assignment target."
        }),
        "expected invalid self assignment error, got: {:?}",
        errors
    );
}

#[test]
fn rejects_self_usage_in_attribute_initializer() {
    let source = r#"
type A(x: Number) {
    x = x;
    y = self.x;
}
"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Semantic);
    assert_eq!(
        errors[0].message,
        "Variable 'self' is used before declaration. Declare it with 'let' first."
    );
}

#[test]
fn rejects_constructor_argument_type_mismatch() {
    let source = r#"
type Point(x: Number, y: Number) {
    x = x;
    y = y;
}

let p = new Point(true, 2);
"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Type 'Point' constructor argument #1 expects Number, but got Boolean."
    );
}

#[test]
fn allows_null_constructor_arguments_for_struct_references() {
    let source = r#"
type Node(value: Number, left: Node, right: Node) {
    value = value;
    left = left;
    right = right;

    isLeaf() => self.left == null && self.right == null;
}

let leaf = new Node(1, null, null);
print(leaf.isLeaf());
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn accepts_inheritance_subtyping_and_inherited_methods() {
    let source = r#"
type Animal(name: String) {
    name = name;
    label() => self.name;
}

type Dog(name: String, age: Number) inherits Animal(name) {
    age = age;
    ageLabel() => self.age @ "";
}

let animal: Animal = new Dog("Sasha", 4);
let dog = new Dog("Firu", 2);
print(dog.label() @ " " @ dog.ageLabel());
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_unknown_parent_type() {
    let source = r#"
type Dog(name: String) inherits Animal(name) {
    name = name;
}
"#;

    let errors = analyze_source(source);
    assert!(
        errors.iter().any(|error| {
            error.category == ErrorCategory::Semantic
                && error.message == "Parent type 'Animal' not found."
        }),
        "expected unknown parent type error, got: {:?}",
        errors
    );
}

#[test]
fn rejects_override_with_different_signature() {
    let source = r#"
type Animal() {
    speak() => "generic";
}

type Dog() inherits Animal() {
    speak(times: Number) => "woof";
}
"#;

    let errors = analyze_source(source);
    assert!(
        errors.iter().any(|error| {
            error.category == ErrorCategory::Semantic
                && error.message
                    == "Method 'speak' override in type 'Dog' has different signature than parent."
        }),
        "expected invalid override error, got: {:?}",
        errors
    );
}

#[test]
fn rejects_parent_constructor_argument_type_mismatch() {
    let source = r#"
type Animal(name: String) {
    name = name;
}

type Dog(age: Number) inherits Animal(age) {
    age = age;
}
"#;

    let errors = analyze_source(source);
    assert!(
        errors.iter().any(|error| {
            error.category == ErrorCategory::Type
                && error.message
                    == "Parent type 'Animal' constructor argument #1 expects String, but got Number."
        }),
        "expected parent constructor type error, got: {:?}",
        errors
    );
}

#[test]
fn rejects_null_for_number_constructor_parameter() {
    let source = r#"
type Box(value: Number) {
    value = value;
}

let b = new Box(null);
"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Type 'Box' constructor argument #1 expects Number, but got Null."
    );
}
