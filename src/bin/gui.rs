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
#[path = "../semantic/mod.rs"]
mod semantic;

use std::{fs, path::PathBuf, process::Command};

use compiler::{CompileOptions, Compiler, OutputKind};
use eframe::egui::{
    self, CollapsingHeader, Color32, FontId, TextEdit, TextFormat, TextStyle, text::LayoutJob,
};
use error::CompilerError;
use lexer::{Lexer, Token, TokenKind};
use parser::expression::{
    AssignTarget, AsExpr, BinaryExpr, BinaryOp, BlockExpr, BuiltinCallExpr, DestructiveAssignExpr,
    Expr, FunctionCallExpr, FunctionDecl, IfExpr, IsExpr, LetInExpr, Literal, MemberAccessExpr,
    MethodCallExpr, NewExpr, Program, Span, Statement, UnaryExpr, UnaryOp, WhileExpr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AstViewMode {
    Tree,
    DebugText,
}

//modo normal

// const VS_BG_MAIN: Color32 = Color32::from_rgb(30, 30, 30);
// const VS_BG_PANEL: Color32 = Color32::from_rgb(37, 37, 38);
// const VS_BG_INPUT: Color32 = Color32::from_rgb(45, 45, 45);
// const VS_TEXT: Color32 = Color32::from_rgb(212, 212, 212);
// const VS_ACCENT: Color32 = Color32::from_rgb(86, 156, 214);
// const VS_KEYWORD: Color32 = Color32::from_rgb(197, 134, 192);
// const VS_FUNCTION_YELLOW: Color32 = Color32::from_rgb(220, 220, 170);
// const VS_VARIABLE: Color32 = Color32::from_rgb(212, 212, 212);
// const VS_NUMBER: Color32 = Color32::from_rgb(181, 206, 168);
// const VS_STRING: Color32 = Color32::from_rgb(206, 145, 120);
// const VS_BOOLEAN: Color32 = Color32::from_rgb(86, 156, 214);
// const VS_OPERATOR: Color32 = Color32::from_rgb(212, 212, 212);
// const VS_UNKNOWN: Color32 = Color32::from_rgb(244, 71, 71);

// modo Cyberpunk

// const VS_BG_MAIN: Color32 = Color32::from_rgb(10, 10, 18);      // #0A0A12 (negro azulado profundo)
// const VS_BG_PANEL: Color32 = Color32::from_rgb(16, 16, 28);     // #10101C
// const VS_BG_INPUT: Color32 = Color32::from_rgb(22, 22, 38);     // #161626
// const VS_TEXT: Color32 = Color32::from_rgb(220, 220, 235);      // #DCDCEB (más suave, menos blanco puro)
// const VS_ACCENT: Color32 = Color32::from_rgb(0, 255, 255);      // #00FFFF (cian neón)
// const VS_KEYWORD: Color32 = Color32::from_rgb(255, 0, 200);     // #FF00C8 (magenta neón)
// const VS_FUNCTION_YELLOW: Color32 = Color32::from_rgb(255, 255, 80); // #FFFF50 (amarillo eléctrico suave)
// const VS_VARIABLE: Color32 = Color32::from_rgb(200, 200, 220);  // gris frío
// const VS_NUMBER: Color32 = Color32::from_rgb(0, 255, 140);      // #00FF8C (verde neón)
// const VS_STRING: Color32 = Color32::from_rgb(255, 140, 0);      // naranja neon controlado
// const VS_BOOLEAN: Color32 = Color32::from_rgb(0, 200, 255);     // azul eléctrico
// const VS_OPERATOR: Color32 = Color32::from_rgb(180, 180, 200);  // gris suave
// const VS_UNKNOWN: Color32 = Color32::from_rgb(255, 60, 60);     // rojo glitch

//Modo capuccino

const VS_BG_MAIN: Color32 = Color32::from_rgb(30, 30, 46); // base
const VS_BG_PANEL: Color32 = Color32::from_rgb(24, 24, 37); // mantle
const VS_BG_INPUT: Color32 = Color32::from_rgb(17, 17, 27); // crust
const VS_TEXT: Color32 = Color32::from_rgb(205, 214, 244); // text
const VS_ACCENT: Color32 = Color32::from_rgb(137, 180, 250); // blue
const VS_KEYWORD: Color32 = Color32::from_rgb(203, 166, 247); // mauve
const VS_FUNCTION_YELLOW: Color32 = Color32::from_rgb(250, 179, 135); // peach (funciones/builtins)
const VS_VARIABLE: Color32 = Color32::from_rgb(186, 194, 222); // subtext1
const VS_NUMBER: Color32 = Color32::from_rgb(166, 227, 161); // green
const VS_STRING: Color32 = Color32::from_rgb(249, 226, 175); // yellow pastel
const VS_BOOLEAN: Color32 = Color32::from_rgb(137, 180, 250); // blue
const VS_OPERATOR: Color32 = Color32::from_rgb(180, 190, 254); // lavender
const VS_UNKNOWN: Color32 = Color32::from_rgb(243, 139, 168); // red (error)

struct HulkGui {
    source: String,
    status: String,
    tokens: Vec<Token>,
    errors: Vec<CompilerError>,
    ast_program: Option<Program>,
    ast_text: Option<String>,
    ast_view_mode: AstViewMode,
    ir_text: Option<String>,
    exec_output: String,
    output_path: PathBuf,
    input_path: String,
    example_files: Vec<String>,
    lli_path: String,
    new_example_name: String,
    show_tutorial: bool,
    show_vscode_guide: bool,
    ast_search_query: String,
    editor_font_size: f32,
    focus_mode: bool,
}

fn apply_pro_visual_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(16.0, egui::FontFamily::Monospace),
    );

    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(VS_TEXT);
    visuals.panel_fill = VS_BG_MAIN;
    visuals.window_fill = VS_BG_PANEL;
    visuals.faint_bg_color = VS_BG_PANEL;
    visuals.extreme_bg_color = VS_BG_INPUT;
    visuals.code_bg_color = VS_BG_INPUT;
    visuals.selection.bg_fill = VS_ACCENT.gamma_multiply(0.45);
    visuals.widgets.noninteractive.bg_fill = VS_BG_PANEL;
    visuals.widgets.noninteractive.bg_stroke.color = Color32::from_rgb(66, 66, 66);
    visuals.widgets.inactive.bg_fill = VS_BG_INPUT;
    visuals.widgets.inactive.bg_stroke.color = Color32::from_rgb(76, 76, 76);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(59, 59, 59);
    visuals.widgets.hovered.bg_stroke.color = VS_ACCENT;
    visuals.widgets.active.bg_fill = Color32::from_rgb(66, 66, 66);
    visuals.widgets.active.bg_stroke.color = VS_ACCENT;
    visuals.widgets.open.bg_fill = Color32::from_rgb(63, 63, 70);

    style.visuals = visuals;
    ctx.set_style(style);
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1500.0, 900.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Hulk Compiler GUI",
        options,
        Box::new(|cc| Ok(Box::new(HulkGui::new(cc)))),
    )
}

