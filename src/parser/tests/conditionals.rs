use crate::lexer::Lexer;
use crate::parser::{
    Parser,
    expression::{BinaryOp, Expr, Literal, Program, Statement},
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
fn parses_if_else_expression() {
    let program = parse_program(r#"if (x > 0) "positive" else "zero";"#);
    assert_eq!(program.statements.len(), 1);

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    let Expr::If(if_expr) = value else {
        panic!("expected if expression");
    };

    let Expr::Binary(condition) = if_expr.condition.as_ref() else {
        panic!("expected binary condition");
    };
    assert!(matches!(condition.op, BinaryOp::Greater));

    assert!(if_expr.elif_branches.is_empty());
    assert!(matches!(
        if_expr.then_branch.as_ref(),
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "positive"
    ));
    assert!(matches!(
        if_expr.else_branch.as_ref(),
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "zero"
    ));
}

#[test]
fn parses_if_else_with_block_branch() {
    let program = parse_program(
        r#"
if (x > 0) {
    print(x);
    "positive"
} else "zero";
"#,
    );

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    let Expr::If(if_expr) = value else {
        panic!("expected if expression");
    };

    let Expr::Block(block) = if_expr.then_branch.as_ref() else {
        panic!("expected block then branch");
    };

    assert_eq!(block.statements.len(), 2);
    assert!(matches!(
        block.statements[0],
        Statement::Expr { .. } | Statement::Print { .. }
    ));
    assert!(matches!(
        block.statements[1],
        Statement::Expr {
            value: Expr::Literal {
                value: Literal::String(_),
                ..
            },
            ..
        }
    ));
}

#[test]
fn parses_if_elif_else_expression() {
    let program = parse_program(
        r#"
if (x > 0) "positive"
elif (x == 0) "zero"
else "negative";
"#,
    );

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    let Expr::If(if_expr) = value else {
        panic!("expected if expression");
    };

    assert_eq!(if_expr.elif_branches.len(), 1);
    let elif_branch = &if_expr.elif_branches[0];

    let Expr::Binary(condition) = &elif_branch.condition else {
        panic!("expected binary elif condition");
    };
    assert!(matches!(condition.op, BinaryOp::Equal));

    assert!(matches!(
        &elif_branch.body,
        Expr::Literal {
            value: Literal::String(text),
            ..
        } if text == "zero"
    ));
}
