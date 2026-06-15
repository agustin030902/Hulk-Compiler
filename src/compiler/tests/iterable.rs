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
fn writes_llvm_ir_for_iterable_assignment_with_range() {
    let source = r#"
    let it: Iterable = new Range(0, 10);
    print(it.next());
    "#;
    let output_path = unique_output_path("iterable_assignment_ir");

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
        llvm_ir.contains("define i1 @hulk_is_subtype"),
        "IR should contain hulk_is_subtype for interface conformance, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("@hulk_type_parents"),
        "IR should contain type hierarchy global, got:\n{}",
        llvm_ir
    );
}
