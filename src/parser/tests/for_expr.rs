use crate::lexer::Lexer;
use crate::parser::{
    Parser,
    expression::{Expr, ForExpr, Program, Statement},
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
fn parses_for_loop_produces_for_expr() {
    let program = parse_program(r#"for (x in range(0, 5)) { print(x); };"#);

    assert_eq!(program.statements.len(), 1);

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    let Expr::For(ForExpr { id, iter, body, .. }) = value else {
        panic!("expected For expression, got: {:?}", value);
    };
    assert_eq!(id, "x");
    assert!(body.statements.len() > 0);

    let Expr::New(new_expr) = iter.as_ref() else {
        panic!("expected New expression for iter, got: {:?}", iter);
    };
    assert_eq!(new_expr.type_name, "Range");
}

#[test]
fn parses_for_loop_with_different_variable_name() {
    let program = parse_program(r#"for (i in range(1, 11)) { print(i); };"#);

    assert_eq!(program.statements.len(), 1);
    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };
    let Expr::For(ForExpr { id, .. }) = value else {
        panic!("expected For expression");
    };
    assert_eq!(id, "i");
}

#[test]
fn parses_range_as_new_expression() {
    let program = parse_program(r#"print(range(0, 5));"#);

    assert_eq!(program.statements.len(), 1);
    let value = match &program.statements[0] {
        Statement::Expr { value, .. } => value,
        Statement::Print { value, .. } => value,
        _ => panic!("expected print statement"),
    };

    let Expr::BuiltinCall(print_call) = value else {
        panic!("expected builtin print call");
    };
    let Expr::New(new_expr) = print_call.args.first().unwrap() else {
        panic!("expected new Range expression, got: {:?}", print_call.args.first());
    };
    assert_eq!(new_expr.type_name, "Range");
    assert_eq!(new_expr.args.len(), 2);
}

#[test]
fn parses_for_loop_with_block_body() {
    let program = parse_program(
        r#"
        for (x in range(0, 3)) {
            let y = x + 1;
            print(y);
        };
        "#,
    );

    assert_eq!(program.statements.len(), 1);
    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };
    let Expr::For(ForExpr { body, .. }) = value else {
        panic!("expected For expression");
    };
    assert_eq!(body.statements.len(), 2);
}

#[test]
fn rejects_for_loop_with_non_identifier_variable() {
    let mut lexer = Lexer::new(r#"for (5 in range(0, 3)) { print(x); };"#.to_string());
    let tokens = lexer.lex();
    let mut parser = Parser::new(r#"for (5 in range(0, 3)) { print(x); };"#);
    let result = parser.parse_program(tokens);
    assert!(
        parser.has_errors() || result.is_none(),
        "expected parse error for non-identifier loop variable"
    );
}
