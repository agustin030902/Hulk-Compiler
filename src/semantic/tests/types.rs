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
    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
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
fn allows_null_for_struct_typed_binding() {
    let source = r#"
type Node(v: Number, next: Node) {
    v = v;
    next = next;
}

let head: Node = null;
"#;

    let errors = analyze_source(source);
    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn rejects_null_for_number_typed_binding() {
    let source = r#"
let n: Number = null;
"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
}

#[test]
fn allows_polymorphic_assignment_from_child_to_parent() {
    let source = r#"
type Animal(name: String) {
    name = name;
    speak() => "generic";
}

type Dog(name: String) inherits Animal(name) {
    speak() => "woof";
}

let pet: Animal = new Dog("Firulais");
print(pet.speak());
"#;

    let errors = analyze_source(source);
    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn registers_object_as_implicit_parent() {
    let source = r#"
type A() { id() => 1; }
"#;

    let mut lexer = crate::lexer::Lexer::new(source.to_string());
    let tokens = lexer.lex();
    assert!(!lexer.has_errors(), "lexer errors: {:?}", lexer.errors());

    let mut parser = crate::parser::Parser::new(source);
    let program = parser.parse_program(tokens).expect("program");
    assert!(!parser.has_errors(), "parser errors: {:?}", parser.errors());

    let mut analyzer = crate::semantic::SemanticAnalyzer::new();
    let errors = analyzer.analyze(&program, source);
    assert!(errors.is_empty(), "semantic errors: {:?}", errors);

    let object_id = analyzer.type_symbols().get("Object").copied().unwrap();
    let a_id = analyzer.type_symbols().get("A").copied().unwrap();
    let a_info = analyzer.type_table().get_struct(a_id).unwrap();

    assert_eq!(a_info.parent, Some(object_id));
}
