use crate::lexer::Lexer;
use crate::parser::{
    Parser,
    expression::{BinaryOp, Expr, Program, Statement},
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
fn parses_while_expression_with_block_body() {
    let program = parse_program(
        r#"
while (x < 3) {
    print(x);
    x = x + 1;
};
"#,
    );

    assert_eq!(program.statements.len(), 1);

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    let Expr::While(while_expr) = value else {
        panic!("expected while expression");
    };

    let Expr::Binary(condition) = while_expr.condition.as_ref() else {
        panic!("expected binary condition");
    };
    assert!(matches!(condition.op, BinaryOp::Less));

    assert_eq!(while_expr.body.statements.len(), 2);
    assert!(matches!(
        while_expr.body.statements[0],
        Statement::Expr { .. } | Statement::Print { .. }
    ));
    assert!(matches!(
        while_expr.body.statements[1],
        Statement::Assign { .. }
    ));
}
