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
fn writes_llvm_ir_txt_for_valid_escaped_strings() {
    let source = r#"
let quoted = "The message is \"Hello World\"";
let multiline = "Line1\nLine2\tDone";
let final_msg = quoted @ " | " @ multiline;
print(final_msg);
"#;
    let output_path = unique_output_path("valid_escaped_strings_ir");

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
        "output file should contain LLVM IR entrypoint, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("The message is \\22Hello World\\22\\00"),
        "IR should encode escaped double quotes in string constants, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Line1\\0ALine2\\09Done\\00"),
        "IR should encode newline and tab escapes in string constants, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_diagnostics_txt_for_invalid_escape_related_expression() {
    let source = r#"print(true @ "Line1\nLine2\tDone");"#;
    let output_path = unique_output_path("invalid_escaped_strings_diagnostics");

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
            "Operator '@' expects (String, String), (String, Number), or (Number, String), but got Boolean and String."
        ),
        "diagnostics file should contain the semantic error, got:\n{}",
        diagnostics
    );
}
