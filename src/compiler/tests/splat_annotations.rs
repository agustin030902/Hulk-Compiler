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
fn compiles_function_with_splat_param_using_range() {
    let source = r#"
    function sum(numbers: Number*): Number {
        let total = 0;
        for (x in numbers) {
            total := total + x;
        };
        total
    }
    print(sum(range(1, 5)));
    "#;
    let output_path = unique_output_path("splat_param_range");

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
        llvm_ir.contains("@hulk_sum"),
        "IR should contain sum function, got:\n{}",
        llvm_ir
    );
}

#[test]
fn compiles_let_binding_with_splat_annotation() {
    let source = r#"
    let nums: Number* = range(0, 10);
    let total = 0;
    for (x in nums) {
        total := total + x;
    };
    print(total);
    "#;
    let output_path = unique_output_path("splat_let_binding");

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
}

#[test]
fn compiles_function_with_splat_param_and_interface_conformance() {
    let source = r#"
    function first(items: Number*): Number {
        let result = 0;
        for (x in items) {
            result := x;
        };
        result
    }
    print(first(range(10, 20)));
    "#;
    let output_path = unique_output_path("splat_interface_conformance");

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
        llvm_ir.contains("define double @hulk_first"),
        "IR should contain first function, got:\n{}",
        llvm_ir
    );
}
