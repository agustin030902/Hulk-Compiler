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
fn writes_llvm_ir_for_is_expression() {
    let source = r#"
    type Animal { name: String = "unknown"; }
    type Bird inherits Animal { }
    let x = new Bird();
    print(x is Animal);
    "#;
    let output_path = unique_output_path("is_expression_ir");

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
        llvm_ir.contains("call i1 @hulk_is_subtype"),
        "IR should contain hulk_is_subtype call, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("@hulk_type_parents"),
        "IR should contain type hierarchy global, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_llvm_ir_for_as_expression() {
    let source = r#"
    type Animal { value: Number = 42; }
    type Bird inherits Animal { }
    let x = new Bird();
    let y = x as Animal;
    print(y is Animal);
    "#;
    let output_path = unique_output_path("as_expression_ir");

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
        llvm_ir.contains("@hulk_type_parents"),
        "IR should contain type hierarchy global, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_llvm_ir_for_type_tag_in_malloc() {
    let source = r#"
    type Point { x: Number = 0; y: Number = 0; }
    let p = new Point();
    print(p is Point);
    "#;
    let output_path = unique_output_path("type_tag_malloc_ir");

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
        llvm_ir.contains("store i64"),
        "IR should store type tag (i64) in malloc'd struct, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("@hulk_type_parents"),
        "IR should contain type hierarchy global, got:\n{}",
        llvm_ir
    );
}
