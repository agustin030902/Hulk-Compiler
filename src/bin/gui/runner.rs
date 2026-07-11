//! Interacción con el mundo exterior: ejecutar el IR generado (lli o clang
//! según plataforma), instalar la extensión de VSCode y listar ejemplos.

use std::{fs, path::PathBuf, process::Command};

use hulk_compiler::lexer::Token;

pub fn format_token(token: &Token) -> String {
    format!(
        "{:?} '{}' @ {}:{}",
        token.kind, token.value, token.line, token.column
    )
}

pub fn default_source() -> String {
    r#"
function fib(n) => if (n == 0) 0 elif (n == 1) 1 else fib(n - 1) + fib(n - 2);

let n = 8;
let value = fib(n);
print("fib(" @ n @ ") = " @ value);
"#
    .trim_start_matches('\n')
    .to_string()
}

pub fn list_example_files() -> Vec<String> {
    let dir = PathBuf::from("examples");
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|s| s.to_str())
                && (ext.eq_ignore_ascii_case("hulk") || ext.eq_ignore_ascii_case("hk"))
                && let Some(p) = path.to_str()
            {
                files.push(p.to_string());
            }
        }
    }
    files.sort();
    files
}

pub fn install_vscode_extension() -> Result<String, String> {
    let vsix_path = PathBuf::from("hulk-vscode.vsix");
    if !vsix_path.exists() {
        return Err(format!(
            "No se encontró '{}' en la raíz del proyecto.",
            vsix_path.display()
        ));
    }

    let commands = ["code", "code-insiders"];
    let mut last_error = String::new();

    for cmd in commands {
        match Command::new(cmd)
            .arg("--install-extension")
            .arg(&vsix_path)
            .arg("--force")
            .output()
        {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return Ok(format!(
                    "Extensión instalada con '{}'. {}",
                    cmd,
                    stdout.trim()
                ));
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                last_error = format!("{} devolvió error: {}", cmd, stderr.trim());
            }
            Err(err) => {
                last_error = format!("No se pudo ejecutar '{}': {}", cmd, err);
            }
        }
    }

    Err(format!(
        "No se pudo instalar la extensión. {}\nTip: instala VSCode y asegúrate de tener el comando 'code' en PATH.",
        last_error
    ))
}

fn run_with_lli(lli_path: &str, ll_path: &PathBuf) -> Result<String, String> {
    if !ll_path.exists() {
        return Err(format!(
            "No se encontró el archivo LLVM IR en {}",
            ll_path.display()
        ));
    }
    let output = Command::new(lli_path)
        .arg(ll_path)
        .output()
        .map_err(|e| format!("Fallo al ejecutar lli: {e}"))?;

    let mut result = String::new();
    result.push_str(&format!("Comando: {} {}\n", lli_path, ll_path.display()));
    result.push_str(&format!("Exit code: {:?}\n", output.status.code()));
    if !output.stdout.is_empty() {
        result.push_str("\n--- stdout ---\n");
        result.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        result.push_str("\n--- stderr ---\n");
        result.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    Ok(result)
}

fn run_with_clang(ll_path: &PathBuf) -> Result<String, String> {
    let exe_path = if cfg!(target_os = "windows") {
        PathBuf::from("artifacts/gui_output.exe")
    } else {
        PathBuf::from("artifacts/gui_output")
    };

    let compile = Command::new("clang")
        .arg(ll_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .map_err(|e| format!("Error ejecutando clang: {e}"))?;

    if !compile.status.success() {
        return Err(format!(
            "Error compilando:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        ));
    }

    let run = Command::new(&exe_path)
        .output()
        .map_err(|e| format!("Error ejecutando exe: {e}"))?;

    let mut result = String::new();
    result.push_str("Modo: clang → exe\n");
    result.push_str(&format!("Ejecutable: {}\n", exe_path.display()));
    result.push_str(&format!("Exit code: {:?}\n", run.status.code()));

    if !run.stdout.is_empty() {
        result.push_str("\n--- stdout ---\n");
        result.push_str(&String::from_utf8_lossy(&run.stdout));
    }
    if !run.stderr.is_empty() {
        result.push_str("\n--- stderr ---\n");
        result.push_str(&String::from_utf8_lossy(&run.stderr));
    }

    Ok(result)
}

pub fn run_program(lli_path: &str, ll_path: &PathBuf) -> Result<String, String> {
    if cfg!(target_os = "windows") {
        run_with_clang(ll_path)
    } else {
        run_with_lli(lli_path, ll_path)
    }
}