impl HulkGui {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_pro_visual_theme(&cc.egui_ctx);

        let example_files = list_example_files();
        Self {
            source: default_source(),
            status: "Listo para compilar".to_string(),
            tokens: Vec::new(),
            errors: Vec::new(),
            ast_program: None,
            ast_text: None,
            ast_view_mode: AstViewMode::Tree,
            ir_text: None,
            exec_output: String::new(),
            output_path: PathBuf::from("artifacts/gui_output.ll"),
            input_path: "examples/power_ok.hulk".to_string(),
            example_files,
            lli_path: "lli".to_string(),
            new_example_name: "mi_ejemplo.hulk".to_string(),
            show_tutorial: false,
            show_vscode_guide: false,
            ast_search_query: String::new(),
            editor_font_size: 16.0,
            focus_mode: false,
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
        self.ast_program = report.ast;
        self.ast_text = self.ast_program.as_ref().map(|ast| format!("{:#?}", ast));
        self.ir_text = report.llvm_ir;
        self.exec_output.clear();

        self.status = if self.errors.is_empty() {
            match report.output_kind {
                Some(OutputKind::LlvmIr) => {
                    self.exec_output =
                        run_program(&self.lli_path, &self.output_path).unwrap_or_else(|e| e);
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

    fn save_current_source_as_example(&mut self) {
        let mut name = self.new_example_name.trim().to_string();
        if name.is_empty() {
            self.status = "Ponle un nombre al nuevo ejemplo".to_string();
            return;
        }
        if !name.ends_with(".hulk") && !name.ends_with(".hk") {
            name.push_str(".hulk");
        }
        let path = PathBuf::from("examples").join(name);
        match fs::write(&path, &self.source) {
            Ok(_) => {
                self.status = format!("Ejemplo guardado en {}", path.display());
                self.input_path = path.to_string_lossy().to_string();
                self.refresh_examples();
            }
            Err(err) => {
                self.status = format!("No se pudo guardar: {}", err);
            }
        }
    }

    fn install_vscode_extension_from_gui(&mut self) {
        match install_vscode_extension() {
            Ok(message) => {
                self.status = message;
            }
            Err(err) => {
                self.status = err;
            }
        }
    }

    fn show_editor(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Editor Hulk");
            ui.separator();
            ui.label("Tamaño de fuente");
            ui.add(egui::Slider::new(&mut self.editor_font_size, 13.0..=24.0).show_value(true));
            if self.focus_mode {
                ui.colored_label(Color32::from_rgb(120, 210, 150), "Modo enfoque activado");
            }
        });

        ui.separator();

        let font_size = self.editor_font_size;
        let mut layouter = move |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut job = hulk_highlight_job(text.as_str(), font_size);
            job.wrap.max_width = wrap_width;
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        };

        let editor = TextEdit::multiline(&mut self.source)
            .code_editor()
            .desired_rows(48)
            .desired_width(f32::INFINITY)
            .font(TextStyle::Monospace)
            .lock_focus(true)
            .layouter(&mut layouter);

        egui::Frame::default()
            .fill(VS_BG_INPUT)
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(64, 64, 64)))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                egui::ScrollArea::both()
                    .id_salt("source_editor_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_sized(ui.available_size(), editor);
                    });
            });
    }

    fn show_ast_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("ast_panel")
            .resizable(true)
            .default_width(520.0)
            .min_width(360.0)
            .show(ctx, |ui| {
                ui.heading("AST (panel redimensionable)");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.ast_view_mode, AstViewMode::Tree, "Vista árbol");
                    ui.selectable_value(
                        &mut self.ast_view_mode,
                        AstViewMode::DebugText,
                        "Vista debug",
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Buscar:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.ast_search_query)
                            .hint_text("nodo, variable, function, call..."),
                    );
                    if ui.button("Limpiar").clicked() {
                        self.ast_search_query.clear();
                    }
                });
                ui.separator();

                match self.ast_view_mode {
                    AstViewMode::Tree => {
                        if let Some(program) = &self.ast_program {
                            let query = self.ast_search_query.trim();
                            if !query.is_empty() {
                                let matches = count_ast_matches(program, query);
                                ui.small(format!("Coincidencias estimadas: {matches}"));
                            }
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                render_program_tree(ui, program, query);
                            });
                        } else {
                            ui.label("Sin AST disponible. Compila para generarla.");
                        }
                    }
                    AstViewMode::DebugText => {
                        if let Some(ast) = &self.ast_text {
                            let mut ast_display = ast.clone();
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.add(
                                    TextEdit::multiline(&mut ast_display)
                                        .code_editor()
                                        .desired_rows(26)
                                        .interactive(false),
                                );
                            });
                        } else {
                            ui.label("Sin AST disponible. Compila para generarla.");
                        }
                    }
                }
            });
    }

    fn show_diagnostics_bottom(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("diagnostics_bottom")
            .resizable(true)
            .default_height(300.0)
            .show(ctx, |ui| {
                ui.heading("Diagnóstico / Tokens / IR / Salida");
                ui.separator();

                CollapsingHeader::new("Errores")
                    .default_open(true)
                    .show(ui, |ui| {
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

                CollapsingHeader::new("Tokens")
                    .default_open(false)
                    .show(ui, |ui| {
                        if self.tokens.is_empty() {
                            ui.label("Compila para ver los tokens.");
                        } else {
                            egui::ScrollArea::vertical()
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    for token in &self.tokens {
                                        ui.monospace(format_token(token));
                                    }
                                });
                        }
                    });

                CollapsingHeader::new("LLVM IR")
                    .default_open(false)
                    .show(ui, |ui| {
                        if let Some(ir) = &self.ir_text {
                            let mut ir_display = ir.clone();
                            egui::ScrollArea::vertical()
                                .max_height(220.0)
                                .show(ui, |ui| {
                                    ui.add(
                                        TextEdit::multiline(&mut ir_display)
                                            .code_editor()
                                            .desired_rows(12)
                                            .interactive(false),
                                    );
                                });
                            ui.small(format!("Archivo de salida: {}", self.output_path.display()));
                        } else {
                            ui.label("Compila para generar IR.");
                        }
                    });

                CollapsingHeader::new("Salida del programa")
                    .default_open(false)
                    .show(ui, |ui| {
                        if self.exec_output.is_empty() {
                            ui.label("Compila para ejecutar con lli y ver la salida.");
                        } else {
                            egui::ScrollArea::vertical()
                                .max_height(220.0)
                                .show(ui, |ui| {
                                    let mut out = self.exec_output.clone();
                                    ui.add(
                                        TextEdit::multiline(&mut out)
                                            .code_editor()
                                            .desired_rows(10)
                                            .interactive(false),
                                    );
                                });
                        }
                        if ui.button("Re-ejecutar").clicked() {
                            self.exec_output = run_program(&self.lli_path, &self.output_path)
                                .unwrap_or_else(|e| e);
                        }
                    });
            });
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

                ui.separator();
                ui.label("Nuevo ejemplo:");
                ui.text_edit_singleline(&mut self.new_example_name);
                if ui.button("Guardar en examples").clicked() {
                    self.save_current_source_as_example();
                }

                ui.separator();
                if ui.button("Compilar").clicked() {
                    self.compile_source();
                }
                if ui
                    .button(if self.focus_mode {
                        "Salir enfoque"
                    } else {
                        "Modo enfoque"
                    })
                    .clicked()
                {
                    self.focus_mode = !self.focus_mode;
                }

                ui.separator();
                if ui.button("Instalar hulk-vscode.vsix").clicked() {
                    self.install_vscode_extension_from_gui();
                }
                if ui.button("Guía VSCode").clicked() {
                    self.show_vscode_guide = true;
                }

                ui.separator();
                if ui.button("Guía Hulk").clicked() {
                    self.show_tutorial = true;
                }

                ui.label("lli:");
                ui.text_edit_singleline(&mut self.lli_path);
                ui.label(format!("Estado: {}", self.status));
            });
        });

        if !self.focus_mode {
            self.show_ast_panel(ctx);
            self.show_diagnostics_bottom(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_editor(ui);
        });

        if self.show_tutorial {
            egui::Window::new("Guía rápida de sintaxis Hulk")
                .resizable(true)
                .collapsible(true)
                .open(&mut self.show_tutorial)
                .show(ctx, |ui| {
                    ui.label("Conceptos clave:");
                    ui.monospace(
                        "- El lenguaje es basado en expresiones; el último ';' es opcional.",
                    );
                    ui.monospace(
                        "- Bloques y let-in devuelven el valor de su última expresión.",
                    );
                    ui.monospace("- Declaración: let x = expr;");
                    ui.monospace("- Asignación destructiva: x := expr (mismo tipo).");
                    ui.monospace(
                        "- Builtins: sin, cos, sqrt, exp, log(base, value), rand().",
                    );
                    ui.monospace("- Constantes: PI, E.");
                    ui.monospace("- Operadores: + - * / ^ @ && || ! == != < > <= >=.");
                    ui.monospace("- Identificadores: empiezan con letra; luego letras, dígitos, '_'. No inician con '_' ni dígito.");
                });
        }

        if self.show_vscode_guide {
            egui::Window::new("Guía VSCode + extensión Hulk")
                .resizable(true)
                .open(&mut self.show_vscode_guide)
                .show(ctx, |ui| {
                    ui.label("Para una experiencia externa de editor con la extensión:");
                    ui.monospace("1) Asegúrate de tener VS Code en PATH (comando 'code').");
                    ui.monospace("2) Desde esta GUI usa 'Instalar hulk-vscode.vsix'.");
                    ui.monospace("3) O manual: code --install-extension hulk-vscode.vsix --force");
                    ui.monospace(
                        "4) Abre la carpeta del proyecto en VSCode para usar tema/syntax allí.",
                    );
                    ui.separator();
                    ui.label("Nota: esta GUI usa su propio resaltado, no el motor de VSCode.");
                });
        }
    }
}

