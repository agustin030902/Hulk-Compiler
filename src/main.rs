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

    // Eliminar artefactos de compilaciones anteriores
    let _ = fs::remove_file("output");
    let _ = fs::remove_file("temp.ll");

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
        for error in &report.errors {
            eprintln!("{}", error);
        }
    
        let exit_code = report
            .errors
            .iter()
            .map(|e| e.exit_code())
            .min()
            .unwrap();
    
        std::process::exit(exit_code);
    }

    // =========================
    // GENERACIÓN DE CÓDIGO
    // =========================
    let ir = match report.llvm_ir {
        Some(ir) => ir,
        None => {
            eprintln!("(0,0) SEMANTIC: no LLVM IR generated");
            std::process::exit(3);
        }
    };

    // Archivo temporal con el LLVM IR
    fs::write("temp.ll", ir)
        .expect("failed to write temporary LLVM IR");

    // Generar ejecutable final requerido por el contrato
    let status = Command::new("clang")
        .args(["temp.ll", "-o", "output"])
        .status()
        .expect("failed to run clang");

    // Limpiar archivo temporal
    let _ = fs::remove_file("temp.ll");

    if !status.success() {
        eprintln!("(0,0) SEMANTIC: LLVM compilation failed");
        std::process::exit(3);
    }

    // Compilación exitosa
    std::process::exit(0);
}