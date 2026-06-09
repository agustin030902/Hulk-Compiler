use crate::lexer::Lexer;

use super::{parse_program, Expr, Statement};

#[test]
fn parses_is_expression() {
    let program = parse_program("let x = new Bird(); print(x is Animal);");

    let Statement::Let { value, .. } = &program.statements[0] else {
        panic!("expected let statement");
    };
    let Expr::New(new_expr) = value else {
        panic!("expected new expression");
    };
    assert_eq!(new_expr.type_name, "Bird");

    let Statement::Expr { value, .. } = &program.statements[1] else {
        panic!("expected expression statement");
    };
    let Expr::BuiltinCall(print_call) = value else {
        panic!("expected print call");
    };
    let Expr::Is(is_expr) = &print_call.args[0] else {
        panic!("expected is expression");
    };
    assert_eq!(is_expr.target_type, "Animal");
}

#[test]
fn parses_as_expression() {
    let program = parse_program("let x = new Bird(); let y = x as Animal;");

    let Statement::Let { value, .. } = &program.statements[1] else {
        panic!("expected let statement for y");
    };
    let Expr::As(as_expr) = value else {
        panic!("expected as expression");
    };
    assert_eq!(as_expr.target_type, "Animal");
}

#[test]
fn parses_is_with_complex_expression() {
    let program = parse_program("print(new Bird() is Animal);");

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };
    let Expr::BuiltinCall(print_call) = value else {
        panic!("expected print call");
    };
    let Expr::Is(is_expr) = &print_call.args[0] else {
        panic!("expected is expression");
    };
    assert_eq!(is_expr.target_type, "Animal");
}

#[test]
fn parses_as_with_complex_expression() {
    let program = parse_program("print(new Bird() as Animal);");

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };
    let Expr::BuiltinCall(print_call) = value else {
        panic!("expected print call");
    };
    let Expr::As(as_expr) = &print_call.args[0] else {
        panic!("expected as expression");
    };
    assert_eq!(as_expr.target_type, "Animal");
}

#[test]
fn parses_is_with_equality_precedence() {
    let program = parse_program("print(x is Bird == true);");

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };
    let Expr::BuiltinCall(print_call) = value else {
        panic!("expected print call");
    };
    let Expr::Binary(binary) = &print_call.args[0] else {
        panic!("expected binary expression");
    };
    assert_eq!(format!("{:?}", binary.op), "Equal");
    let Expr::Is(is_expr) = &*binary.left else {
        panic!("expected is expression on left side");
    };
    assert_eq!(is_expr.target_type, "Bird");
}

#[test]
fn parses_as_with_higher_precedence_than_is() {
    let program = parse_program("print(x as Bird is Animal);");

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };
    let Expr::BuiltinCall(print_call) = value else {
        panic!("expected print call");
    };
    let Expr::Is(is_expr) = &print_call.args[0] else {
        panic!("expected is expression at top level");
    };
    assert_eq!(is_expr.target_type, "Animal");
    let Expr::As(as_expr) = &*is_expr.expr else {
        panic!("expected as expression on left side of is");
    };
    assert_eq!(as_expr.target_type, "Bird");
}