fn hulk_highlight_job(source: &str, font_size: f32) -> LayoutJob {
    let mut job = LayoutJob::default();

    let normal = text_format(font_size, VS_TEXT);
    let keyword = text_format(font_size, VS_KEYWORD);
    let function = text_format(font_size, VS_FUNCTION_YELLOW);
    let builtin = text_format(font_size, VS_FUNCTION_YELLOW);
    let variable = text_format(font_size, VS_VARIABLE);
    let number = text_format(font_size, VS_NUMBER);
    let string = text_format(font_size, VS_STRING);
    let boolean = text_format(font_size, VS_BOOLEAN);
    let operator = text_format(font_size, VS_OPERATOR);
    let unknown = text_format(font_size, VS_UNKNOWN);

    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.lex();

    let mut cursor = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if token.start > cursor {
            job.append(&source[cursor..token.start], 0.0, normal.clone());
        }

        if token.end > token.start {
            let piece = &source[token.start..token.end];
            let format = match classify_highlight_role(&tokens, idx) {
                HighlightRole::Keyword => keyword.clone(),
                HighlightRole::BuiltinFunction => builtin.clone(),
                HighlightRole::FunctionName => function.clone(),
                HighlightRole::Variable => variable.clone(),
                HighlightRole::Number => number.clone(),
                HighlightRole::String => string.clone(),
                HighlightRole::Boolean => boolean.clone(),
                HighlightRole::Operator => operator.clone(),
                HighlightRole::Unknown => unknown.clone(),
                HighlightRole::Plain => normal.clone(),
            };
            job.append(piece, 0.0, format);
        }

        cursor = token.end;
    }

    if cursor < source.len() {
        job.append(&source[cursor..], 0.0, normal);
    }

    job
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HighlightRole {
    Keyword,
    BuiltinFunction,
    FunctionName,
    Variable,
    Number,
    String,
    Boolean,
    Operator,
    Unknown,
    Plain,
}

