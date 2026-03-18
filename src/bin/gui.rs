#[path = "../codegen/mod.rs"]
mod codegen;
#[path = "../compiler/mod.rs"]
mod compiler;
#[path = "../error/mod.rs"]
mod error;
#[path = "../lexer/mod.rs"]
mod lexer;
#[path = "../parser/mod.rs"]
mod parser;
#[path = "../runner/mod.rs"]
mod runner;
#[path = "../semantic/mod.rs"]
mod semantic;

use std::{fs, path::PathBuf};

use compiler::{CompileOptions, Compiler, OutputKind};
use eframe::egui::{self, TextEdit};
use error::CompilerError;
use lexer::Token;
use std::process::Command;

struct HulkGui {
    source: String,
    status: String,
    tokens: Vec<Token>,
    errors: Vec<CompilerError>,
    ast_text: Option<String>,
    ir_text: Option<String>,
    exec_output: String,
    output_path: PathBuf,
    input_path: String,
    example_files: Vec<String>,
    lli_path: String,
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 780.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Hulk Compiler GUI",
        options,
        Box::new(|cc| Ok(Box::new(HulkGui::new(cc)))),
    )
}

impl HulkGui {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let example_files = list_example_files();
        Self {
            source: default_source(),
            status: "Listo para compilar".to_string(),
            tokens: Vec::new(),
            errors: Vec::new(),
            ast_text: None,
            ir_text: None,
            exec_output: String::new(),
            output_path: PathBuf::from("artifacts/gui_output.ll"),
            input_path: "examples/power_ok.hulk".to_string(),
            example_files,
            lli_path: "lli".to_string(),
        }
    }

    fn compile_source(&mut self) {
        let mut compiler = Compiler::new();
        let options = CompileOptions {
            output_path: self.output_path.clone(),
        };
        let report = compiler.compile(&self.source, &options);

        self.tokens = report.tokens;
        self.errors = report.errors;
        self.ast_text = report.ast.as_ref().map(|ast| format!("{:#?}", ast));
        self.ir_text = report.llvm_ir;
        self.exec_output.clear();

        self.status = if self.errors.is_empty() {
            match report.output_kind {
                Some(OutputKind::LlvmIr) => {
                    // Ejecutar automáticamente con lli si se generó IR
                    self.exec_output =
                        run_lli(&self.lli_path, &self.output_path).unwrap_or_else(|e| e);
                    format!("OK: LLVM IR generado en {}", self.output_path.display())
                }
                Some(OutputKind::Diagnostics) => {
                    "Solo se generó reporte de diagnóstico".to_string()
                }
                None => "Compilación completada".to_string(),
            }
        } else {
            format!("Se encontraron {} error(es)", self.errors.len())
        };
    }

    fn load_from_file(&mut self) {
        if self.input_path.trim().is_empty() {
            self.status = "Ingresa una ruta primero".to_string();
            return;
        }

        match fs::read_to_string(&self.input_path) {
            Ok(contents) => {
                self.source = contents;
                self.status = format!("Archivo cargado: {}", self.input_path);
            }
            Err(err) => {
                self.status = format!("No se pudo leer {}: {}", self.input_path, err);
            }
        }
    }

    fn refresh_examples(&mut self) {
        self.example_files = list_example_files();
    }
}

