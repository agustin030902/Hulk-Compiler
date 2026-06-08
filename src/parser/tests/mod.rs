mod conditionals;
mod destructive_assign;
mod for_expr;
mod function_type_annotations;
mod is_as_expr;
mod let_in;
mod print_expr;
mod string_escape;
mod type_annotations;
mod types;
mod while_expr;

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

fn unwrap_print_arg<'a>(expr: &'a Expr) -> &'a Expr {
    if let Expr::BuiltinCall(call) = expr {
        if matches!(call.function, BuiltinFunction::Print) {
            return call.args.first().expect("print call should have one arg");
        }
    }
    expr
}

#[test]
fn parses_function_declaration_and_recursive_call() {
    let program =
        parse_program("function fact(n) => if (n == 0) 1 else n * fact(n - 1); print(fact(5));");

    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "fact");
    assert_eq!(program.functions[0].params.len(), 1);
    assert_eq!(program.functions[0].params[0].name, "n");

    let Expr::If(if_expr) = &program.functions[0].body else {
        panic!("expected function body to be an if expression");
    };
    let Expr::Binary(binary) = if_expr.else_branch.as_ref() else {
        panic!("expected recursive multiplication in else branch");
    };
    assert!(matches!(binary.op, BinaryOp::Mul));
    assert!(matches!(
        binary.right.as_ref(),
        Expr::FunctionCall(call) if call.name == "fact" && call.args.len() == 1
    ));

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected print expression statement");
    };
    let Expr::BuiltinCall(print_call) = value else {
        panic!("expected print call");
    };
    assert!(matches!(
        print_call.args.first(),
        Some(Expr::FunctionCall(call)) if call.name == "fact" && call.args.len() == 1
    ));
}