fn classify_highlight_role(tokens: &[Token], idx: usize) -> HighlightRole {
    let kind = &tokens[idx].kind;
    let prev_kind = idx.checked_sub(1).map(|i| &tokens[i].kind);
    let next_kind = tokens.get(idx + 1).map(|t| &t.kind);

    match kind {
        TokenKind::Let
        | TokenKind::Function
        | TokenKind::Type
        | TokenKind::Interface
        | TokenKind::Extends
        | TokenKind::New
        | TokenKind::While
        | TokenKind::For
        | TokenKind::Range
        | TokenKind::In
        | TokenKind::If
        | TokenKind::Else
        | TokenKind::Elif
        | TokenKind::Inherits
        | TokenKind::Is
        | TokenKind::As => HighlightRole::Keyword,
        TokenKind::Print
        | TokenKind::Sin
        | TokenKind::Cos
        | TokenKind::Sqrt
        | TokenKind::Exp
        | TokenKind::Log
        | TokenKind::Rand => HighlightRole::BuiltinFunction,
        TokenKind::Pi | TokenKind::E => HighlightRole::FunctionName,
        TokenKind::Number(_) => HighlightRole::Number,
        TokenKind::String(_) => HighlightRole::String,
        TokenKind::Boolean(_) | TokenKind::Null => HighlightRole::Boolean,
        TokenKind::Identifier(_) => {
            let is_declaration_name = matches!(prev_kind, Some(TokenKind::Function));
            let is_call_name = matches!(next_kind, Some(TokenKind::LeftParen));
            if is_declaration_name || is_call_name {
                HighlightRole::FunctionName
            } else {
                HighlightRole::Variable
            }
        }
        TokenKind::Unknown => HighlightRole::Unknown,
        TokenKind::Assign
        | TokenKind::Arrow
        | TokenKind::Add
        | TokenKind::Power
        | TokenKind::Concat
        | TokenKind::ConcatSpace
        | TokenKind::Minus
        | TokenKind::Multiply
        | TokenKind::Divide
        | TokenKind::Mod
        | TokenKind::EqualEqual
        | TokenKind::NotEqual
        | TokenKind::Less
        | TokenKind::Greater
        | TokenKind::LessEqual
        | TokenKind::GreaterEqual
        | TokenKind::And
        | TokenKind::Or
        | TokenKind::Not
        | TokenKind::DestructiveAssign
        | TokenKind::Colon
        | TokenKind::Comma
        | TokenKind::Semicolon
        | TokenKind::Dot
        | TokenKind::LeftBrace
        | TokenKind::RightBrace
        | TokenKind::LeftParen
        | TokenKind::RightParen => HighlightRole::Operator,
        TokenKind::EOF => HighlightRole::Plain,
    }
}

