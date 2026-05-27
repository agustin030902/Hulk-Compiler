mod conditionals;
mod destructive_assign;
mod function_type_annotations;
mod let_in;
mod print_expr;
mod string_escape;
mod type_annotations;
mod types;
mod types_namespace_integration;
mod while_expr;

use crate::{error::ErrorCategory, lexer::Lexer, parser::Parser};

use super::SemanticAnalyzer;

fn analyze_source(source: &str) -> Vec<crate::error::CompilerError> {
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
    analyzer.analyze(
        &program.expect("parser did not produce program for semantic analysis"),
        source,
    )
}

#[test]
fn allows_declared_recursive_function_calls_by_arity() {
    let source = r#"
function fact(n) => if (n == 0) 1 else n * fact(n - 1);
print(fact(5));
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_user_function_call_with_wrong_arity() {
    let source = r#"
function add_one(x) => x + 1;
print(add_one(1, 2));
"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Semantic);
    assert_eq!(
        errors[0].message,
        "Function 'add_one' expects 1 argument(s), but got 2."
    );
}

#[test]
fn infers_boolean_parameter_and_string_return_for_function() {
    let source = r#"
function label(flag) => if (flag) "yes" else "no";
print(label(true));
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_user_function_call_with_wrong_argument_type() {
    let source = r#"
function negate(flag) => !flag;
print(negate(1));
"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Function 'negate' argument #1 expects Boolean, but got Number."
    );
}

#[test]
fn infers_identity_function_from_return_context() {
    let source = r#"
function id(x) => x;
function plus_one(y) => id(y) + 1;
print(plus_one(41));
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn infers_recursive_string_function_return_type() {
    let source = r#"
function stars(n) => if (n == 0) "" else stars(n - 1) @ "*";
print(stars(5));
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn infers_recursive_return_type_from_if_branch_context() {
    let source = r#"
function mod(x, y) => if (x < y) x else mod(x - y, y);
function mcd(a, b) => if (b == 0) a else mcd(a, mod(a, b));
print(mcd(13, 5));
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn allows_concat_between_string_and_number_and_string() {
    let source = r#"
let a = "hello " @ 1;
let b = 2 @ "world";
let c = "foo" @ "bar";
let d = a @ "!";
print(d);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_concat_between_number_and_number_with_specific_error() {
    let source = r#"print(1 @ 2);"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Operator '@' expects (String, String), (String, Number), or (Number, String), but got Number and Number."
    );
}

#[test]
fn rejects_concat_between_boolean_and_string_with_specific_error() {
    let source = r#"print(true @ "x");"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Operator '@' expects (String, String), (String, Number), or (Number, String), but got Boolean and String."
    );
}

#[test]
fn allows_concat_space_between_string_number_and_string() {
    let source = r#"
let label = "score" @@ 42;
let message = label @@ "pts";
print(message);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_concat_space_between_number_and_number_with_specific_error() {
    let source = r#"print(1 @@ 2);"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Operator '@@' expects (String, String), (String, Number), or (Number, String), but got Number and Number."
    );
}

#[test]
fn allows_boolean_comparison_and_logic_expressions() {
    let source = r#"
let x = 10;
let y = 5;

print(x > y);
print(x == 10);
print(true && false);
print(!(x < y));
print((x + y) > 10);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_logical_operator_with_non_boolean_operands() {
    let source = r#"print(1 && true);"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "logical operator requires Boolean operands"
    );
}

#[test]
fn rejects_comparison_with_non_numeric_operands() {
    let source = r#"print("a" < "b");"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Comparison operator '<' expects Number and Number, but got String and String."
    );
}

#[test]
fn rejects_equality_with_mismatched_types() {
    let source = r#"print(1 == true);"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Operator '==' expects operands of the same type, but got Number and Boolean."
    );
}

#[test]
fn allows_reassignment_even_when_type_changes() {
    let source = r#"
let x = 45;
x = true;
print(x);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_assignment_before_declaration() {
    let source = r#"
x = 10;
print(x);
"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].category, ErrorCategory::Semantic);
    assert_eq!(
        errors[0].message,
        "Variable 'x' is assigned before declaration. Declare it with 'let' first."
    );
}

#[test]
fn allows_builtin_math_functions() {
    let source = r#"
let a = sin(PI);
let b = cos(E);
let c = sqrt(9);
let d = exp(1);
let e = log(4, 64);
print(a + b + c + d + e);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_log_with_invalid_argument_types() {
    let source = r#"print(log(2, "x"));"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Function 'log' expects (Number, Number), but got Number and String."
    );
}

#[test]
fn rejects_sin_with_non_numeric_argument() {
    let source = r#"print(sin(true));"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Function 'sin' expects Number, but got Boolean."
    );
}

#[test]
fn allows_power_with_numeric_operands() {
    let source = r#"
let result = 2 ^ 3 ^ 2;
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
fn rejects_power_with_non_numeric_operands() {
    let source = r#"print("x" ^ 2);"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].category, ErrorCategory::Type);
    assert_eq!(
        errors[0].message,
        "Operator '^' expects Number and Number, but got String and Number."
    );
}

#[test]
fn allows_expression_statement_without_side_effects() {
    let source = r#"
42;
let x = 1 + 2;
print(x);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn allows_rand_builtin_call() {
    let source = r#"
let r = rand();
print(r);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn allows_block_expression_and_shadows_inner_variables() {
    let source = r#"
let y = 1;
let x = { let x = 9; let z = 1; x + y };
print(x);
print(y);
"#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );
}

#[test]
fn rejects_access_to_inner_block_bindings() {
    let source = r#"
let outer = { let inner = 5; inner };
print(inner);
"#;

    let errors = analyze_source(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].message,
        "Variable 'inner' is used before declaration. Declare it with 'let' first."
    );
}
