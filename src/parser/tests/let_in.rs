use crate::lexer::Lexer;
use crate::parser::{
    Parser,
    expression::{BinaryOp, Expr, LetBinding, Statement},
};

fn parse_program(source: &str) -> crate::parser::expression::Program {
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
fn parses_let_in_expression_with_single_binding() {
    let program = parse_program("let x = 7 in x;");
    assert_eq!(program.statements.len(), 1);

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    let Expr::LetIn(let_in) = value else {
        panic!("expected let-in expression");
    };
    assert_eq!(let_in.bindings.len(), 1);
    assert_eq!(let_in.bindings[0].name, "x");
    assert!(matches!(
        let_in.body.as_ref(),
        Expr::Variable { name, .. } if name == "x"
    ));
}

#[test]
fn parses_let_in_with_multiple_bindings_and_body_expr() {
    let program = parse_program("let a = 9, b = 5, c = true in { print(a + b); a + b };");
    assert_eq!(program.statements.len(), 1);

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    let Expr::LetIn(let_in) = value else {
        panic!("expected let-in expression");
    };
    assert_eq!(let_in.bindings.len(), 3);
    assert!(matches!(let_in.bindings[0], LetBinding { ref name, .. } if name == "a"));
    assert!(matches!(let_in.bindings[1], LetBinding { ref name, .. } if name == "b"));
    assert!(matches!(let_in.bindings[2], LetBinding { ref name, .. } if name == "c"));

    let Expr::Block(block) = let_in.body.as_ref() else {
        panic!("expected block body in let-in");
    };
    assert_eq!(block.statements.len(), 2);

    let value = match &block.statements[0] {
        crate::parser::expression::Statement::Expr { value, .. } => value,
        crate::parser::expression::Statement::Print { value, .. } => value,
        _ => panic!("expected first statement to be print"),
    };
    let value = match value {
        Expr::BuiltinCall(call)
            if matches!(
                call.function,
                crate::parser::expression::BuiltinFunction::Print
            ) =>
        {
            call.args.first().expect("print should have arg")
        }
        other => other,
    };
    assert!(matches!(value, Expr::Binary(bin) if matches!(bin.op, BinaryOp::Add)));
}

#[test]
fn parses_right_associative_let_in_chain() {
    let program = parse_program("let a = 1 in let b = 2 in a + b;");
    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    let Expr::LetIn(outer) = value else {
        panic!("expected outer let-in");
    };
    let Expr::LetIn(inner) = outer.body.as_ref() else {
        panic!("expected inner let-in");
    };

    assert_eq!(outer.bindings[0].name, "a");
    assert_eq!(inner.bindings[0].name, "b");
    assert!(matches!(
        inner.body.as_ref(),
        Expr::Binary(bin) if matches!(bin.op, BinaryOp::Add)
    ));
}
