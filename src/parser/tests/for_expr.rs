use crate::lexer::Lexer;
use crate::parser::{
    Parser,
    expression::{Expr, LetInExpr, Program, Statement, WhileExpr},
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
fn parses_for_loop_desugars_to_let_in_while() {
    let program = parse_program(r#"for (x in range(0, 5)) { print(x); };"#);

    assert_eq!(program.statements.len(), 1);

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    // for is desugared to: let __hulk_iter__ = <iter> in while (cond) let x = current() in body
    let Expr::LetIn(LetInExpr { bindings, body, .. }) = value else {
        panic!("expected let-in (outer desugaring), got: {:?}", value);
    };
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].name, "__hulk_iter__");

    let Expr::While(WhileExpr { condition, body, .. }) = body.as_ref() else {
        panic!("expected while loop as body of outer let-in");
    };

    // condition should be __hulk_iter__.next()
    let Expr::MethodCall(call) = condition.as_ref() else {
        panic!("expected method call for while condition");
    };
    assert_eq!(call.method_name, "next");

    // while body should have inner let-in: let x = __hulk_iter__.current() in body
    assert_eq!(body.statements.len(), 1);
    let Statement::Expr { value: inner, .. } = &body.statements[0] else {
        panic!("expected expression statement in while body");
    };
    let Expr::LetIn(LetInExpr {
        bindings: inner_bindings,
        body: inner_body,
        ..
    }) = inner
    else {
        panic!("expected inner let-in binding for loop variable");
    };
    assert_eq!(inner_bindings.len(), 1);
    assert_eq!(inner_bindings[0].name, "x");

    // inner let-in body should be a block with print(x)
    let Expr::Block(block) = inner_body.as_ref() else {
        panic!("expected block body");
    };
    assert_eq!(block.statements.len(), 1);
}

#[test]
fn parses_for_loop_with_different_variable_name() {
    let program = parse_program(r#"for (i in range(1, 11)) { print(i); };"#);

    assert_eq!(program.statements.len(), 1);
    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };
    let Expr::LetIn(LetInExpr { body, .. }) = value else {
        panic!("expected let-in");
    };
    let Expr::While(WhileExpr { body, .. }) = body.as_ref() else {
        panic!("expected while");
    };
    let Statement::Expr { value: inner, .. } = &body.statements[0] else {
        panic!("expected expression statement");
    };
    let Expr::LetIn(LetInExpr { bindings, .. }) = inner else {
        panic!("expected inner let-in");
    };
    assert_eq!(bindings[0].name, "i");
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
    let Expr::LetIn(LetInExpr { body, .. }) = value else {
        panic!("expected let-in");
    };
    let Expr::While(WhileExpr { body, .. }) = body.as_ref() else {
        panic!("expected while");
    };
    let Statement::Expr { value: inner, .. } = &body.statements[0] else {
        panic!("expected expression statement");
    };
    let Expr::LetIn(LetInExpr { body: inner_body, .. }) = inner else {
        panic!("expected inner let-in");
    };
    // The inner body should be a block with 2 statements
    let Expr::Block(block) = inner_body.as_ref() else {
        panic!("expected block body");
    };
    assert_eq!(block.statements.len(), 2);
}

#[test]
fn rejects_for_loop_with_non_identifier_variable() {
    // 'for (5 in range(0, 3)) { print(x); };' should produce a parse error
    let mut lexer = Lexer::new(r#"for (5 in range(0, 3)) { print(x); };"#.to_string());
    let tokens = lexer.lex();
    let mut parser = Parser::new(r#"for (5 in range(0, 3)) { print(x); };"#);
    let result = parser.parse_program(tokens);
    assert!(
        parser.has_errors() || result.is_none(),
        "expected parse error for non-identifier loop variable"
    );
}
