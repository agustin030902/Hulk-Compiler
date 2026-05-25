use crate::lexer::Lexer;
use crate::parser::{
    Parser,
    expression::{AssignTarget, Expr, Program},
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

fn parse_error_message(source: &str) -> String {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.lex();
    assert!(
        !lexer.has_errors(),
        "lexer produced errors: {:?}",
        lexer.errors()
    );

    let mut parser = Parser::new(source);
    let program = parser.parse_program(tokens);
    assert!(program.is_none(), "program should fail parsing");
    assert!(parser.has_errors(), "parser should report syntax errors");

    parser.errors()[0].message.clone()
}

#[test]
fn parses_type_declaration_and_instantiation_flow() {
    let source = r#"
type Point(x: Number, y: Number) {
    x = x;
    y = y;
    add(other: Point) => new Point(self.x + other.x, self.y + other.y);
    describe() => "(" @ self.x @ "," @ self.y @ ")";
}

let p1 = new Point(1, 2);
let p2 = p1.add(p1);
print(p2.describe());
"#;

    let program = parse_program(source);

    assert_eq!(program.types.len(), 1);
    assert_eq!(program.types[0].name, "Point");
    assert_eq!(program.types[0].params.len(), 2);
    assert_eq!(program.types[0].attributes.len(), 2);
    assert_eq!(program.types[0].methods.len(), 2);

    let add = &program.types[0].methods[0];
    assert_eq!(add.name, "add");
    let Expr::New(new_expr) = &add.body else {
        panic!("expected add body to be new expression");
    };
    assert_eq!(new_expr.type_name, "Point");

    let stmt = &program.statements[1];
    let crate::parser::expression::Statement::Let { value, .. } = stmt else {
        panic!("expected let statement");
    };
    assert!(matches!(value, Expr::MethodCall(_)));
}

#[test]
fn parses_destructive_assign_to_member_inside_method() {
    let source = r#"
type Box(v: Number) {
    v = v;
    setV(next: Number) => self.v := next;
}
"#;

    let program = parse_program(source);
    let method = &program.types[0].methods[0];

    let Expr::DestructiveAssign(assign) = &method.body else {
        panic!("expected destructive assignment in method body");
    };

    assert!(matches!(
        &assign.target,
        AssignTarget::Member { member, .. } if member == "v"
    ));
}

#[test]
fn parses_type_declaration_with_inheritance_initializer() {
    let source = r#"
type Animal(name: String) {
    name = name;
    label() => self.name;
}

type Dog(name: String, age: Number) inherits Animal(name) {
    age = age;
    speak() => "woof";
}
"#;

    let program = parse_program(source);
    let dog = &program.types[1];

    assert_eq!(dog.name, "Dog");
    assert_eq!(dog.parent_name.as_deref(), Some("Animal"));
    assert!(dog.parent_span.is_some());
    assert_eq!(dog.parent_init_exprs.len(), 1);
    assert_eq!(dog.attributes.len(), 1);
    assert_eq!(dog.methods.len(), 1);
}

#[test]
fn reports_error_for_invalid_destructive_assignment_target() {
    let message = parse_error_message("let x = (a + b) := 1;");
    assert!(message.contains("Invalid assignment target for ':='"));
}
