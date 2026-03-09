use std::fs;

use super::{CompileOptions, Compiler, OutputKind, unique_output_path};

#[test]
fn emits_rand_call_inside_expression_statement() {
    let source = r#"
rand();
print(rand());
"#;
    let output_path = unique_output_path("rand_expression_statement_ir");

    let mut compiler = Compiler::new();
    let report = compiler.compile(
        source,
        &CompileOptions {
            output_path: output_path.clone(),
        },
    );

    assert!(
        report.errors.is_empty(),
        "expected successful compilation, got errors: {:?}",
        report.errors
    );
    assert_eq!(report.output_kind, Some(OutputKind::LlvmIr));

    let llvm_ir = fs::read_to_string(&output_path)
        .expect("compiler should write llvm output file on success");
    assert!(
        llvm_ir.contains("call i32 @rand()"),
        "IR should contain rand calls for rand() expressions, got:\n{}",
        llvm_ir
    );
}

#[test]
fn compiles_program_with_only_expression_statement() {
    let source = "42;";
    let output_path = unique_output_path("only_expression_statement_ir");

    let mut compiler = Compiler::new();
    let report = compiler.compile(
        source,
        &CompileOptions {
            output_path: output_path.clone(),
        },
    );

    assert!(
        report.errors.is_empty(),
        "expected successful compilation, got errors: {:?}",
        report.errors
    );
    assert_eq!(report.output_kind, Some(OutputKind::LlvmIr));

    let llvm_ir = fs::read_to_string(&output_path)
        .expect("compiler should write llvm output file on success");
    assert!(
        llvm_ir.contains("define i32 @main()"),
        "IR should define main for expression-only program, got:\n{}",
        llvm_ir
    );
}
