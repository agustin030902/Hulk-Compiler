use std::fs;

use super::{CompileOptions, Compiler, OutputKind, unique_output_path};

#[test]
fn writes_llvm_ir_for_let_in() {
    let source = r#"
let a = 1;
let b = let x = 9, y = 1 in x + y;
print(a);
print(b);
"#;
    let output_path = unique_output_path("let_in_ir");

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
        llvm_ir.contains("alloca"),
        "IR should allocate storage for let-in bindings, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_diagnostics_for_let_in_type_error() {
    let source = r#"
let bad = let a = true in a + 1;
print(bad);
"#;
    let output_path = unique_output_path("let_in_type_error");

    let mut compiler = Compiler::new();
    let report = compiler.compile(
        source,
        &CompileOptions {
            output_path: output_path.clone(),
        },
    );

    assert!(!report.errors.is_empty());
    assert_eq!(report.output_kind, Some(OutputKind::Diagnostics));

    let diagnostics = fs::read_to_string(&output_path)
        .expect("compiler should write diagnostics output file on error");
    assert!(
        diagnostics.contains("Operator '+' expects Number and Number"),
        "diagnostics file should contain type error, got:\n{}",
        diagnostics
    );
}
