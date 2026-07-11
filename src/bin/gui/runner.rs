//! Interacción con el mundo exterior: ejecutar el IR generado (lli o clang
//! según plataforma), instalar la extensión de VSCode y listar ejemplos.

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