#[test]
fn parses_string_concat_with_number() {
    let program = parse_program(r#"print("The meaning of life is " @ 42);"#);

    assert_eq!(program.statements.len(), 1);
    let value = match &program.statements[0] {
        Statement::Expr { value, .. } => value,
        Statement::Print { value, .. } => value,
        _ => panic!("expected print statement"),
    };
    let value = unwrap_print_arg(value);
    let value = unwrap_print_arg(value);

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
    let value = match &program.statements[0] {
        Statement::Expr { value, .. } => value,
        Statement::Print { value, .. } => value,
        _ => panic!("expected print statement"),
    };
    let value = unwrap_print_arg(value);
    let value = unwrap_print_arg(value);

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
fn parses_concat_space_operator() {
    let program = parse_program(r#"print("hello" @@ "world");"#);

    assert_eq!(program.statements.len(), 1);
    let value = match &program.statements[0] {
        Statement::Expr { value, .. } => value,
        Statement::Print { value, .. } => value,
        _ => panic!("expected print statement"),
    };
    let value = unwrap_print_arg(value);
    let value = unwrap_print_arg(value);

    let Expr::Binary(binary) = value else {
        panic!("expected binary expression");
    };

    assert!(matches!(binary.op, BinaryOp::ConcatSpace));
    assert!(matches!(
        binary.left.as_ref(),
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "hello"
    ));
    assert!(matches!(
        binary.right.as_ref(),
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "world"
    ));
}

#[test]
fn parses_logical_and_comparison_precedence() {
    let program = parse_program(r#"print(5 > 3 && 2 < 8 || !false);"#);

    let value = match &program.statements[0] {
        Statement::Expr { value, .. } => value,
        Statement::Print { value, .. } => value,
        _ => panic!("expected print statement"),
    };
    let value = unwrap_print_arg(value);

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

    let value = match &program.statements[0] {
        Statement::Expr { value, .. } => value,
        Statement::Print { value, .. } => value,
        _ => panic!("expected print statement"),
    };
    let value = unwrap_print_arg(value);

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
    assert!(matches!(
        &program.statements[2],
        Statement::Expr { .. } | Statement::Print { .. }
    ));
}

#[test]
fn parses_builtin_calls_with_primary_precedence() {
    let program = parse_program(r#"print(sin(2 + 1) * cos(0));"#);

    let value = match &program.statements[0] {
        Statement::Expr { value, .. } => value,
        Statement::Print { value, .. } => value,
        _ => panic!("expected print statement"),
    };
    let value = unwrap_print_arg(value);

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

    let value = match &program.statements[0] {
        Statement::Expr { value, .. } => value,
        Statement::Print { value, .. } => value,
        _ => panic!("expected print statement"),
    };
    let value = unwrap_print_arg(value);

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

    let value = match &program.statements[0] {
        Statement::Expr { value, .. } => value,
        Statement::Print { value, .. } => value,
        _ => panic!("expected print statement"),
    };
    let value = unwrap_print_arg(value);

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

    let value = match &program.statements[0] {
        Statement::Expr { value, .. } => value,
        Statement::Print { value, .. } => value,
        _ => panic!("expected print statement"),
    };
    let value = unwrap_print_arg(value);

    let Expr::Binary(outer_pow) = value else {
        panic!("expected top-level power expression");
    };
    assert!(matches!(outer_pow.op, BinaryOp::Pow));

    let Expr::Binary(inner_pow) = outer_pow.right.as_ref() else {
        panic!("expected right-nested power expression");
    };
    assert!(matches!(inner_pow.op, BinaryOp::Pow));
}

#[test]
fn parses_expression_statement_literal() {
    let program = parse_program("42;");
    assert_eq!(program.statements.len(), 1);

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    assert!(matches!(
        value,
        Expr::Literal {
            value: Literal::Integer(42),
            ..
        }
    ));
}

#[test]
fn parses_rand_builtin_call_without_arguments() {
    let program = parse_program("print(rand());");
    assert_eq!(program.statements.len(), 1);

    let value = match &program.statements[0] {
        Statement::Expr { value, .. } => value,
        Statement::Print { value, .. } => value,
        _ => panic!("expected print statement"),
    };
    let value = unwrap_print_arg(value);
    let Expr::BuiltinCall(call) = value else {
        panic!("expected builtin call");
    };

    assert_eq!(call.function, BuiltinFunction::Rand);
    assert!(call.args.is_empty());
}

#[test]
fn parses_program_without_trailing_semicolon() {
    let program = parse_program("let a = 1; print(a)");

    assert_eq!(program.statements.len(), 2);
    if let Statement::Let { name, .. } = &program.statements[0] {
        assert_eq!(name, "a");
    } else {
        panic!("expected let statement for a");
    }
    assert!(matches!(
        program.statements[1],
        Statement::Expr { .. } | Statement::Print { .. }
    ));
}

#[test]
fn parses_block_expression_and_scoping_shape() {
    let program = parse_program("let y = 1; let x = { let x = 9; let z = 1; x + y }");

    assert_eq!(program.statements.len(), 2);
    let Statement::Let { value, .. } = &program.statements[1] else {
        panic!("expected let binding for x");
    };

    let Expr::Block(block) = value else {
        panic!("expected block expression as initializer");
    };

    assert_eq!(block.statements.len(), 3);
    let Statement::Expr {
        value: final_expr, ..
    } = &block.statements[2]
    else {
        panic!("expected final expression inside block");
    };

    let Expr::Binary(add_expr) = final_expr else {
        panic!("expected addition as last expression");
    };
    assert!(matches!(add_expr.op, BinaryOp::Add));

    assert!(matches!(
        add_expr.left.as_ref(),
        Expr::Variable { name, .. } if name == "x"
    ));
    assert!(matches!(
        add_expr.right.as_ref(),
        Expr::Variable { name, .. } if name == "y"
    ));
}

#[test]
fn parses_null_literal_in_let_initializer() {
    let program = parse_program("let root = null;");
    assert_eq!(program.statements.len(), 1);

    let Statement::Let { value, .. } = &program.statements[0] else {
        panic!("expected let statement");
    };

    assert!(matches!(
        value,
        Expr::Literal {
            value: Literal::Null,
            ..
        }
    ));
}
