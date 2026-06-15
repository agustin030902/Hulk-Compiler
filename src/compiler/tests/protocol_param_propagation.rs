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
fn writes_llvm_ir_dispatching_concrete_method_for_protocol_param() {
    let source = r#"
    interface Printable { show(): String; }
    type Person {
        name: String = "John";
        show(): String => self.name;
    }
    function mostrar(p: Printable): String => p.show();
    print(mostrar(new Person()));
    "#;
    let output_path = unique_output_path("protocol_param_dispatch");

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
        llvm_ir.contains("hulk_type"),
        "IR should contain concrete type method dispatch, got:\n{}",
        llvm_ir
    );
    assert!(
        !llvm_ir.contains(" Printable::"),
        "IR should NOT dispatch via interface method, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_llvm_ir_for_protocol_param_with_multiple_concrete_calls() {
    let source = r#"
    interface Printable { show(): String; }
    type Person { show(): String => "Person"; }
    type Robot { show(): String => "Robot"; }
    function mostrar(p: Printable): Unit => print(p.show());
    mostrar(new Person());
    mostrar(new Robot());
    "#;
    let output_path = unique_output_path("protocol_param_multi_call");

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
        llvm_ir.contains("hulk_type"),
        "IR should contain concrete type method dispatch, got:\n{}",
        llvm_ir
    );
}
