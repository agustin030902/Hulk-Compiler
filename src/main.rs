mod codegen;
mod compiler;
mod error;
mod lexer;
mod parser;
mod runner;
mod semantic;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use compiler::{CompileOptions, Compiler, OutputKind};
use runner::{LlvmRunner, RunnerOptions};

#[derive(Debug)]
enum Command {
    RunOne(RunOneConfig),
    RunAll(RunAllConfig),
    Run(RunConfig),
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

/// Configuración para el comando 'run': compilar un archivo .hk a ejecutable y ejecutarlo.
#[derive(Debug)]
struct RunConfig {
    input_path: PathBuf,
    ir_path: Option<PathBuf>,
    exe_path: Option<PathBuf>,
    clang_bin: String,
    opt_level: u8,
    no_exec: bool,
    program_args: Vec<String>,
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
        Command::Run(config) => run_with_execution(config),
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

fn run_with_execution(config: RunConfig) {
    // Cargar y compilar el archivo fuente
    let source = match fs::read_to_string(&config.input_path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("Error reading '{}': {}", config.input_path.display(), err);
            return;
        }
    };

    let mut compiler = Compiler::new();

    // Determinar ruta de salida para el LLVM IR
    let ir_path = if let Some(ir) = &config.ir_path {
        ir.clone()
    } else {
        let stem = file_stem_or_default(&config.input_path);
        PathBuf::from(format!("artifacts/{}.ll", stem))
    };

    // Compilar a LLVM IR
    let options = CompileOptions {
        output_path: ir_path.clone(),
    };
    let report = compiler.compile(&source, &options);

    if !report.errors.is_empty() {
        eprintln!("Compilation failed:");
        for error in &report.errors {
            eprintln!("  {}", error);
        }
        return;
    }

    println!("✓ Compiled to LLVM IR: {}", ir_path.display());

    // Compilar LLVM IR a ejecutable con clang
    let exe_path = if let Some(exe) = &config.exe_path {
        exe.clone()
    } else {
        let stem = file_stem_or_default(&config.input_path);
        runner::platform::Platform::as_executable_path(Path::new(&format!("artifacts/{}", stem)))
    };

    let runner_opts = RunnerOptions {
        clang_bin: config.clang_bin.clone(),
        opt_level: config.opt_level,
        extra_args: Vec::new(),
    };

