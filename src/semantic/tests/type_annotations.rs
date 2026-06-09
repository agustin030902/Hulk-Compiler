use crate::error::ErrorCategory;

use super::analyze_source;

#[test]
fn accepts_typed_let_statement_when_initializer_matches() {
    let source = r#"
let x: Number = 42;
let y: String = "hello";
let z: Boolean = true;
print(x);
print(y);
print(z);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn accepts_typed_let_in_binding_when_initializer_matches() {
    let source = r#"
let result = let n: Number = 41, suffix: String = "!" in "value=" @ (n + 1) @ suffix;
print(result);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_typed_let_statement_when_initializer_type_mismatches() {
    let source = r#"let x: Number = "oops";"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Type annotation for variable 'x' expects Number, but initializer is String."
    );
}

#[test]
fn rejects_typed_let_in_binding_when_initializer_type_mismatches() {
    let source = r#"let x = let flag: Boolean = 1 in flag;"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Type annotation for variable 'flag' expects Boolean, but initializer is Number."
    );
}

#[test]
fn rejects_unknown_type_annotation_name() {
    let source = r#"let x: Numeric = 42;"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Semantic);
    assert_eq!(
        errors[0].message,
        "Unknown type annotation 'Numeric'. Expected one of: Number, Boolean, String, Unit, Null, Iterable, Object, Range."
    );
}

#[test]
fn uses_annotation_to_constrain_inferred_function_return_type() {
    let source = r#"
function id(x) => x;
let value: Number = id(42);
print(value + 1);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}
