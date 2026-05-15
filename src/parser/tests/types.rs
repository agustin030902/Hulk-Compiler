use super::parse_program;
use crate::parser::expression::{Expr, Statement};

#[test]
fn parses_type_declaration_with_member_access_and_member_assign() {
    let program = parse_program(
        r#"
type Point(x: Number, y: Number) {
    x: Number = x;
    y: Number = y;
    getX(): Number => self.x;
    setX(value: Number) => self.x := value;
}
42;
"#,
    );

    assert_eq!(program.type_decls.len(), 1);
    let point = &program.type_decls[0];
    assert_eq!(point.name, "Point");
    assert_eq!(point.params.len(), 2);
    assert_eq!(point.attributes.len(), 2);
    assert_eq!(point.methods.len(), 2);

    let get_x = &point.methods[0];
    let Expr::MemberAccess(access) = &get_x.body else {
        panic!("expected getX body to be member access");
    };
    assert_eq!(access.member, "x");
    assert!(matches!(
        access.instance.as_ref(),
        Expr::Variable { name, .. } if name == "self"
    ));

    let set_x = &point.methods[1];
    let Expr::MemberAssign(assign) = &set_x.body else {
        panic!("expected setX body to be member assignment");
    };
    assert_eq!(assign.member, "x");
}

#[test]
fn parses_new_expression_and_method_call_in_let_in() {
    let program = parse_program(
        r#"
type Box(value: Number) {
    value: Number = value;
    get(): Number => self.value;
}
let b = new Box(10) in b.get();
"#,
    );

    assert_eq!(program.statements.len(), 1);
    let Statement::Expr { value, .. } = &program.statements[0] else {
        panic!("expected expression statement");
    };

    let Expr::LetIn(let_in) = value else {
        panic!("expected let-in expression");
    };
    assert_eq!(let_in.bindings.len(), 1);
    assert!(matches!(let_in.bindings[0].value, Expr::New(_)));
    assert!(matches!(let_in.body.as_ref(), Expr::MethodCall(_)));
}
