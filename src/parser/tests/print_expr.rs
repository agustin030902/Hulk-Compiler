use crate::lexer::Lexer;
use crate::parser::{
    Parser,
    expression::{BuiltinFunction, Expr, Program, Statement},
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
fn parses_print_as_expression_inside_let_in() {
    let program = parse_program("let x = print(5) in print(x);");
    assert_eq!(program.statements.len(), 1);

    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    let Expr::LetIn(let_in) = value else {
        panic!("expected let-in expression");
    };

    let Expr::BuiltinCall(bind_call) = &let_in.bindings[0].value else {
        panic!("expected print builtin in binding");
    };
    assert!(matches!(bind_call.function, BuiltinFunction::Print));

    let Expr::BuiltinCall(body_call) = let_in.body.as_ref() else {
        panic!("expected print builtin in body");
    };
    assert!(matches!(body_call.function, BuiltinFunction::Print));
}
