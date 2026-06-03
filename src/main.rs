mod compiler;
mod error;
mod codegen;
mod lexer;
mod parser;
mod semantic;

use std::{env, fs, path::PathBuf, process::Command};

use compiler::{Compiler, CompileOptions};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("(0,0) LEXICAL: expected exactly one input file");
        std::process::exit(1);
    }

    let input_path = &args[1];

    let source = match fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("(0,0) LEXICAL: failed to read file '{}'", input_path);
            std::process::exit(1);
        }
    };

    let mut compiler = Compiler::new();

    let options = CompileOptions {
        output_path: PathBuf::from("output"),
    };

    let report = compiler.compile(&source, &options);

    // =========================
    // ERRORES
    // =========================
    if !report.errors.is_empty() {
        for err in &report.errors {
            eprintln!("{}", err);
        }
        std::process::exit(report.errors[0].exit_code());
    }

    // =========================
    // CODEGEN + EXECUTE
    // =========================
    if let Some(ir) = report.llvm_ir.clone() {
        use std::fs;
        use std::process::Command;

        // 1. escribir LLVM IR
        fs::write("output.ll", &ir).expect("failed to write output.ll");

        // 2. compilar con clang
        let status = Command::new("clang")
            .args(["output.ll", "-o", "output"])
            .status()
            .expect("failed to run clang");

        if !status.success() {
            eprintln!("LLVM compilation failed");
            std::process::exit(1);
        }

        // 3. ejecutar binario
        let run_status = Command::new("./output")
            .status()
            .expect("failed to execute program");

        if !run_status.success() {
            eprintln!("Runtime error");
            std::process::exit(1);
        }
    }

    std::process::exit(0);
}