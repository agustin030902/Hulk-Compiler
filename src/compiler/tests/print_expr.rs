use std::fs;

use super::{CompileOptions, Compiler, OutputKind, unique_output_path};

#[test]
fn compiles_print_as_expression_in_let_in() {
    let source = r#"
let side = print(5) in { side; print(7) };
"#;
    let output_path = unique_output_path("print_expr");

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
        ir.contains("@printf"),
        "IR should include printf call for print expression"
    );
    assert!(
        ir.contains("alloca i8"),
        "IR should store Unit values when print is bound, got:\n{}",
        ir
    );
}
