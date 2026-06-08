use std::fs;

use super::{CompileOptions, Compiler, OutputKind, unique_output_path};

#[test]
fn writes_llvm_ir_for_basic_for_loop() {
    let source = r#"
for (x in range(0, 5)) {
    print(x);
};
"#;
    let output_path = unique_output_path("for_basic_loop");

    let mut compiler = Compiler::new();
    let report = compiler.compile(
        source,
        &CompileOptions {
            output_path: output_path.clone(),
        },
    );

    assert!(
        report.errors.is_empty(),
        "expected successful compilation, got: {:?}",
        report.errors
    );
    assert_eq!(report.output_kind, Some(OutputKind::LlvmIr));

    let ir = fs::read_to_string(&output_path).expect("should write llvm ir");
    assert!(
        ir.contains("while.cond."),
        "IR should include while condition label (from for desugaring), got:\n{}",
        ir
    );
    assert!(
        ir.contains("@printf"),
        "IR should include print calls, got:\n{}",
        ir
    );
}

#[test]
fn writes_llvm_ir_for_for_loop_with_accumulation() {
    let source = r#"
let total = 0;
for (x in range(1, 11)) {
    total = total + x;
};
print(total);
"#;
    let output_path = unique_output_path("for_accumulation");

    let mut compiler = Compiler::new();
    let report = compiler.compile(
        source,
        &CompileOptions {
            output_path: output_path.clone(),
        },
    );

    assert!(
        report.errors.is_empty(),
        "expected successful compilation, got: {:?}",
        report.errors
    );
    assert_eq!(report.output_kind, Some(OutputKind::LlvmIr));

    let ir = fs::read_to_string(&output_path).expect("should write llvm ir");
    assert!(
        ir.contains("while.cond."),
        "IR should contain while loop from for desugaring, got:\n{}",
        ir
    );
    assert!(
        ir.contains("@printf"),
        "IR should include print, got:\n{}",
        ir
    );
}

#[test]
fn writes_llvm_ir_for_user_type_iterable() {
    let source = r#"
type Countdown(count: Number) {
    count = count;
    next() => { self.count := self.count - 1; self.count > 0; };
    current() => self.count;
}
for (x in new Countdown(3)) {
    print(x);
};
"#;
    let output_path = unique_output_path("for_user_type_iterable");

    let mut compiler = Compiler::new();
    let report = compiler.compile(
        source,
        &CompileOptions {
            output_path: output_path.clone(),
        },
    );

    assert!(
        report.errors.is_empty(),
        "expected successful compilation, got: {:?}",
        report.errors
    );
    assert_eq!(report.output_kind, Some(OutputKind::LlvmIr));

    let ir = fs::read_to_string(&output_path).expect("should write llvm ir");
    assert!(
        ir.contains("while.cond."),
        "IR should contain while loop from for desugaring, got:\n{}",
        ir
    );
    assert!(
        ir.contains("next"),
        "IR should include Countdown next method, got:\n{}",
        ir
    );
    assert!(
        ir.contains("current"),
        "IR should include Countdown current method, got:\n{}",
        ir
    );
}

#[test]
fn writes_diagnostics_for_for_loop_with_non_iterable() {
    let source = r#"
for (x in true) {
    print(x);
};
"#;
    let output_path = unique_output_path("for_non_iterable_error");

    let mut compiler = Compiler::new();
    let report = compiler.compile(
        source,
        &CompileOptions {
            output_path: output_path.clone(),
        },
    );

    assert!(
        !report.errors.is_empty(),
        "expected compilation errors for non-iterable"
    );
    assert_eq!(report.output_kind, Some(OutputKind::Diagnostics));
}
