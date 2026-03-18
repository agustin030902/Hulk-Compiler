use std::fs;

use super::{CompileOptions, Compiler, OutputKind, unique_output_path};

#[test]
fn writes_ir_for_destructive_assignment() {
    let source = r#"
let a = 0;
a := 1;
print(a);
"#;
    let output_path = unique_output_path("destructive_assign_ir");

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

    let ir = fs::read_to_string(&output_path).expect("should write llvm ir");
    assert!(
        ir.contains("store double"),
        "IR should contain a store for ':=', got:\n{}",
        ir
    );
}

#[test]
fn writes_diagnostics_on_type_mismatch() {
    let source = r#"
let a = 1;
a := true;
"#;
    let output_path = unique_output_path("destructive_assign_type_error");

    let mut compiler = Compiler::new();
    let report = compiler.compile(
        source,
        &CompileOptions {
            output_path: output_path.clone(),
        },
    );

    assert!(!report.errors.is_empty());
    assert_eq!(report.output_kind, Some(OutputKind::Diagnostics));

    let diagnostics = fs::read_to_string(&output_path).expect("should write diagnostics on error");
    assert!(
        diagnostics.contains("Destructive assignment ':=' requires the same type"),
        "unexpected diagnostics:\n{}",
        diagnostics
    );
}
