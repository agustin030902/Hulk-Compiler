use crate::{codegen::CodegenBackend, error::CompilerError, lexer::Lexer, parser::Parser};

use super::LlvmBackend;

fn compile_source(source: &str) -> Result<String, Vec<CompilerError>> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.lex();
    assert!(
        lexer.errors().is_empty(),
        "lexer produced errors: {:?}",
        lexer.errors()
    );

    let mut parser = Parser::new(source);
    let program = parser
        .parse_program(tokens)
        .expect("parser should return a program");

    let mut backend = LlvmBackend::new();
    backend.generate(&program)
}

#[test]
fn generates_ir_for_recursive_function_declaration_and_call() {
    let source = r#"
function fact(n) => if (n == 0) 1 else n * fact(n - 1);
print(fact(5));
"#;
    let ir = compile_source(source).expect("codegen should succeed");

    assert!(
        ir.contains("define double @hulk_fact(double %n)"),
        "expected function definition, got:\n{}",
        ir
    );
    assert!(
        ir.contains("call double @hulk_fact(double"),
        "expected recursive/user function call, got:\n{}",
        ir
    );
    assert!(
        ir.contains("define i32 @main()"),
        "expected main after function definitions, got:\n{}",
        ir
    );
}

#[test]
fn generates_ir_for_string_function_signature_and_call() {
    let source = r#"
function greet(name) => "hi " @ name;
print(greet("ana"));
"#;
    let ir = compile_source(source).expect("codegen should succeed");

    assert!(
        ir.contains("define i8* @hulk_greet(i8* %name)"),
        "expected string function definition, got:\n{}",
        ir
    );
    assert!(
        ir.contains("call i8* @hulk_greet(i8*"),
        "expected string function call, got:\n{}",
        ir
    );
}

#[test]
fn infers_identity_signature_from_return_context() {
    let source = r#"
function id(x) => x;
function plus_one(y) => id(y) + 1;
print(plus_one(41));
"#;
    let ir = compile_source(source).expect("codegen should succeed");

    assert!(
        ir.contains("define double @hulk_id(double %x)"),
        "expected inferred numeric signature for id, got:\n{}",
        ir
    );
    assert!(
        ir.contains("call double @hulk_id(double"),
        "expected typed call to id, got:\n{}",
        ir
    );
}

#[test]
fn generates_ir_for_recursive_string_function() {
    let source = r#"
function stars(n) => if (n == 0) "" else stars(n - 1) @ "*";
print(stars(5));
"#;
    let ir = compile_source(source).expect("codegen should succeed");

    assert!(
        ir.contains("define i8* @hulk_stars(double %n)"),
        "expected recursive string function signature, got:\n{}",
        ir
    );
    assert!(
        ir.contains("call i8* @hulk_stars(double"),
        "expected recursive typed string call, got:\n{}",
        ir
    );
}

#[test]
fn generates_ir_for_block_expression_scope() {
    let source = "let y = 1; let x = { let x = 9; let z = 1; x + y }; print(x)";
    let ir = compile_source(source).expect("codegen should succeed");

    assert!(
        ir.contains("fadd double"),
        "expected block result to include addition"
    );
    assert!(
        ir.contains("@printf"),
        "expected printf call for print statement"
    );
}

#[test]
fn generates_ir_for_while_expression_and_unit_storage() {
    let source = r#"
let i = 0;
let loop_result = while (i < 2) {
    i = i + 1;
};
print(i);
"#;
    let ir = compile_source(source).expect("codegen should succeed");

    assert!(
        ir.contains("while.cond."),
        "expected condition label for while loop, got:\n{}",
        ir
    );
    assert!(
        ir.contains("while.body."),
        "expected body label for while loop, got:\n{}",
        ir
    );
    assert!(
        ir.contains("while.end."),
        "expected exit label for while loop, got:\n{}",
        ir
    );
    assert!(
        ir.contains("alloca i8"),
        "expected Unit storage for loop_result, got:\n{}",
        ir
    );
}

#[test]
fn generates_ir_for_simple_if_else() {
    let source = r#"
let a = 42 in 
if (a > 40) 
  print("Greater") 
else 
  print("Less");
"#;
    let ir = compile_source(source).expect("codegen should succeed");

    assert!(
        ir.contains("br i1"),
        "expected conditional branch (br i1) for if expression, got:\n{}",
        ir
    );
    assert!(
        ir.contains("if.then."),
        "expected then label for if expression, got:\n{}",
        ir
    );
    assert!(
        ir.contains("if.else."),
        "expected else label for if expression, got:\n{}",
        ir
    );
    assert!(
        ir.contains("if.end."),
        "expected end label for if expression, got:\n{}",
        ir
    );
}