    match LlvmRunner::compile_ll_to_executable(&ir_path, Some(&exe_path), &runner_opts) {
        Ok(exe) => {
            println!("✓ Compiled to executable: {}", exe.display());

            // Si no tiene --no-exec, ejecutar el programa
            if !config.no_exec {
                println!("\n--- Program output ---");
                match LlvmRunner::run_executable(&exe, &config.program_args) {
                    Ok(output) => {
                        if !output.stdout.is_empty() {
                            print!("{}", String::from_utf8_lossy(&output.stdout));
                        }
                        if !output.stderr.is_empty() {
                            eprint!("{}", String::from_utf8_lossy(&output.stderr));
                        }
                        if !output.status.success() {
                            eprintln!(
                                "Program exited with status: {}",
                                output.status.code().unwrap_or(-1)
                            );
                        }
                    }
                    Err(err) => {
                        eprintln!("Error executing program: {}", err);
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Error compiling to executable: {}", err);
        }
    }
}

fn parse_command() -> Result<Command, String> {
    let mut args = env::args().skip(1).peekable();

    // Detectar el primer argumento para determinar el comando
    if let Some(first_arg) = args.peek() {
        match first_arg.as_str() {
            "run" => {
                args.next(); // consumir "run"
                let remaining: Vec<String> = args.collect();
                return parse_run_command(remaining);
            }
            "--help" | "-h" => return Ok(Command::Help),
            _ => {}
        }
    }

    // Parsing para los comandos antiguos: RunOne y RunAll
    let mut single = RunOneConfig::default();
    let mut batch = RunAllConfig::default();
    let mut run_all = false;

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

fn parse_run_command(args: Vec<String>) -> Result<Command, String> {
    // Mostrar ayuda si se solicita
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: run [<file.hk>] [options]");
        println!();
        println!("Options:");
        println!("  --input <file>         Input .hk file");
        println!("  --emit-ir <path>       Output LLVM IR file");
        println!("  --out <exe>            Output executable path");
        println!("  --clang <path>         Path to clang binary (default: 'clang')");
        println!("  --opt-level <0-3>      Optimization level (default: 2)");
        println!("  --no-exec              Don't execute after compilation");
        println!("  -- args...             Arguments to pass to the program");
        std::process::exit(0);
    }

    let mut args_iter = args.into_iter();
    let mut input_path: Option<PathBuf> = None;
    let mut ir_path: Option<PathBuf> = None;
    let mut exe_path: Option<PathBuf> = None;
    let mut clang_bin = "clang".to_string();
    let mut opt_level = 2u8;
    let mut no_exec = false;
    let mut program_args: Vec<String> = Vec::new();
    let mut reading_program_args = false;

    while let Some(arg) = args_iter.next() {
        // Si encontramos "--", el resto son argumentos del programa
        if arg == "--" {
            reading_program_args = true;
            continue;
        }

        if reading_program_args {
            program_args.push(arg);
            continue;
        }

        if arg.starts_with("--") {
            match arg.as_str() {
                "--input" => {
                    let value = args_iter
                        .next()
                        .ok_or_else(|| "Missing value for --input".to_string())?;
                    input_path = Some(PathBuf::from(value));
                }
                "--emit-ir" => {
                    let value = args_iter
                        .next()
                        .ok_or_else(|| "Missing value for --emit-ir".to_string())?;
                    ir_path = Some(PathBuf::from(value));
                }
                "--out" => {
                    let value = args_iter
                        .next()
                        .ok_or_else(|| "Missing value for --out".to_string())?;
                    exe_path = Some(PathBuf::from(value));
                }
                "--clang" => {
                    let value = args_iter
                        .next()
                        .ok_or_else(|| "Missing value for --clang".to_string())?;
                    clang_bin = value;
                }
                "--opt-level" => {
                    let value = args_iter
                        .next()
                        .ok_or_else(|| "Missing value for --opt-level".to_string())?;
                    opt_level = value
                        .parse::<u8>()
                        .map_err(|_| format!("Invalid optimization level: {}", value))?;
                }
                "--no-exec" => {
                    no_exec = true;
                }
                other => {
                    return Err(format!("Unknown flag for 'run': {}", other));
                }
            }
        } else {
            // Es un argumento posicional (archivo de entrada)
            if input_path.is_none() {
                input_path = Some(PathBuf::from(arg));
            } else {
                // Después del archivo de entrada, todo lo demás son argumentos del programa
                program_args.push(arg);
                reading_program_args = true;
            }
        }
    }

    let input = input_path.ok_or_else(|| {
        "Missing input file. Usage: run [<file.hk>] [options] [-- program_args]".to_string()
    })?;

    Ok(Command::Run(RunConfig {
        input_path: input,
        ir_path,
        exe_path,
        clang_bin,
        opt_level,
        no_exec,
        program_args,
    }))
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
    println!("  cargo run -- run [--input <file.hk>] [options] [-- args...]");
    println!();
    println!("Commands:");
    println!("  run                    Compile a .hk file to executable and run it");
    println!();
    println!("Run options:");
    println!("  --input <file>         Input .hk file (required for 'run')");
    println!("  --emit-ir <path>       Output LLVM IR file (optional)");
    println!("  --out <exe>            Output executable path (optional)");
    println!("  --clang <path>         Path to clang binary (default: 'clang')");
    println!("  --opt-level <0-3>      Optimization level (default: 2)");
    println!("  --no-exec              Generate executable but don't run it");
    println!("  -- args...             Arguments to pass to the program");
    println!();
    println!("Examples:");
    println!("  cargo run -- --emit-ir artifacts/intermediate.txt");
    println!(
        "  cargo run -- --input examples/calculator_ok.hk --emit-ir artifacts/calculator_ir.txt"
    );
    println!("  cargo run -- run --input examples/calculator_ok.hk");
    println!("  cargo run -- run examples/calculator_ok.hk --opt-level 3");
    println!("  cargo run -- run examples/calculator_ok.hk -- arg1 arg2");
}
