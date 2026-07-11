use std::{env, fs, path::PathBuf, process::Command};

use hulk_compiler::compiler::{Compiler, CompileOptions};

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
    if let Err(e) = fs::write("temp.ll", ir) {
        eprintln!("(0,0) SEMANTIC: failed to write temporary LLVM IR: {}", e);
        std::process::exit(3);
    }

    // Generar ejecutable final requerido por el contrato
    let status = match Command::new("clang")
        .args([
            "-Wno-override-module",
            "temp.ll",
            "-lm",
            "-o",
            "output",
        ])
        .status()
    {
        Ok(status) => status,
        Err(e) => {
            let _ = fs::remove_file("temp.ll");
            eprintln!("(0,0) SEMANTIC: failed to run clang: {}", e);
            std::process::exit(3);
        }
    };

    // Limpiar archivo temporal
    let _ = fs::remove_file("temp.ll");

    if !status.success() {
        eprintln!("(0,0) SEMANTIC: LLVM compilation failed");
        std::process::exit(3);
    }

    // Compilación exitosa
    std::process::exit(0);
}