fn text_format(font_size: f32, color: Color32) -> TextFormat {
    TextFormat {
        font_id: FontId::monospace(font_size),
        color,
        ..Default::default()
    }
}

fn match_rich_text(text: impl Into<String>, query: &str) -> egui::RichText {
    let text = text.into();
    if query.is_empty() {
        return egui::RichText::new(text).color(VS_TEXT);
    }
    if text
        .to_ascii_lowercase()
        .contains(&query.to_ascii_lowercase())
    {
        egui::RichText::new(text)
            .color(Color32::from_rgb(255, 214, 102))
            .strong()
    } else {
        egui::RichText::new(text).color(Color32::from_rgb(178, 178, 178))
    }
}

fn count_ast_matches(program: &Program, query: &str) -> usize {
    if query.trim().is_empty() {
        return 0;
    }
    let query_lc = query.to_ascii_lowercase();
    let debug_text = format!("{:#?}", program).to_ascii_lowercase();
    debug_text.matches(&query_lc).count()
}
fn render_program_tree(ui: &mut egui::Ui, program: &Program, query: &str) {
    ui.label(match_rich_text(
        format!(
            "Programa: {} función(es) global(es), {} statement(s) en main",
            program.functions.len(),
            program.statements.len()
        ),
        query,
    ));
    ui.separator();

    CollapsingHeader::new(match_rich_text("Funciones globales", query))
        .default_open(true)
        .show(ui, |ui| {
            if program.functions.is_empty() {
                ui.small("Sin funciones declaradas.");
            }
            for (index, function) in program.functions.iter().enumerate() {
                render_function_tree(ui, function, index, query);
            }
        });

    CollapsingHeader::new(match_rich_text("Statements de main", query))
        .default_open(true)
        .show(ui, |ui| {
            if program.statements.is_empty() {
                ui.small("Sin statements globales.");
            }
            for (index, statement) in program.statements.iter().enumerate() {
                render_statement_tree(ui, statement, &format!("main[{index}]"), query);
            }
        });
}

