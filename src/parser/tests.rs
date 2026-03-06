use crate::lexer::Lexer;

use super::{
    Parser,
    expression::{BinaryOp, BuiltinFunction, Expr, Literal, Program, Statement, UnaryOp},
};

fn parse_program(source: &str) -> Program {
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

    program.expect("parser did not produce a program")
}

#[test]
fn parses_string_concat_with_number() {
    let program = parse_program(r#"print("The meaning of life is " @ 42);"#);

    assert_eq!(program.statements.len(), 1);
    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };

    let Expr::Binary(binary) = value else {
        panic!("expected binary expression");
    };

    assert!(matches!(binary.op, BinaryOp::Concat));
    assert!(matches!(
        binary.left.as_ref(),
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "The meaning of life is "
    ));
    assert!(matches!(
        binary.right.as_ref(),
        Expr::Literal {
            value: Literal::Integer(42),
            ..
        }
    ));
}

#[test]
fn parses_concat_as_left_associative() {
    let program = parse_program(r#"print("a" @ 1 @ "b");"#);

    assert_eq!(program.statements.len(), 1);
    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };

    let Expr::Binary(outer) = value else {
        panic!("expected outer binary expression");
    };
    assert!(matches!(outer.op, BinaryOp::Concat));

    assert!(matches!(
        outer.right.as_ref(),
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "b"
    ));

    let Expr::Binary(inner) = outer.left.as_ref() else {
        panic!("expected inner binary expression");
    };
    assert!(matches!(inner.op, BinaryOp::Concat));
    assert!(matches!(
        inner.left.as_ref(),
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "a"
    ));
    assert!(matches!(
        inner.right.as_ref(),
        Expr::Literal {
            value: Literal::Integer(1),
            ..
        }
    ));
}

#[test]
fn parses_logical_and_comparison_precedence() {
    let program = parse_program(r#"print(5 > 3 && 2 < 8 || !false);"#);

    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };

    let Expr::Binary(or_expr) = value else {
        panic!("expected top-level or binary expression");
    };
    assert!(matches!(or_expr.op, BinaryOp::Or));

    let Expr::Binary(and_expr) = or_expr.left.as_ref() else {
        panic!("expected and expression on the left side of or");
    };
    assert!(matches!(and_expr.op, BinaryOp::And));

    let Expr::Binary(gt_expr) = and_expr.left.as_ref() else {
        panic!("expected greater-than expression");
    };
    assert!(matches!(gt_expr.op, BinaryOp::Greater));

    let Expr::Binary(lt_expr) = and_expr.right.as_ref() else {
        panic!("expected less-than expression");
    };
    assert!(matches!(lt_expr.op, BinaryOp::Less));

    let Expr::Unary(not_expr) = or_expr.right.as_ref() else {
        panic!("expected unary not expression");
    };
    assert!(matches!(not_expr.op, UnaryOp::Not));
}

#[test]
fn parses_arithmetic_before_comparison() {
    let program = parse_program(r#"print(x + 5 > y * 2);"#);

    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };

    let Expr::Binary(cmp_expr) = value else {
        panic!("expected comparison expression");
    };
    assert!(matches!(cmp_expr.op, BinaryOp::Greater));

    let Expr::Binary(add_expr) = cmp_expr.left.as_ref() else {
        panic!("expected addition on left side of comparison");
    };
    assert!(matches!(add_expr.op, BinaryOp::Add));

    let Expr::Binary(mul_expr) = cmp_expr.right.as_ref() else {
        panic!("expected multiplication on right side of comparison");
    };
    assert!(matches!(mul_expr.op, BinaryOp::Mul));
}

#[test]
fn parses_reassignment_statement() {
    let program = parse_program(
        r#"
let x = 45;
x = true;
print(x);
"#,
    );

    assert_eq!(program.statements.len(), 3);

    assert!(matches!(
        &program.statements[0],
        Statement::Let { name, .. } if name == "x"
    ));
    assert!(matches!(
        &program.statements[1],
        Statement::Assign { name, .. } if name == "x"
    ));
    assert!(matches!(&program.statements[2], Statement::Print { .. }));
}

#[test]
fn parses_builtin_calls_with_primary_precedence() {
    let program = parse_program(r#"print(sin(2 + 1) * cos(0));"#);

    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };

    let Expr::Binary(mul_expr) = value else {
        panic!("expected multiplication at top level");
    };
    assert!(matches!(mul_expr.op, BinaryOp::Mul));

    let Expr::BuiltinCall(left_call) = mul_expr.left.as_ref() else {
        panic!("expected sin() call on left side");
    };
    assert_eq!(left_call.function, BuiltinFunction::Sin);

    let Expr::BuiltinCall(right_call) = mul_expr.right.as_ref() else {
        panic!("expected cos() call on right side");
    };
    assert_eq!(right_call.function, BuiltinFunction::Cos);
}

#[test]
fn parses_log_with_two_arguments() {
    let program = parse_program(r#"print(log(4, 64));"#);

    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };

    let Expr::BuiltinCall(call) = value else {
        panic!("expected builtin call");
    };

    assert_eq!(call.function, BuiltinFunction::Log);
    assert_eq!(call.args.len(), 2);
    assert!(matches!(
        &call.args[0],
        Expr::Literal {
            value: Literal::Integer(4),
            ..
        }
    ));
    assert!(matches!(
        &call.args[1],
        Expr::Literal {
            value: Literal::Integer(64),
            ..
        }
    ));
}

#[test]
fn parses_power_with_higher_precedence_than_mul_and_add() {
    let program = parse_program(r#"print(2 + 3 * 2 ^ 3);"#);

    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };

    let Expr::Binary(add_expr) = value else {
        panic!("expected top-level addition");
    };
    assert!(matches!(add_expr.op, BinaryOp::Add));

    let Expr::Binary(mul_expr) = add_expr.right.as_ref() else {
        panic!("expected multiplication on right side of addition");
    };
    assert!(matches!(mul_expr.op, BinaryOp::Mul));

    let Expr::Binary(pow_expr) = mul_expr.right.as_ref() else {
        panic!("expected power expression on right side of multiplication");
    };
    assert!(matches!(pow_expr.op, BinaryOp::Pow));
}

#[test]
fn parses_power_as_right_associative() {
    let program = parse_program(r#"print(2 ^ 3 ^ 2);"#);

    let Statement::Print { value, .. } = &program.statements[0] else {
        panic!("expected print statement");
    };

    let Expr::Binary(outer_pow) = value else {
        panic!("expected top-level power expression");
    };
    assert!(matches!(outer_pow.op, BinaryOp::Pow));

    let Expr::Binary(inner_pow) = outer_pow.right.as_ref() else {
        panic!("expected right-nested power expression");
    };
    assert!(matches!(inner_pow.op, BinaryOp::Pow));
}
