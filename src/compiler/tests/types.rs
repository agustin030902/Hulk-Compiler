use std::fs;

use super::{CompileOptions, Compiler, OutputKind, unique_output_path};

#[test]
fn writes_llvm_ir_for_type_declaration_and_instantiation() {
    let source = r#"
type Point(x: Number, y: Number) {
    x = x;
    y = y;
    norm() => sqrt(self.x ^ 2 + self.y ^ 2);
}

let p = new Point(3, 4);
print(p.norm());
"#;
    let output_path = unique_output_path("type_feature_ir");

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
        llvm_ir.contains("call i8* @malloc(i64"),
        "output should contain object allocation, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("norm(i8* %self)"),
        "output should contain method definition with receiver, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_llvm_ir_for_null_in_struct_context() {
    let source = r#"
type Node(v: Number, next: Node) {
    v = v;
    next = next;
}

let head: Node = null;
let n1 = new Node(1, head);
"#;
    let output_path = unique_output_path("type_null_ir");

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
        llvm_ir.contains("store i8* null"),
        "output should contain null pointer stores, got:\n{}",
        llvm_ir
    );
}