fn render_function_tree(ui: &mut egui::Ui, function: &FunctionDecl, index: usize, query: &str) {
    let params = function
        .params
        .iter()
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    CollapsingHeader::new(match_rich_text(
        format!("[{index}] function {}({params})", function.name),
        query,
    ))
    .default_open(false)
    .show(ui, |ui| {
        ui.small(match_rich_text(
            format!("span {}", span_text(function.span)),
            query,
        ));

        CollapsingHeader::new(match_rich_text("Parámetros", query))
            .default_open(true)
            .show(ui, |ui| {
                if function.params.is_empty() {
                    ui.small("Sin parámetros.");
                }
                for param in &function.params {
                    ui.monospace(format!("{} [{}]", param.name, span_text(param.span)));
                }
            });

        CollapsingHeader::new(match_rich_text("Cuerpo", query))
            .default_open(true)
            .show(ui, |ui| {
                render_expr_tree(ui, &function.body, "body", query);
            });
    });
}

fn render_statement_tree(ui: &mut egui::Ui, statement: &Statement, label: &str, query: &str) {
    match statement {
        Statement::Let {
            name, value, span, ..
        } => {
            CollapsingHeader::new(match_rich_text(format!("{label}: let {name} = ..."), query))
                .default_open(false)
                .show(ui, |ui| {
                    ui.small(match_rich_text(format!("span {}", span_text(*span)), query));
                    render_expr_tree(ui, value, "value", query);
                });
        }
        Statement::Assign {
            name, value, span, ..
        } => {
            CollapsingHeader::new(match_rich_text(format!("{label}: {name} = ..."), query))
                .default_open(false)
                .show(ui, |ui| {
                    ui.small(match_rich_text(format!("span {}", span_text(*span)), query));
                    render_expr_tree(ui, value, "value", query);
                });
        }
        Statement::Print { value, span } => {
            CollapsingHeader::new(match_rich_text(format!("{label}: print(...)"), query))
                .default_open(false)
                .show(ui, |ui| {
                    ui.small(match_rich_text(format!("span {}", span_text(*span)), query));
                    render_expr_tree(ui, value, "arg", query);
                });
        }
        Statement::Expr { value, span } => {
            CollapsingHeader::new(match_rich_text(format!("{label}: expr statement"), query))
                .default_open(false)
                .show(ui, |ui| {
                    ui.small(match_rich_text(format!("span {}", span_text(*span)), query));
                    render_expr_tree(ui, value, "expr", query);
                });
        }
    }
}

fn render_expr_tree(ui: &mut egui::Ui, expr: &Expr, label: &str, query: &str) {
    match expr {
        Expr::Literal { value, span } => {
            ui.label(match_rich_text(
                format!(
                    "{label}: literal {} [{}]",
                    literal_text(value),
                    span_text(*span)
                ),
                query,
            ));
        }
        Expr::Variable { name, span } => {
            ui.label(match_rich_text(
                format!("{label}: variable {name} [{}]", span_text(*span)),
                query,
            ));
        }
        Expr::Binary(binary) => render_binary_tree(ui, binary, label, query),
        Expr::Unary(unary) => render_unary_tree(ui, unary, label, query),
        Expr::BuiltinCall(call) => render_builtin_call_tree(ui, call, label, query),
        Expr::FunctionCall(call) => render_function_call_tree(ui, call, label, query),
        Expr::MethodCall(call) => render_method_call_tree(ui, call, label, query),
        Expr::MemberAccess(access) => render_member_access_tree(ui, access, label, query),
        Expr::New(new_expr) => render_new_expr_tree(ui, new_expr, label, query),
        Expr::DestructiveAssign(assign) => render_destructive_assign_tree(ui, assign, label, query),
        Expr::LetIn(let_in) => render_let_in_tree(ui, let_in, label, query),
        Expr::Block(block) => render_block_tree(ui, block, label, query),
        Expr::While(while_expr) => render_while_tree(ui, while_expr, label, query),
        Expr::If(if_expr) => render_if_tree(ui, if_expr, label, query),
        Expr::Is(is_expr) => render_is_tree(ui, is_expr, label, query),
        Expr::As(as_expr) => render_as_tree(ui, as_expr, label, query),
        Expr::For(for_expr) => {
            ui.label(match_rich_text(
                format!(
                    "{label}: for ({}) in ... [{}]",
                    for_expr.id,
                    span_text(for_expr.span)
                ),
                query,
            ));
        }
        Expr::BaseCall(call) => {
            ui.label(match_rich_text(
                format!(
                    "{label}: base({}) [{}]",
                    call.args.len(),
                    span_text(call.span)
                ),
                query,
            ));
            for (i, arg) in call.args.iter().enumerate() {
                CollapsingHeader::new(match_rich_text(format!("arg {i}"), query)).show(
                    ui,
                    |ui| {
                        render_expr_tree(ui, arg, "expr", query);
                    },
                );
            }
        }
    }
}

