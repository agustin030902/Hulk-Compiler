mod codegen;
mod compiler;
mod error;
mod lexer;
mod parser;
mod semantic;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use compiler::{CompileOptions, Compiler, OutputKind};

#[derive(Debug)]
enum Command {
    RunOne(RunOneConfig),
    RunAll(RunAllConfig),
    Help,
}

#[derive(Debug)]
struct RunOneConfig {
    input_path: Option<PathBuf>,
    output_path: PathBuf,
}

impl Default for RunOneConfig {
    fn default() -> Self {
        Self {
            input_path: None,
            output_path: PathBuf::from("artifacts/intermediate.txt"),
        }
    }
}

#[derive(Debug)]
struct RunAllConfig {
    input_dir: PathBuf,
    output_dir: PathBuf,
}

impl Default for RunAllConfig {
    fn default() -> Self {
        Self {
            input_dir: PathBuf::from("examples"),
            output_dir: PathBuf::from("artifacts/batch"),
        }
    }
}

fn main() {
    let command = match parse_command() {
        Ok(command) => command,
        Err(message) => {
            eprintln!("CLI error: {message}");
            print_usage();
            return;
        }
    };

    match command {
        Command::Help => print_usage(),
        Command::RunOne(config) => run_one(config),
        Command::RunAll(config) => run_all(config),
    }
}

fn run_one(config: RunOneConfig) {
    let source = match load_source(config.input_path.as_ref()) {
        Ok(source) => source,
        Err(message) => {
            eprintln!("Source error: {message}");
            return;
        }
    };

    let mut compiler = Compiler::new();
    let options = CompileOptions {
        output_path: config.output_path.clone(),
    };
    let report = compiler.compile(&source, &options);

    println!("==============================");
    println!("SOURCE:\n{}\n", source);

    println!("TOKENS:");
    for token in &report.tokens {
        println!("{:?}", token);
    }

    if report.errors.is_empty() {
        println!("\nERRORS: none");
    } else {
        println!("\nERRORS:");
        for error in &report.errors {
            println!("{}", error);
        }
    }

    if let Some(ast) = &report.ast {
        println!("\nAST:");
        println!("{:#?}", ast);
    }

    if let Some(output_path) = &report.output_path {
        match report.output_kind {
            Some(OutputKind::LlvmIr) => {
                println!("\nLLVM IR generated at: {}", output_path.display());
                if let Some(ir) = &report.llvm_ir {
                    println!("LLVM IR lines: {}", ir.lines().count());
                }
                println!("Command to execute intermediate code:");
                println!("  lli {}", output_path.display());
            }
            Some(OutputKind::Diagnostics) => {
                println!(
                    "\nDiagnostics report generated at: {}",
                    output_path.display()
                );
            }
            None => {}
        }
    }

    if report.errors.is_empty() {
        println!("\nCOMPILATION STATUS: OK");
    } else {
        println!("\nCOMPILATION STATUS: FAILED");
    }

    println!("==============================");
}

fn run_all(config: RunAllConfig) {
    let mut compiler = Compiler::new();
    let mut files = match list_hk_files(&config.input_dir) {
        Ok(files) => files,
        Err(message) => {
            eprintln!("Batch error: {message}");
            return;
        }
    };

    if files.is_empty() {
        println!(
            "No .hk files found in directory '{}'.",
            config.input_dir.display()
        );
        return;
    }

    files.sort();

    let mut ok_count = 0usize;
    let mut fail_count = 0usize;

    println!(
        "Running batch compilation for {} file(s) from '{}'",
        files.len(),
        config.input_dir.display()
    );

    for file_path in files {
        let source = match fs::read_to_string(&file_path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!(
                    "FAILED [{}]: cannot read source ({})",
                    file_path.display(),
                    error
                );
                fail_count += 1;
                continue;
            }
        };

        let stem = file_stem_or_default(&file_path);
        let output_path = config.output_dir.join(format!("{stem}.txt"));
        let report = compiler.compile(
            &source,
            &CompileOptions {
                output_path: output_path.clone(),
            },
        );

        if report.errors.is_empty() {
            ok_count += 1;
            println!(
                "OK    [{}] -> {}",
                file_path.display(),
                output_path.display()
            );
        } else {
            fail_count += 1;
            println!(
                "FAIL  [{}] -> {} ({} error(s))",
                file_path.display(),
                output_path.display(),
                report.errors.len()
            );
            for error in &report.errors {
                println!("  - {}", error);
            }
        }
    }

    println!(
        "Batch summary: total={}, ok={}, failed={}",
        ok_count + fail_count,
        ok_count,
        fail_count
    );
}

fn parse_command() -> Result<Command, String> {
    let mut single = RunOneConfig::default();
    let mut batch = RunAllConfig::default();
    let mut run_all = false;
    let mut args = env::args().skip(1).peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            "--input" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --input".to_string())?;
                single.input_path = Some(PathBuf::from(value));
            }
            "--emit-ir" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --emit-ir".to_string())?;
                single.output_path = PathBuf::from(&value);
                batch.output_dir = PathBuf::from(value);
            }
            "--run-all" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --run-all".to_string())?;
                run_all = true;
                batch.input_dir = PathBuf::from(value);
            }
            "--emit-dir" => {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --emit-dir".to_string())?;
                batch.output_dir = PathBuf::from(value);
            }
            other => return Err(format!("Unknown argument '{other}'")),
        }
    }

    if run_all {
        if single.input_path.is_some() {
            return Err("Cannot use --input together with --run-all.".to_string());
        }
        return Ok(Command::RunAll(batch));
    }

    Ok(Command::RunOne(single))
}

fn list_hk_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(dir)
        .map_err(|error| format!("Failed to read directory '{}': {}", dir.display(), error))?;

    let files = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension().and_then(|x| x.to_str()) == Some("hk"))
        .collect::<Vec<_>>();

    Ok(files)
}

fn file_stem_or_default(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string()
}

fn load_source(input_path: Option<&PathBuf>) -> Result<String, String> {
    match input_path {
        Some(path) => fs::read_to_string(path)
            .map_err(|err| format!("Failed to read '{}': {}", path.display(), err)),
        None => Ok(default_source().to_string()),
    }
}

fn default_source() -> &'static str {
    r#"
print("zzzz");

let x = 90/ 4.567 + 1 - 9 + (9 -1) * 2 ;

print (x);
"#
}

fn print_usage() {
    println!("Usage:");
    println!("  cargo run -- [--input <source.hk>] [--emit-ir <output.txt>]");
    println!("  cargo run -- --run-all <input_dir> [--emit-dir <output_dir>]");
    println!("  cargo run -- --run-all <input_dir> [--emit-ir <output_dir>]");
    println!("  output is LLVM IR on success or diagnostics on error.");
    println!();
    println!("Examples:");
    println!("  cargo run -- --emit-ir artifacts/intermediate.txt");
    println!(
        "  cargo run -- --input examples/calculator_ok.hk --emit-ir artifacts/calculator_ir.txt"
    );
    println!("  cargo run -- --input examples/type_error.hk --emit-ir artifacts/type_error.txt");
    println!("  cargo run -- --run-all examples --emit-dir artifacts/batch");
}