#[test]
fn generates_ir_for_if_as_expression() {
    let source = r#"
let a = 42 in 
print(
  if (a > 40) 
    "greater" 
  else 
    "less"
);
"#;
    let ir = compile_source(source).expect("codegen should succeed");

    assert!(
        ir.contains("@printf"),
        "expected printf call for print statement, got:\n{}",
        ir
    );
    assert!(
        ir.contains("if.then."),
        "expected if.then label, got:\n{}",
        ir
    );
}

#[test]
fn generates_ir_for_if_elif_else() {
    let source = r#"
let a = 3 in
  print(
    if (a == 0) 
      "Zero"
    elif (a == 1) 
      "One"
    else 
      "Other"
  );
"#;
    let ir = compile_source(source).expect("codegen should succeed");

    assert!(
        ir.contains("if.elif."),
        "expected elif label for if expression with elif, got:\n{}",
        ir
    );
    assert!(
        ir.contains("if.elif.then."),
        "expected elif.then label for elif branch, got:\n{}",
        ir
    );
}

#[test]
fn generates_ir_for_if_multiple_elif() {
    let source = r#"
let a = 5 in
  print(
    if (a < 0)
      "Negative"
    elif (a == 0)
      "Zero"
    elif (a < 10)
      "Single digit"
    else
      "Greater than 10"
  );
"#;
    let ir = compile_source(source).expect("codegen should succeed");

    // Count the number of elif branches
    let elif_count = ir.matches("if.elif.").count();
    assert!(
        elif_count >= 2,
        "expected at least 2 elif labels for multiple elif branches, got:\n{}",
        ir
    );
    assert!(
        ir.contains("if.else."),
        "expected else label for multiple elif, got:\n{}",
        ir
    );
}

#[test]
fn generates_ir_for_type_instantiation_and_method_calls() {
    let source = r#"
type Point(x: Number, y: Number) {
    x = x;
    y = y;
    norm() => sqrt(self.x ^ 2 + self.y ^ 2);
    describe() => "(" @ self.x @ ", " @ self.y @ ")";
}

let p = new Point(3, 4);
print(p.describe() @ " norm=" @ p.norm());
"#;
    let ir = compile_source(source).expect("codegen should succeed");

    assert!(
        ir.contains("call i8* @malloc(i64"),
        "expected heap allocation for object instantiation, got:\n{}",
        ir
    );
    assert!(
        ir.contains("norm(i8* %self)"),
        "expected method definition with receiver parameter, got:\n{}",
        ir
    );
    assert!(
        ir.contains("describe(i8* %self)"),
        "expected second method definition with receiver parameter, got:\n{}",
        ir
    );
}

#[test]
fn generates_ir_for_method_using_another_struct_argument() {
    let source = r#"
type Point(x: Number, y: Number) {
    x = x;
    y = y;
    add(other: Point) => new Point(self.x + other.x, self.y + other.y);
    getX() => self.x;
}

let p1 = new Point(1, 2);
let p2 = new Point(3, 4);
let p3 = p1.add(p2);
print(p3.getX());
"#;
    let ir = compile_source(source).expect("codegen should succeed");

    assert!(
        ir.contains("add(i8* %self, i8* %other)"),
        "expected typed method signature for struct argument, got:\n{}",
        ir
    );
    assert!(
        ir.contains("call i8* @hulk_type"),
        "expected method call returning struct value, got:\n{}",
        ir
    );
}

#[test]
fn generates_ir_for_inherited_fields_and_methods() {
    let source = r#"
type Entity(name: String) {
    name = name;
    label() => self.name;
}

type Player(name: String, score: Number) inherits Entity(name) {
    score = score;
    describe() => self.label() @ ":" @ self.score;
}

let player = new Player("Ada", 42);
print(player.describe());
"#;
    let ir = compile_source(source).expect("codegen should succeed");

    assert!(
        ir.contains("_label(i8* %self)"),
        "expected inherited parent method definition, got:\n{}",
        ir
    );
    assert!(
        ir.contains("_describe(i8* %self)"),
        "expected child method definition, got:\n{}",
        ir
    );
    assert!(
        ir.contains("call i8* @hulk_type") && ir.contains("_label(i8*"),
        "expected child method to call inherited method, got:\n{}",
        ir
    );
}
