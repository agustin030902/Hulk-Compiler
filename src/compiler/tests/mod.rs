mod destructive_assign;
mod let_in;
mod print_expr;
mod rand_expression;
mod string_escape;
mod types;
mod while_expr;

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
fn writes_llvm_ir_for_recursive_function_program() {
    let source = r#"
function fact(n) => if (n == 0) 1 else n * fact(n - 1);
print(fact(5));
"#;
    let output_path = unique_output_path("recursive_function_ir");

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
        llvm_ir.contains("define double @hulk_fact(double %n)"),
        "output file should contain user function definition, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call double @hulk_fact(double"),
        "output file should contain user function call, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_llvm_ir_for_string_function_program() {
    let source = r#"
function greet(name) => "hi " @ name;
print(greet("ana"));
"#;
    let output_path = unique_output_path("string_function_ir");

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
        llvm_ir.contains("define i8* @hulk_greet(i8* %name)"),
        "output file should contain string function definition, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call i8* @hulk_greet(i8*"),
        "output file should contain typed string function call, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_llvm_ir_for_identity_function_inferred_by_context() {
    let source = r#"
function id(x) => x;
function plus_one(y) => id(y) + 1;
print(plus_one(41));
"#;
    let output_path = unique_output_path("identity_inferred_ir");

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
        llvm_ir.contains("define double @hulk_id(double %x)"),
        "output file should contain inferred id signature, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_llvm_ir_for_recursive_string_function_program() {
    let source = r#"
function stars(n) => if (n == 0) "" else stars(n - 1) @ "*";
print(stars(5));
"#;
    let output_path = unique_output_path("recursive_string_function_ir");

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
        llvm_ir.contains("define i8* @hulk_stars(double %n)"),
        "output file should contain recursive string function definition, got:\n{}",
        llvm_ir
    );
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

    let llvm_ir = fs::read_to_string(&output_path)
        .expect("compiler should write llvm output file on success");
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

#[test]
fn writes_llvm_ir_for_reassignment_with_type_change() {
    let source = r#"
let x = 45;
x = true;
print(x);
"#;
    let output_path = unique_output_path("valid_reassignment_ir");

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
        llvm_ir.contains("alloca i1"),
        "reassignment to boolean should allocate bool storage, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("zext i1"),
        "printing reassigned boolean should convert i1 to i32, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_llvm_ir_for_builtin_math_functions() {
    let source = r#"
let a = sin(PI);
let b = cos(E);
let c = sqrt(9);
let d = exp(1);
let e = log(4, 64);
print(a + b + c + d + e);
"#;
    let output_path = unique_output_path("valid_builtin_math_ir");

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
        llvm_ir.contains("@llvm.sin.f64"),
        "IR should include sin intrinsic, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("@llvm.cos.f64"),
        "IR should include cos intrinsic, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("@llvm.sqrt.f64"),
        "IR should include sqrt intrinsic, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("@llvm.exp.f64"),
        "IR should include exp intrinsic, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("@llvm.log.f64"),
        "IR should include log intrinsic, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_diagnostics_for_invalid_builtin_log_arguments() {
    let source = r#"print(log(2, "x"));"#;
    let output_path = unique_output_path("invalid_builtin_log_args");

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
        diagnostics.contains("Function 'log' expects (Number, Number), but got Number and String."),
        "diagnostics file should contain builtin type error, got:\n{}",
        diagnostics
    );
}

#[test]
fn writes_llvm_ir_for_power_operator() {
    let source = r#"
let a = 2 ^ 3 ^ 2;
print(a);
"#;
    let output_path = unique_output_path("valid_power_ir");

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
        llvm_ir.contains("@llvm.pow.f64"),
        "IR should include pow intrinsic, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_diagnostics_for_invalid_power_operands() {
    let source = r#"print("x" ^ 2);"#;
    let output_path = unique_output_path("invalid_power_operands");

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
        diagnostics.contains("Operator '^' expects Number and Number, but got String and Number."),
        "diagnostics file should contain power type error, got:\n{}",
        diagnostics
    );
}

#[test]
fn writes_llvm_ir_for_rand_builtin() {
    let source = r#"
let r = rand();
print(r);
"#;
    let output_path = unique_output_path("valid_rand_ir");

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
        llvm_ir.contains("declare i32 @rand()"),
        "IR should declare rand runtime, got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call i32 @rand()"),
        "IR should call rand runtime, got:\n{}",
        llvm_ir
    );
}

#[test]
fn writes_llvm_ir_for_expression_statement_program() {
    let source = r#"
42;
print(42);
"#;
    let output_path = unique_output_path("expression_statement_ir");

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
        llvm_ir.contains("@printf"),
        "print should still emit runtime call with expression statements present, got:\n{}",
        llvm_ir
    );
}