impl eframe::App for HulkGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.horizontal_wrapped(|ui| {
                ui.label("Archivo (.hulk | .hk):");
                egui::ComboBox::from_id_salt("example_select")
                    .width(220.0)
                    .selected_text(
                        self.input_path
                            .split('/')
                            .last()
                            .unwrap_or(&self.input_path)
                            .to_string(),
                    )
                    .show_ui(ui, |ui| {
                        for file in &self.example_files {
                            ui.selectable_value(&mut self.input_path, file.clone(), file);
                        }
                    });
                ui.text_edit_singleline(&mut self.input_path);
                if ui.button("Refrescar ejemplos").clicked() {
                    self.refresh_examples();
                }
                if ui.button("Cargar").clicked() {
                    self.load_from_file();
                }
                if ui.button("Demo rápida").clicked() {
                    self.source = default_source();
                    self.status = "Ejemplo precargado".to_string();
                }
                if ui.button("Compilar").clicked() {
                    self.compile_source();
                }
                ui.label("lli:");
                ui.text_edit_singleline(&mut self.lli_path);
                ui.label(format!("Estado: {}", self.status));
            });
        });

        egui::SidePanel::left("source_panel")
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.heading("Editor Hulk");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add(
                        TextEdit::multiline(&mut self.source)
                            .code_editor()
                            .desired_rows(32),
                    );
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Resultados del compilador");
            ui.separator();

            ui.collapsing("Errores", |ui| {
                if self.errors.is_empty() {
                    ui.label("Sin errores.");
                } else {
                    for error in &self.errors {
                        ui.monospace(format!(
                            "{:?} @ {}:{} -> {}",
                            error.category, error.line, error.column, error.message
                        ));
                    }
                }
            });

            ui.collapsing("Tokens", |ui| {
                if self.tokens.is_empty() {
                    ui.label("Compila para ver los tokens.");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| {
                            for token in &self.tokens {
                                ui.monospace(format_token(token));
                            }
                        });
                }
            });

            ui.collapsing("AST", |ui| {
                if let Some(ast) = &self.ast_text {
                    let mut ast_display = ast.clone();
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            ui.add(
                                TextEdit::multiline(&mut ast_display)
                                    .code_editor()
                                    .desired_rows(12)
                                    .interactive(false),
                            );
                        });
                } else {
                    ui.label("Sin AST disponible. Compila para generarla.");
                }
            });

            ui.collapsing("LLVM IR", |ui| {
                if let Some(ir) = &self.ir_text {
                    let mut ir_display = ir.clone();
                    egui::ScrollArea::vertical()
                        .max_height(240.0)
                        .show(ui, |ui| {
                            ui.add(
                                TextEdit::multiline(&mut ir_display)
                                    .code_editor()
                                    .desired_rows(12)
                                    .interactive(false),
                            );
                        });
                    ui.label(format!("Archivo de salida: {}", self.output_path.display()));
                } else {
                    ui.label("Compila para generar IR.");
                }
            });

            ui.collapsing("Salida lli", |ui| {
                if self.exec_output.is_empty() {
                    ui.label("Compila para ejecutar con lli y ver la salida.");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(260.0)
                        .show(ui, |ui| {
                            let mut out = self.exec_output.clone();
                            ui.add(
                                TextEdit::multiline(&mut out)
                                    .code_editor()
                                    .desired_rows(14)
                                    .interactive(false),
                            );
                        });
                }
                if ui.button("Re-ejecutar lli").clicked() {
                    self.exec_output =
                        run_lli(&self.lli_path, &self.output_path).unwrap_or_else(|e| e);
                }
            });
        });
    }
}

fn format_token(token: &Token) -> String {
    format!(
        "{:?} '{}' @ {}:{}",
        token.kind, token.value, token.line, token.column
    )
}

fn default_source() -> String {
    r#"
let base = 2;
let expv = 3;

let direct = base ^ expv;
let chained = 2 ^ 3 ^ 2;
let trig = sin(PI / 2) ^ 2 + cos(0);

print(direct);
print(chained);
print(trig);

"#
    .trim_start_matches('\n')
    .to_string()
}

fn list_example_files() -> Vec<String> {
    let dir = PathBuf::from("examples");
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if ext.eq_ignore_ascii_case("hulk") || ext.eq_ignore_ascii_case("hk") {
                        if let Some(p) = path.to_str() {
                            files.push(p.to_string());
                        }
                    }
                }
            }
        }
    }
    files.sort();
    files
}

fn run_lli(lli_path: &str, ll_path: &PathBuf) -> Result<String, String> {
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