fn render_binary_tree(ui: &mut egui::Ui, binary: &BinaryExpr, label: &str, query: &str) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: Binary '{}' [{}]",
            binary_op_symbol(&binary.op),
            span_text(binary.span)
        ),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &binary.left, "left", query);
        render_expr_tree(ui, &binary.right, "right", query);
    });
}

fn render_unary_tree(ui: &mut egui::Ui, unary: &UnaryExpr, label: &str, query: &str) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: Unary '{}' [{}]",
            unary_op_symbol(&unary.op),
            span_text(unary.span)
        ),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &unary.expr, "expr", query);
    });
}

fn render_builtin_call_tree(ui: &mut egui::Ui, call: &BuiltinCallExpr, label: &str, query: &str) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: Builtin {}(...) [{}]",
            call.function.name(),
            span_text(call.span)
        ),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        if call.args.is_empty() {
            ui.small("Sin argumentos");
        }
        for (idx, arg) in call.args.iter().enumerate() {
            render_expr_tree(ui, arg, &format!("arg[{idx}]"), query);
        }
    });
}

fn render_function_call_tree(ui: &mut egui::Ui, call: &FunctionCallExpr, label: &str, query: &str) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: Call {}(...) [{}]",
            call.name,
            span_text(call.span)
        ),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        if call.args.is_empty() {
            ui.small("Sin argumentos");
        }
        for (idx, arg) in call.args.iter().enumerate() {
            render_expr_tree(ui, arg, &format!("arg[{idx}]"), query);
        }
    });
}

fn render_method_call_tree(ui: &mut egui::Ui, call: &MethodCallExpr, label: &str, query: &str) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: MethodCall .{}(...) [{}]",
            call.method_name,
            span_text(call.span)
        ),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &call.receiver, "receiver", query);
        if call.args.is_empty() {
            ui.small("Sin argumentos");
        }
        for (idx, arg) in call.args.iter().enumerate() {
            render_expr_tree(ui, arg, &format!("arg[{idx}]"), query);
        }
    });
}

fn render_member_access_tree(
    ui: &mut egui::Ui,
    access: &MemberAccessExpr,
    label: &str,
    query: &str,
) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: MemberAccess .{} [{}]",
            access.member,
            span_text(access.span)
        ),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &access.object, "object", query);
    });
}

fn render_new_expr_tree(ui: &mut egui::Ui, new_expr: &NewExpr, label: &str, query: &str) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: New {}(...) [{}]",
            new_expr.type_name,
            span_text(new_expr.span)
        ),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        if new_expr.args.is_empty() {
            ui.small("Sin argumentos");
        }
        for (idx, arg) in new_expr.args.iter().enumerate() {
            render_expr_tree(ui, arg, &format!("arg[{idx}]"), query);
        }
    });
}

fn render_destructive_assign_tree(
    ui: &mut egui::Ui,
    assign: &DestructiveAssignExpr,
    label: &str,
    query: &str,
) {
    let target_text = match &assign.target {
        AssignTarget::Variable { name, .. } => name.clone(),
        AssignTarget::Member { member, .. } => format!(".{}", member),
    };

    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: DestructiveAssign {} := ... [{}]",
            target_text,
            span_text(assign.span)
        ),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        match &assign.target {
            AssignTarget::Variable { name, .. } => {
                ui.small(match_rich_text(format!("target variable: {name}"), query));
            }
            AssignTarget::Member { object, member, .. } => {
                ui.small(match_rich_text(format!("target member: .{member}"), query));
                render_expr_tree(ui, object, "target object", query);
            }
        }
        render_expr_tree(ui, &assign.value, "value", query);
    });
}

