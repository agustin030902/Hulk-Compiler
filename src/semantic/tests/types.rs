use crate::error::ErrorCategory;

use super::analyze_source;

#[test]
fn allows_recursive_type_declaration_for_tree_nodes() {
    let source = r#"
type Node(value: Number, left: Node, right: Node) {
    value: Number = value;
    left: Node = left;
    right: Node = right;
    valueOf(): Number => self.value;
    replaceLeft(child: Node) => self.left := child;
}
42;
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_self_as_assignment_target_inside_method() {
    let source = r#"
type A {
    reset() {
        self := new A();
        0;
    }
}
42;
"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Semantic);
    assert_eq!(
        errors[0].message,
        "`self` is not a valid assignment target."
    );
}

#[test]
fn rejects_private_attribute_access_outside_type() {
    let source = r#"
type Point(x: Number) {
    x: Number = x;
    getX(): Number => self.x;
}
let p = new Point(3) in p.x;
"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Semantic);
    assert!(
        errors[0]
            .message
            .contains("is private and cannot be accessed outside its type"),
        "unexpected message: {}",
        errors[0].message
    );
}

#[test]
fn allows_new_and_method_call_with_typed_return() {
    let source = r#"
type Box(v: Number) {
    v: Number = v;
    get(): Number => self.v;
}
let b = new Box(10) in b.get() + 1;
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}
