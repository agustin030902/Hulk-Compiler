//! Interacción con el mundo exterior: ejecutar el IR generado (lli o clang
//! según plataforma), abrir una terminal real del sistema, instalar la
//! extensión de VSCode y listar ejemplos.

use std::{fs, path::PathBuf, process::Command};

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

/// Snippets de demostración de un click, uno por feature del lenguaje.
pub fn snippets() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "λ Lambdas y closures",
            r#"function make_adder(n: Number): (Number) -> Number {
    function (x: Number): Number -> x + n;
}

let add5: (Number) -> Number = make_adder(5) in
let double: (Number) -> Number = function (x: Number): Number -> x * 2 in {
    print(add5(10));        // 15
    print(double(add5(1))); // 12
};
"#,
        ),
        (
            "⚙ Macros (define)",
            r#"define square(x: Number): Number -> x * x;

define repeat(times: Number, body: Number): Number {
    let i: Number = times in
        while (i > 0) {
            i := i - 1;
            body;
        };
}

let count = 0 in {
    repeat(5, count := count + 1);
    print(square(count)); // 25
};
"#,
        ),
        (
            "📦 Arrays",
            r#"let a: Number[] = new Number[5]{ i -> i * i } in {
    let total = 0, j = 0 in {
        while (j < a.size()) {
            total := total + a[j];
            j := j + 1;
        };
        print(total); // 0+1+4+9+16 = 30
    };
    let b: Number[] = {10, 20, 30} in
        print(b[1] + b.size()); // 23
};
"#,
        ),
        (
            "🧬 Tipos y protocolos",
            r#"protocol Shape {
    area(): Number;
}

type Circle(r: Number) {
    radius: Number = r;
    area(): Number { PI * self.radius ^ 2; }
}

type Square(s: Number) {
    side: Number = s;
    area(): Number { self.side * self.side; }
}

function describe(s: Shape): Number { s.area(); }

{
    print(describe(new Circle(1)));
    print(describe(new Square(4)));
}
"#,
        ),
        (
            "🔁 Generadores (splat)",
            r#"type Squares(n: Number) {
    i: Number = 0;
    limit: Number = n;
    next(): Boolean {
        self.i := self.i + 1;
        self.i <= self.limit;
    }
    current(): Number { self.i * self.i; }
}

function sum_gen(gen: Number*): Number {
    let s: Number = 0 in {
        for (x in gen) { s := s + x; };
        s;
    };
}

print(sum_gen(new Squares(4))); // 1+4+9+16 = 30
"#,
        ),
    ]
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

// ── Ejecución del programa ───────────────────────────────────────────────

/// Resultado crudo de ejecutar el programa compilado. Modela una sola
/// ejecución independientemente de la plataforma (lli o clang→exe).
#[derive(Debug, Clone, Default)]
pub struct ProgramOutput {
    /// Comando ejecutado (se muestra como prompt en la consola integrada).
    pub command: String,
    /// Salida estándar del programa: lo que imprime `print`.
    pub stdout: String,
    /// Salida de error del programa, si la hubo.
    pub stderr: String,
    /// Código de salida del proceso.
    pub exit_code: Option<i32>,
}

impl ProgramOutput {
    pub fn is_success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

/// Ruta del ejecutable nativo generado por el flujo clang.
fn executable_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from("artifacts/gui_output.exe")
    } else {
        PathBuf::from("artifacts/gui_output")
    }
}

/// Punto único de ejecución: hace el spawning específico de la plataforma y
/// devuelve la salida cruda. Tanto la vista limpia como la verbosa delegan
/// aquí.
fn execute_program(lli_path: &str, ll_path: &PathBuf) -> Result<ProgramOutput, String> {
    if !ll_path.exists() {
        return Err(format!(
            "No se encontró el archivo LLVM IR en {}",
            ll_path.display()
        ));
    }

    if cfg!(target_os = "windows") {
        execute_with_clang(ll_path)
    } else {
        execute_with_lli(lli_path, ll_path)
    }
}

fn execute_with_lli(lli_path: &str, ll_path: &PathBuf) -> Result<ProgramOutput, String> {
    let output = Command::new(lli_path)
        .arg(ll_path)
        .output()
        .map_err(|e| format!("Fallo al ejecutar lli ('{lli_path}'): {e}"))?;

    Ok(ProgramOutput {
        command: format!("{} {}", lli_path, ll_path.display()),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code(),
    })
}

