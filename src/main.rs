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
    // let ir = match detect_clang_target_triple() {
    //     Some(triple) => llvm_ir_with_target_triple(ir, &triple),
    //     None => ir,
    // };

    // Archivo temporal con el LLVM IR
    fs::write("temp.ll", ir)
        .expect("failed to write temporary LLVM IR");

    // Generar ejecutable final requerido por el contrato
    let status = Command::new("clang")
    .args([
        "-Wno-override-module",
        "temp.ll",
        "-lm",
        "-o",
        "output",
    ])
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

// fn detect_clang_target_triple() -> Option<String> {
//     let null_device = if cfg!(target_os = "windows") {
//         "NUL"
//     } else {
//         "/dev/null"
//     };

//     let output = Command::new("clang")
//         .args(["-###", "-x", "ir", null_device, "-c", "-o", null_device])
//         .output()
//         .ok()?;

//     let mut plan = String::new();
//     plan.push_str(&String::from_utf8_lossy(&output.stderr));
//     plan.push_str(&String::from_utf8_lossy(&output.stdout));

//     extract_clang_target_triple(&plan).and_then(|triple| sanitize_target_triple(&triple))
// }

// fn extract_clang_target_triple(plan: &str) -> Option<String> {
//     let quoted_tokens: Vec<&str> = plan
//         .split('"')
//         .enumerate()
//         .filter_map(|(index, token)| (index % 2 == 1).then_some(token))
//         .collect();

//     if let Some(triple) = token_after_triple_flag(quoted_tokens.iter().copied()) {
//         return Some(triple);
//     }

//     token_after_triple_flag(plan.split_whitespace().map(|token| token.trim_matches('"')))
// }

// fn token_after_triple_flag<'a>(tokens: impl IntoIterator<Item = &'a str>) -> Option<String> {
//     let mut saw_triple_flag = false;

//     for token in tokens {
//         if saw_triple_flag {
//             return Some(token.to_string());
//         }

//         saw_triple_flag = token == "-triple";
//     }

//     None
// }

// fn sanitize_target_triple(triple: &str) -> Option<String> {
//     let triple = triple.trim();

//     if triple.is_empty()
//         || !triple
//             .bytes()
//             .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
//     {
//         return None;
//     }

//     Some(triple.to_string())
// }

// fn llvm_ir_with_target_triple(ir: String, target_triple: &str) -> String {
//     if ir
//         .lines()
//         .any(|line| line.trim_start().starts_with("target triple"))
//     {
//         return ir;
//     }

//     format!("target triple = \"{}\"\n{}", target_triple, ir)
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn extracts_cc1_target_triple_from_clang_plan() {
//         let plan = r#"
//  "/opt/homebrew/bin/clang" "-cc1" "-triple" "arm64-apple-macosx26.0.0" "-emit-obj"
// "#;

//         assert_eq!(
//             extract_clang_target_triple(plan),
//             Some("arm64-apple-macosx26.0.0".to_string())
//         );
//     }

//     #[test]
//     fn rejects_unsafe_target_triple_text() {
//         assert_eq!(
//             sanitize_target_triple("arm64-apple-macosx26.0.0"),
//             Some("arm64-apple-macosx26.0.0".to_string())
//         );
//         assert_eq!(sanitize_target_triple("arm64\"; bad"), None);
//     }

//     #[test]
//     fn prepends_target_triple_when_missing() {
//         let ir = "; Hulk LLVM IR\n".to_string();

//         assert_eq!(
//             llvm_ir_with_target_triple(ir, "arm64-apple-macosx26.0.0"),
//             "target triple = \"arm64-apple-macosx26.0.0\"\n; Hulk LLVM IR\n"
//         );
//     }

//     #[test]
//     fn keeps_existing_target_triple() {
//         let ir = "target triple = \"x86_64-unknown-linux-gnu\"\n; Hulk LLVM IR\n".to_string();

//         assert_eq!(
//             llvm_ir_with_target_triple(ir.clone(), "arm64-apple-macosx26.0.0"),
//             ir
//         );
//     }
// }