fn render_let_in_tree(ui: &mut egui::Ui, let_in: &LetInExpr, label: &str, query: &str) {
    CollapsingHeader::new(match_rich_text(
        format!("{label}: LetIn [{}]", span_text(let_in.span)),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        CollapsingHeader::new(match_rich_text("bindings", query))
            .default_open(true)
            .show(ui, |ui| {
                if let_in.bindings.is_empty() {
                    ui.small("Sin bindings");
                }
                for (idx, binding) in let_in.bindings.iter().enumerate() {
                    CollapsingHeader::new(match_rich_text(
                        format!(
                            "binding[{idx}] {} [{}]",
                            binding.name,
                            span_text(binding.span)
                        ),
                        query,
                    ))
                    .default_open(false)
                    .show(ui, |ui| {
                        render_expr_tree(ui, &binding.value, "value", query);
                    });
                }
            });
        render_expr_tree(ui, &let_in.body, "body", query);
    });
}

fn render_block_tree(ui: &mut egui::Ui, block: &BlockExpr, label: &str, query: &str) {
    CollapsingHeader::new(match_rich_text(
        format!("{label}: Block [{}]", span_text(block.span)),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        if block.statements.is_empty() {
            ui.small("Bloque vacío");
        }
        for (idx, statement) in block.statements.iter().enumerate() {
            render_statement_tree(ui, statement, &format!("stmt[{idx}]"), query);
        }
    });
}

fn render_while_tree(ui: &mut egui::Ui, while_expr: &WhileExpr, label: &str, query: &str) {
    CollapsingHeader::new(match_rich_text(
        format!("{label}: While [{}]", span_text(while_expr.span)),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &while_expr.condition, "condition", query);
        render_block_tree(ui, &while_expr.body, "body", query);
    });
}

fn render_if_tree(ui: &mut egui::Ui, if_expr: &IfExpr, label: &str, query: &str) {
    CollapsingHeader::new(match_rich_text(
        format!("{label}: If [{}]", span_text(if_expr.span)),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &if_expr.condition, "condition", query);
        render_expr_tree(ui, &if_expr.then_branch, "then", query);

        if !if_expr.elif_branches.is_empty() {
            CollapsingHeader::new(match_rich_text("elif branches", query))
                .default_open(true)
                .show(ui, |ui| {
                    for (idx, branch) in if_expr.elif_branches.iter().enumerate() {
                        CollapsingHeader::new(match_rich_text(
                            format!("elif[{idx}] [{}]", span_text(branch.span)),
                            query,
                        ))
                        .default_open(false)
                        .show(ui, |ui| {
                            render_expr_tree(ui, &branch.condition, "condition", query);
                            render_expr_tree(ui, &branch.body, "body", query);
                        });
                    }
                });
        }

        render_expr_tree(ui, &if_expr.else_branch, "else", query);
    });
}

fn render_is_tree(ui: &mut egui::Ui, is_expr: &IsExpr, label: &str, query: &str) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: Is '{}' [{}]",
            is_expr.target_type,
            span_text(is_expr.span),
        ),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &is_expr.expr, "expr", query);
    });
}

fn render_as_tree(ui: &mut egui::Ui, as_expr: &AsExpr, label: &str, query: &str) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: As '{}' [{}]",
            as_expr.target_type,
            span_text(as_expr.span),
        ),
        query,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &as_expr.expr, "expr", query);
    });
}

fn literal_text(literal: &Literal) -> String {
    match literal {
        Literal::Integer(value) => format!("Integer({value})"),
        Literal::Float(value) => format!("Float({value})"),
        Literal::Boolean(value) => format!("Boolean({value})"),
        Literal::String(value) => format!("String(\"{}\")", value.replace('\n', "\\n")),
        Literal::Null => "Null".to_string(),
    }
}

fn binary_op_symbol(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Pow => "^",
        BinaryOp::Concat => "@",
        BinaryOp::ConcatSpace => "@@",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::Greater => ">",
        BinaryOp::LessEqual => "<=",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

fn unary_op_symbol(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
    }
}

fn span_text(span: Span) -> String {
    format!("{}..{}", span.start, span.end)
}

fn format_token(token: &Token) -> String {
    format!(
        "{:?} '{}' @ {}:{}",
        token.kind, token.value, token.line, token.column
    )
}

fn default_source() -> String {
    r#"
function fib(n) => if (n == 0) 0 elif (n == 1) 1 else fib(n - 1) + fib(n - 2);

let n = 8;
let value = fib(n);
print("fib(" @ n @ ") = " @ value);
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

fn install_vscode_extension() -> Result<String, String> {
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

fn run_program(lli_path: &str, ll_path: &PathBuf) -> Result<String, String> {
    if cfg!(target_os = "windows") {
        run_with_clang(ll_path)
    } else {
        run_with_lli(lli_path, ll_path)
    }
}
