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
