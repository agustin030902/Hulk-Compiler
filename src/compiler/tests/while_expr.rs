use std::fs;

use super::{CompileOptions, Compiler, OutputKind, unique_output_path};

#[test]
fn compiles_while_expression_to_llvm_ir() {
    let source = r#"
let i = 0;
let loop_result = while (i < 3) {
    print(i);
    i = i + 1;
};
loop_result;
print(i);
"#;
    let output_path = unique_output_path("while_expr");

    let mut compiler = Compiler::new();
    let report = compiler.compile(
        source,
        &CompileOptions {
            output_path: output_path.clone(),
        },
    );

    assert!(
        report.errors.is_empty(),
        "expected successful compilation, got {:?}",
        report.errors
    );
    assert_eq!(report.output_kind, Some(OutputKind::LlvmIr));

    let ir = fs::read_to_string(&output_path).expect("should write llvm ir");
    assert!(
        ir.contains("while.cond."),
        "IR should include while condition label, got:\n{}",
        ir
    );
    assert!(
        ir.contains("br i1"),
        "IR should branch on the while condition, got:\n{}",
        ir
    );
    assert!(
        ir.contains("alloca i8"),
        "IR should allocate Unit storage for loop_result, got:\n{}",
        ir
    );
}

#[test]
fn writes_diagnostics_for_invalid_while_condition() {
    let source = r#"
while (1) {
    print(1);
};
"#;
    let output_path = unique_output_path("while_invalid_condition");

    let mut compiler = Compiler::new();
    let report = compiler.compile(
        source,
        &CompileOptions {
            output_path: output_path.clone(),
        },
    );

    assert!(!report.errors.is_empty());
    assert_eq!(report.output_kind, Some(OutputKind::Diagnostics));

    let diagnostics = fs::read_to_string(&output_path).expect("should write diagnostics output");
    assert!(
        diagnostics.contains("While condition expects Boolean, but got Number."),
        "diagnostics should contain while type error, got:\n{}",
        diagnostics
    );
}