fn execute_with_clang(ll_path: &PathBuf) -> Result<ProgramOutput, String> {
    let exe_path = executable_path();

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

    Ok(ProgramOutput {
        command: exe_path.display().to_string(),
        stdout: String::from_utf8_lossy(&run.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&run.stderr).into_owned(),
        exit_code: run.status.code(),
    })
}

/// Ejecuta el programa y devuelve la salida cruda, pensada para la consola
/// tipo terminal integrada (solo lo que imprime el programa).
pub fn run_program_clean(lli_path: &str, ll_path: &PathBuf) -> Result<ProgramOutput, String> {
    execute_program(lli_path, ll_path)
}

/// Ejecuta el programa y devuelve un reporte verboso (comando, exit code,
/// stdout y stderr etiquetados). Útil para depuración y logs.
#[allow(dead_code)]
pub fn run_program(lli_path: &str, ll_path: &PathBuf) -> Result<String, String> {
    let output = execute_program(lli_path, ll_path)?;

    let mut result = String::new();
    result.push_str(&format!("Comando: {}\n", output.command));
    result.push_str(&format!("Exit code: {:?}\n", output.exit_code));
    if !output.stdout.is_empty() {
        result.push_str("\n--- stdout ---\n");
        result.push_str(&output.stdout);
    }
    if !output.stderr.is_empty() {
        result.push_str("\n--- stderr ---\n");
        result.push_str(&output.stderr);
    }
    Ok(result)
}

// ── Terminal externa del sistema ──────────────────────────────────────────

/// Escapa una cadena para incrustarla en un literal de AppleScript
/// (osascript), que delimita strings con comillas dobles y usa `\` como
/// carácter de escape.
fn escape_for_applescript(command: &str) -> String {
    command.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Abre una terminal real del sistema y ejecuta el programa compilado en ella,
/// de modo que el usuario vea la salida en una consola nativa.
pub fn open_in_external_terminal(lli_path: &str, ll_path: &PathBuf) -> Result<String, String> {
    if !ll_path.exists() {
        return Err(format!(
            "Compila primero: no existe {}",
            ll_path.display()
        ));
    }

    if cfg!(target_os = "macos") {
        open_terminal_macos(lli_path, ll_path)
    } else if cfg!(target_os = "windows") {
        open_terminal_windows()
    } else {
        open_terminal_linux(lli_path, ll_path)
    }
}

fn open_terminal_macos(lli_path: &str, ll_path: &PathBuf) -> Result<String, String> {
    let shell_cmd = format!("clear; {} '{}'", lli_path, ll_path.display());
    let script = format!(
        "tell application \"Terminal\"\n\tactivate\n\tdo script \"{}\"\nend tell",
        escape_for_applescript(&shell_cmd)
    );

    Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn()
        .map(|_| "Terminal.app abierta con la salida del programa.".to_string())
        .map_err(|e| format!("No se pudo abrir Terminal.app: {e}"))
}

fn open_terminal_linux(lli_path: &str, ll_path: &PathBuf) -> Result<String, String> {
    let shell_cmd = format!(
        "clear; {} '{}'; echo; echo '── programa finalizado, pulsa Enter ──'; read _",
        lli_path,
        ll_path.display()
    );

    let terminals = [
        "x-terminal-emulator",
        "gnome-terminal",
        "konsole",
        "xfce4-terminal",
        "xterm",
    ];

    for term in terminals {
        if Command::new(term)
            .arg("-e")
            .arg("bash")
            .arg("-c")
            .arg(&shell_cmd)
            .spawn()
            .is_ok()
        {
            return Ok(format!("Terminal abierta con '{term}'."));
        }
    }

    Err(
        "No se encontró un emulador de terminal (probé gnome-terminal, konsole, xterm...)."
            .to_string(),
    )
}

fn open_terminal_windows() -> Result<String, String> {
    let exe = executable_path();
    if !exe.exists() {
        return Err(format!(
            "No existe el ejecutable {}. Compila y ejecuta primero.",
            exe.display()
        ));
    }

    Command::new("cmd")
        .args(["/C", "start", "Hulk", "cmd", "/K"])
        .arg(&exe)
        .spawn()
        .map(|_| "Consola de Windows abierta con la salida del programa.".to_string())
        .map_err(|e| format!("No se pudo abrir cmd: {e}"))
}
