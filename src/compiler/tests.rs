use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{CompileOptions, Compiler, OutputKind};

fn unique_output_path(test_name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("hulk_{test_name}_{stamp}.txt"))
}

#[test]
fn writes_diagnostics_txt_for_invalid_concat() {
    let source = r#"print(true @ false);"#;
    let output_path = unique_output_path("invalid_concat_diagnostics");

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
        diagnostics.contains(
            "Operator '@' expects (String, String), (String, Number), or (Number, String), but got Boolean and Boolean."
        ),
        "diagnostics file should contain the specific concat error, got:\n{}",
        diagnostics
    );
}

#[test]
fn writes_llvm_ir_txt_for_valid_concat() {
    let source = r#"print("The meaning of life is " @ 42);"#;
    let output_path = unique_output_path("valid_concat_ir");

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

    let llvm_ir =
        fs::read_to_string(&output_path).expect("compiler should write llvm output file on success");
    assert!(
        llvm_ir.contains("define i32 @main()"),
        "output file should contain LLVM IR entrypoint, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("@asprintf"),
        "output LLVM IR should include concat runtime declaration/calls, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_llvm_ir_for_boolean_and_comparison_expressions() {
    let source = r#"
let x = 10;
let y = 20;

print(x < y);
print(x == 10);
print(true && (x < y));
print(!(x >= y));
"#;
    let output_path = unique_output_path("valid_boolean_comparison_ir");

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
        llvm_ir.contains("fcmp olt double"),
        "expected numeric comparison in LLVM IR, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("fcmp oeq double"),
        "expected numeric equality in LLVM IR, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains(" and i1 "),
        "expected logical and in LLVM IR, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains(" xor i1 "),
        "expected logical not in LLVM IR, got:\n{}",
        llvm_ir
    );
}
