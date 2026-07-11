//! Estado de la aplicación y layout de paneles.
//!
//! Estructura de la ventana:
//! ┌──────────── toolbar (branding · archivo · acciones · apariencia) ────────┐
//! ├──────────── pipeline visual: Lexer → Parser → Semántica → Codegen → Run ─┤
//! │ editor con gutter de líneas (central)        │ AST (panel derecho)       │
//! ├──────────── diagnósticos en pestañas: Errores·Tokens·IR·Salida ──────────┤
//! └──────────── barra de estado (semáforo + ⏱ tiempo + métricas) ────────────┘

use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    time::Instant,
};

use eframe::egui::{self, FontId, RichText, TextEdit, TextStyle, text::LayoutJob};
use hulk_compiler::compiler::{CompileOptions, Compiler, OutputKind};
use hulk_compiler::error::{CompilerError, ErrorCategory};
use hulk_compiler::lexer::Token;
use hulk_compiler::parser::expression::Program;

use crate::ast_view;
use crate::highlight::{classify_highlight_role, hulk_highlight_job, role_color};
use crate::runner;
use crate::theme::{Theme, ThemeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AstViewMode {
    Tree,
    DebugText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticsTab {
    Errors,
    Tokens,
    LlvmIr,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompileState {
    Idle,
    Success,
    Failure,
}

/// Estado visual de cada etapa del pipeline en la franja superior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhaseState {
    Pending,
    Ok,
    Failed,
}

const PHASES: [&str; 5] = ["Lexer", "Parser", "Semántica", "Codegen", "Run"];

pub struct HulkGui {
    source: String,
    status: String,
    compile_state: CompileState,
    last_compile_ms: Option<u128>,
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
    diagnostics_tab: DiagnosticsTab,
    theme_kind: ThemeKind,
    theme: Theme,
}

impl HulkGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme_kind = ThemeKind::HulkSmash;
        let theme = theme_kind.palette();
        theme.apply(&cc.egui_ctx);

        let example_files = runner::list_example_files();
        Self {
            source: runner::default_source(),
            status: "Listo para compilar".to_string(),
            compile_state: CompileState::Idle,
            last_compile_ms: None,
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
            diagnostics_tab: DiagnosticsTab::Errors,
            theme_kind,
            theme,
        }
    }

    fn set_theme(&mut self, ctx: &egui::Context, kind: ThemeKind) {
        self.theme_kind = kind;
        self.theme = kind.palette();
        self.theme.apply(ctx);
    }

    fn compile_source(&mut self) {
        let started = Instant::now();
        let mut compiler = Compiler::new();
        let options = CompileOptions {
            output_path: self.output_path.clone(),
        };
        let report = compiler.compile(&self.source, &options);
        self.last_compile_ms = Some(started.elapsed().as_millis());

        self.tokens = report.tokens;
        self.errors = report.errors;
        self.ast_program = report.ast;
        self.ast_text = self.ast_program.as_ref().map(|ast| format!("{:#?}", ast));
        self.ir_text = report.llvm_ir;
        self.exec_output.clear();

        if self.errors.is_empty() {
            self.compile_state = CompileState::Success;
            self.status = match report.output_kind {
                Some(OutputKind::LlvmIr) => {
                    self.exec_output = runner::run_program(&self.lli_path, &self.output_path)
                        .unwrap_or_else(|e| e);
                    self.diagnostics_tab = DiagnosticsTab::Output;
                    format!("LLVM IR generado en {}", self.output_path.display())
                }
                Some(OutputKind::Diagnostics) => "Solo se generó reporte de diagnóstico".to_string(),
                None => "Compilación completada".to_string(),
            };
        } else {
            self.compile_state = CompileState::Failure;
            self.diagnostics_tab = DiagnosticsTab::Errors;
            self.status = format!("Se encontraron {} error(es)", self.errors.len());
        }
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
                self.example_files = runner::list_example_files();
            }
            Err(err) => {
                self.status = format!("No se pudo guardar: {}", err);
            }
        }
    }

    /// Etapas del pipeline según el resultado de la última compilación.
    /// La primera categoría de error determina la fase que falló; las
    /// posteriores quedan pendientes.
    fn phase_states(&self) -> [PhaseState; 5] {
        match self.compile_state {
            CompileState::Idle => [PhaseState::Pending; 5],
            CompileState::Success => {
                let run = if self.exec_output.is_empty() {
                    PhaseState::Pending
                } else {
                    PhaseState::Ok
                };
                [
                    PhaseState::Ok,
                    PhaseState::Ok,
                    PhaseState::Ok,
                    PhaseState::Ok,
                    run,
                ]
            }
            CompileState::Failure => {
                let failed_at = match self.errors.first().map(|e| &e.category) {
                    Some(ErrorCategory::Lexical) => 0,
                    Some(ErrorCategory::Syntax) => 1,
                    Some(ErrorCategory::Type) | Some(ErrorCategory::Semantic) => 2,
                    None => 0,
                };
                let mut states = [PhaseState::Pending; 5];
                for (index, state) in states.iter_mut().enumerate() {
                    if index < failed_at {
                        *state = PhaseState::Ok;
                    } else if index == failed_at {
                        *state = PhaseState::Failed;
                    }
                }
                states
            }
        }
    }

    // ── Paneles ──────────────────────────────────────────────────────────

    fn show_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::default()
                    .fill(self.theme.bg_panel)
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("💪 HULK")
                            .color(self.theme.accent)
                            .strong()
                            .size(20.0),
                    );
                    ui.label(
                        RichText::new("compiler studio")
                            .color(self.theme.text_dim)
                            .italics(),
                    );
                    ui.separator();

                    // Archivo
                    egui::ComboBox::from_id_salt("example_select")
                        .width(190.0)
                        .selected_text(
                            self.input_path
                                .split('/')
                                .next_back()
                                .unwrap_or(&self.input_path)
                                .to_string(),
                        )
                        .show_ui(ui, |ui| {
                            for file in &self.example_files.clone() {
                                ui.selectable_value(&mut self.input_path, file.clone(), file);
                            }
                        });
                    if ui.button("📂 Cargar").clicked() {
                        self.load_from_file();
                    }
                    if ui.button("🔄").on_hover_text("Refrescar ejemplos").clicked() {
                        self.example_files = runner::list_example_files();
                    }

                    // Snippets de demostración por feature
                    ui.menu_button("⚡ Snippets", |ui| {
                        ui.label(
                            RichText::new("Demos de un click")
                                .color(self.theme.text_dim)
                                .small(),
                        );
                        ui.separator();
                        for (name, code) in runner::snippets() {
                            if ui.button(name).clicked() {
                                self.source = code.to_string();
                                self.status = format!("Snippet cargado: {name}");
                                ui.close();
                            }
                        }
                    });

                    ui.separator();

                    // Compilación (Ctrl/Cmd+Enter)
                    let compile_btn = egui::Button::new(
                        RichText::new("▶ Compilar")
                            .color(self.theme.bg_main)
                            .strong(),
                    )
                    .fill(self.theme.accent);
                    if ui
                        .add(compile_btn)
                        .on_hover_text("Atajo: Ctrl/Cmd + Enter")
                        .clicked()
                    {
                        self.compile_source();
                    }

                    if ui
                        .button(if self.focus_mode {
                            "🗖 Paneles"
                        } else {
                            "🎯 Enfoque"
                        })
                        .on_hover_text("Ocultar/mostrar paneles laterales")
                        .clicked()
                    {
                        self.focus_mode = !self.focus_mode;
                    }

                    ui.separator();

                    // Apariencia
                    egui::ComboBox::from_id_salt("theme_select")
                        .width(150.0)
                        .selected_text(self.theme_kind.label())
                        .show_ui(ui, |ui| {
                            for kind in ThemeKind::ALL {
                                if ui
                                    .selectable_label(self.theme_kind == kind, kind.label())
                                    .clicked()
                                {
                                    self.set_theme(ui.ctx(), kind);
                                }
                            }
                        });
                    ui.label("Aa");
                    ui.add(
                        egui::Slider::new(&mut self.editor_font_size, 13.0..=24.0)
                            .show_value(false),
                    );

                    ui.separator();

                    // Extras
                    ui.menu_button("⋯ Más", |ui| {
                        ui.label("Guardar como ejemplo:");
                        ui.text_edit_singleline(&mut self.new_example_name);
                        if ui.button("💾 Guardar en examples/").clicked() {
                            self.save_current_source_as_example();
                            ui.close();
                        }
                        ui.separator();
                        ui.label("Ruta de lli:");
                        ui.text_edit_singleline(&mut self.lli_path);
                        ui.separator();
                        if ui.button("🧩 Instalar hulk-vscode.vsix").clicked() {
                            match runner::install_vscode_extension() {
                                Ok(message) => self.status = message,
                                Err(err) => self.status = err,
                            }
                            ui.close();
                        }
                        if ui.button("📖 Guía VSCode").clicked() {
                            self.show_vscode_guide = true;
                            ui.close();
                        }
                        if ui.button("📚 Guía Hulk").clicked() {
                            self.show_tutorial = true;
                            ui.close();
                        }
                    });
                });
            });
    }

    /// Franja con las fases del pipeline: cada chip se pinta según el
    /// resultado de la última compilación.
    fn show_pipeline_strip(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("pipeline_strip")
            .frame(
                egui::Frame::default()
                    .fill(self.theme.bg_main)
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("PIPELINE")
                            .color(self.theme.text_dim)
                            .small()
                            .strong(),
                    );
                    ui.add_space(6.0);

                    for (index, (phase, state)) in
                        PHASES.iter().zip(self.phase_states()).enumerate()
                    {
                        if index > 0 {
                            ui.label(RichText::new("→").color(self.theme.text_dim));
                        }

                        let (fill, text_color, icon) = match state {
                            PhaseState::Pending => {
                                (self.theme.bg_input, self.theme.text_dim, "○")
                            }
                            PhaseState::Ok => (
                                self.theme.success.gamma_multiply(0.18),
                                self.theme.success,
                                "✔",
                            ),
                            PhaseState::Failed => (
                                self.theme.error.gamma_multiply(0.18),
                                self.theme.error,
                                "✘",
                            ),
                        };

                        egui::Frame::default()
                            .fill(fill)
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(egui::Margin::symmetric(10, 3))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("{icon} {phase}"))
                                        .color(text_color)
                                        .strong(),
                                );
                            });
                    }

                    if let Some(ms) = self.last_compile_ms {
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    RichText::new(format!("⏱ {ms} ms"))
                                        .color(self.theme.accent)
                                        .strong(),
                                );
                            },
                        );
                    }
                });
            });
    }

    fn show_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::default()
                    .fill(self.theme.bg_input)
                    .inner_margin(egui::Margin::symmetric(12, 5)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (dot_color, dot) = match self.compile_state {
                        CompileState::Idle => (self.theme.text_dim, "●"),
                        CompileState::Success => (self.theme.success, "●"),
                        CompileState::Failure => (self.theme.error, "●"),
                    };
                    ui.label(RichText::new(dot).color(dot_color));
                    ui.label(RichText::new(&self.status).color(self.theme.text));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} líneas", self.source.lines().count()))
                                .color(self.theme.text_dim),
                        );
                        ui.separator();
                        ui.label(
                            RichText::new(format!("{} tokens", self.tokens.len()))
                                .color(self.theme.text_dim),
                        );
                        ui.separator();
                        let error_color = if self.errors.is_empty() {
                            self.theme.text_dim
                        } else {
                            self.theme.error
                        };
                        ui.label(
                            RichText::new(format!("{} errores", self.errors.len()))
                                .color(error_color),
                        );
                        if let Some(ms) = self.last_compile_ms {
                            ui.separator();
                            ui.label(
                                RichText::new(format!("⏱ {ms} ms")).color(self.theme.text_dim),
                            );
                        }
                    });
                });
            });
    }

    /// Gutter de números de línea alineado con el editor; las líneas con
    /// errores se marcan en rojo y negrita.
    fn line_number_gutter(&self, font_size: f32) -> LayoutJob {
        let error_lines: HashSet<usize> = self.errors.iter().map(|e| e.line).collect();
        let line_count = self.source.lines().count().max(1);
        let width = line_count.to_string().len();

        let mut job = LayoutJob::default();
        for line in 1..=line_count {
            let is_error = error_lines.contains(&line);
            let color = if is_error {
                self.theme.error
            } else {
                self.theme.text_dim
            };
            let text = if line == line_count {
                format!("{line:>width$}")
            } else {
                format!("{line:>width$}\n")
            };
            job.append(
                &text,
                0.0,
                egui::TextFormat {
                    font_id: FontId::monospace(font_size),
                    color,
                    ..Default::default()
                },
            );
        }
        job
    }

    fn show_editor(&mut self, ui: &mut egui::Ui) {
        let font_size = self.editor_font_size;
        let theme = self.theme.clone();
        let mut layouter = move |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
            let mut job = hulk_highlight_job(text.as_str(), font_size, &theme);
            // Sin wrap: el gutter de líneas asume una fila visual por línea
            // lógica; el ScrollArea horizontal absorbe las líneas largas.
            job.wrap.max_width = f32::INFINITY;
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        };

        let gutter = self.line_number_gutter(font_size);

        egui::Frame::default()
            .fill(self.theme.bg_input)
            .stroke(egui::Stroke::new(
                1.0,
                self.theme.accent.gamma_multiply(0.25),
            ))
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                egui::ScrollArea::both()
                    .id_salt("source_editor_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            // El TextEdit aplica un pequeño margen interno;
                            // se compensa para alinear el gutter.
                            ui.vertical(|ui| {
                                ui.add_space(2.0);
                                ui.label(gutter);
                            });

                            let editor = TextEdit::multiline(&mut self.source)
                                .code_editor()
                                .desired_rows(48)
                                .desired_width(f32::INFINITY)
                                .font(TextStyle::Monospace)
                                .lock_focus(true)
                                .layouter(&mut layouter);
                            ui.add_sized(
                                egui::vec2(
                                    ui.available_width().max(400.0),
                                    ui.available_height(),
                                ),
                                editor,
                            );
                        });
                    });
            });
    }

    fn show_ast_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("ast_panel")
            .resizable(true)
            .default_width(520.0)
            .min_width(360.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("🌳 AST")
                            .color(self.theme.accent)
                            .strong()
                            .size(16.0),
                    );
                    ui.separator();
                    ui.selectable_value(&mut self.ast_view_mode, AstViewMode::Tree, "Árbol");
                    ui.selectable_value(&mut self.ast_view_mode, AstViewMode::DebugText, "Debug");
                });
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.ast_search_query)
                            .hint_text("nodo, variable, function, call..."),
                    );
                    if ui.button("✕").on_hover_text("Limpiar búsqueda").clicked() {
                        self.ast_search_query.clear();
                    }
                });
                ui.separator();

                match self.ast_view_mode {
                    AstViewMode::Tree => {
                        if let Some(program) = &self.ast_program {
                            let query = self.ast_search_query.trim();
                            if !query.is_empty() {
                                let matches = ast_view::count_ast_matches(program, query);
                                ui.small(format!("Coincidencias estimadas: {matches}"));
                            }
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ast_view::render_program_tree(ui, program, query, &self.theme);
                            });
                        } else {
                            ui.label("Sin AST disponible. Compila para generarlo.");
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
                            ui.label("Sin AST disponible. Compila para generarlo.");
                        }
                    }
                }
            });
    }

    fn show_diagnostics_bottom(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("diagnostics_bottom")
            .resizable(true)
            .default_height(280.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let error_label = if self.errors.is_empty() {
                        "✔ Errores".to_string()
                    } else {
                        format!("❗ Errores ({})", self.errors.len())
                    };
                    ui.selectable_value(
                        &mut self.diagnostics_tab,
                        DiagnosticsTab::Errors,
                        error_label,
                    );
                    ui.selectable_value(
                        &mut self.diagnostics_tab,
                        DiagnosticsTab::Tokens,
                        format!("🔤 Tokens ({})", self.tokens.len()),
                    );
                    ui.selectable_value(&mut self.diagnostics_tab, DiagnosticsTab::LlvmIr, "⚙ LLVM IR");
                    ui.selectable_value(
                        &mut self.diagnostics_tab,
                        DiagnosticsTab::Output,
                        "🖥 Salida",
                    );
                });
                ui.separator();

                match self.diagnostics_tab {
                    DiagnosticsTab::Errors => self.show_errors_tab(ui),
                    DiagnosticsTab::Tokens => self.show_tokens_tab(ui),
                    DiagnosticsTab::LlvmIr => self.show_ir_tab(ui),
                    DiagnosticsTab::Output => self.show_output_tab(ui),
                }
            });
    }

    /// Errores como tarjetas con badge de categoría y ubicación.
    fn show_errors_tab(&self, ui: &mut egui::Ui) {
        if self.errors.is_empty() {
            ui.label(RichText::new("Sin errores ✔").color(self.theme.success));
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for error in &self.errors {
                let badge = match error.category {
                    ErrorCategory::Lexical => "LEXICAL",
                    ErrorCategory::Syntax => "SYNTACTIC",
                    ErrorCategory::Type => "TYPE",
                    ErrorCategory::Semantic => "SEMANTIC",
                };
                egui::Frame::default()
                    .fill(self.theme.error.gamma_multiply(0.08))
                    .stroke(egui::Stroke::new(1.0, self.theme.error.gamma_multiply(0.6)))
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(badge)
                                    .color(self.theme.error)
                                    .strong()
                                    .small(),
                            );
                            ui.label(
                                RichText::new(format!("línea {}, col {}", error.line, error.column))
                                    .color(self.theme.text_dim)
                                    .small(),
                            );
                        });
                        ui.label(RichText::new(&error.message).color(self.theme.text).monospace());
                    });
                ui.add_space(4.0);
            }
        });
    }

    /// Tokens como tabla coloreada por rol sintáctico.
    fn show_tokens_tab(&self, ui: &mut egui::Ui) {
        if self.tokens.is_empty() {
            ui.label("Compila para ver los tokens.");
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("tokens_grid")
                .striped(true)
                .spacing(egui::vec2(18.0, 3.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("#").color(self.theme.text_dim).strong());
                    ui.label(RichText::new("token").color(self.theme.text_dim).strong());
                    ui.label(RichText::new("valor").color(self.theme.text_dim).strong());
                    ui.label(RichText::new("posición").color(self.theme.text_dim).strong());
                    ui.end_row();

                    for (index, token) in self.tokens.iter().enumerate() {
                        let role = classify_highlight_role(&self.tokens, index);
                        let color = role_color(role, &self.theme);
                        ui.label(
                            RichText::new(index.to_string())
                                .color(self.theme.text_dim)
                                .monospace(),
                        );
                        ui.label(
                            RichText::new(format!("{:?}", token.kind))
                                .color(color)
                                .monospace(),
                        );
                        ui.label(RichText::new(&token.value).color(self.theme.text).monospace());
                        ui.label(
                            RichText::new(format!("{}:{}", token.line, token.column))
                                .color(self.theme.text_dim)
                                .monospace(),
                        );
                        ui.end_row();
                    }
                });
        });
    }

    fn show_ir_tab(&mut self, ui: &mut egui::Ui) {
        if let Some(ir) = &self.ir_text.clone() {
            ui.horizontal(|ui| {
                ui.small(format!("Archivo de salida: {}", self.output_path.display()));
                if ui.button("📋 Copiar IR").clicked() {
                    ui.ctx().copy_text(ir.clone());
                    self.status = "LLVM IR copiado al portapapeles".to_string();
                }
            });
            let mut ir_display = ir.clone();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut ir_display)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            });
        } else {
            ui.label("Compila para generar IR.");
        }
    }

    fn show_output_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("↻ Re-ejecutar").clicked() {
                self.exec_output =
                    runner::run_program(&self.lli_path, &self.output_path).unwrap_or_else(|e| e);
            }
            if !self.exec_output.is_empty() && ui.button("📋 Copiar").clicked() {
                let output = self.exec_output.clone();
                ui.ctx().copy_text(output);
                self.status = "Salida copiada al portapapeles".to_string();
            }
        });
        if self.exec_output.is_empty() {
            ui.label("Compila para ejecutar y ver la salida del programa.");
            return;
        }
        let mut out = self.exec_output.clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                TextEdit::multiline(&mut out)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .interactive(false),
            );
        });
    }

    fn show_help_windows(&mut self, ctx: &egui::Context) {
        if self.show_tutorial {
            egui::Window::new("Guía rápida de sintaxis Hulk")
                .resizable(true)
                .collapsible(true)
                .open(&mut self.show_tutorial)
                .show(ctx, |ui| {
                    ui.label("Conceptos clave:");
                    ui.monospace("- El lenguaje es basado en expresiones; el último ';' es opcional.");
                    ui.monospace("- Bloques y let-in devuelven el valor de su última expresión.");
                    ui.monospace("- Declaración: let x = expr;");
                    ui.monospace("- Asignación destructiva: x := expr (mismo tipo).");
                    ui.monospace("- Builtins: sin, cos, sqrt, exp, log(base, value), rand().");
                    ui.monospace("- Constantes: PI, E.");
                    ui.monospace("- Operadores: + - * / ^ @ && || ! == != < > <= >=.");
                    ui.monospace("- Arreglos: let a: Number[] = {1, 2, 3}; a[0]; a.size(); new Number[5].");
                    ui.monospace("- Lambdas: function (x: Number): Number -> x * 2.");
                    ui.monospace("- Macros: define doble(x: Number): Number -> x * 2;");
                    ui.monospace("- Identificadores: empiezan con letra; luego letras, dígitos, '_'.");
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
                    ui.monospace("4) Abre la carpeta del proyecto en VSCode para usar tema/syntax allí.");
                    ui.separator();
                    ui.label("Nota: esta GUI usa su propio resaltado, no el motor de VSCode.");
                });
        }
    }
}

impl eframe::App for HulkGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Atajo global: Ctrl/Cmd + Enter compila.
        if ctx.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command) {
            self.compile_source();
        }

        self.show_toolbar(ctx);
        self.show_pipeline_strip(ctx);
        self.show_status_bar(ctx);

        if !self.focus_mode {
            self.show_ast_panel(ctx);
            self.show_diagnostics_bottom(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_editor(ui);
        });

        self.show_help_windows(ctx);
    }
}