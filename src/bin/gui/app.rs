//! Estado de la aplicación y layout de paneles.
//!
//! Estructura de la ventana:
//! ┌──────────── toolbar (branding · archivo · acciones · apariencia) ────────┐
//! ├──────────── pipeline visual: Lexer → Parser → Semántica → Codegen → Run ─┤
//! │ editor con gutter de líneas (central)        │ AST (panel derecho)       │
//! ├─────────── diagnósticos en pestañas: Errores·Tokens·IR ──────────────────┤
//! ├──────────── 🖥 TERMINAL (panel dedicado para salida del programa) ────────┤
//! └──────────── barra de estado (semáforo + ⏱ tiempo + métricas) ────────────┘

use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    time::Instant,
};

use eframe::egui::{self, FontId, RichText, TextEdit, TextStyle, text::LayoutJob, Key, text_selection::CCursorRange};

const HULK_KEYWORDS: &[&str] = &[
    "let", "in", "function", "if", "elif", "else", "while", "for",
    "print", "null", "true", "false", "new", "type", "interface",
    "extends", "inherit", "define",
    "Number", "String", "Boolean",
    "sin", "cos", "sqrt", "exp", "log", "rand",
    "PI", "E", "self", "base",
];
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
    exec_result: Option<Result<runner::ProgramOutput, String>>,
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
    ac_suggestions: Vec<String>,
    ac_index: usize,
    ac_word: String,
    ac_prev_source: String,
    ac_cursor_target: Option<usize>,
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
            exec_result: None,
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
            ac_suggestions: Vec::new(),
            ac_index: 0,
            ac_word: String::new(),
            ac_prev_source: String::new(),
            ac_cursor_target: None,
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
        self.exec_result = None;

        if self.errors.is_empty() {
            self.compile_state = CompileState::Success;
            self.status = match report.output_kind {
                Some(OutputKind::LlvmIr) => {
                    self.exec_result =
                        Some(runner::run_program_clean(&self.lli_path, &self.output_path));
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
                let run = match &self.exec_result {
                    None => PhaseState::Pending,
                    Some(Ok(output)) if output.is_success() => PhaseState::Ok,
                    Some(_) => PhaseState::Failed,
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

                    egui::ComboBox::from_id_salt("example_select")
                        .width(160.0)
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
                    if ui.button("📂").on_hover_text("Cargar archivo").clicked() {
                        self.load_from_file();
                    }
                    if ui.button("🔄").on_hover_text("Refrescar ejemplos").clicked() {
                        self.example_files = runner::list_example_files();
                    }
                    ui.menu_button("⚡", |ui| {
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
                        .button(if self.focus_mode { "🗖 Paneles" } else { "🎯 Enfoque" })
                        .on_hover_text("Ocultar/mostrar paneles laterales")
                        .clicked()
                    {
                        self.focus_mode = !self.focus_mode;
                    }

                    ui.separator();

                    egui::ComboBox::from_id_salt("theme_select")
                        .width(140.0)
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

    fn show_pipeline_strip(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("pipeline_strip")
            .frame(
                egui::Frame::default()
                    .fill(self.theme.bg_main)
                    .inner_margin(egui::Margin::symmetric(12, 7)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("PIPELINE")
                            .color(self.theme.text_dim)
                            .small()
                            .strong(),
                    );
                    ui.add_space(8.0);

                    for (index, (phase, state)) in
                        PHASES.iter().zip(self.phase_states()).enumerate()
                    {
                        if index > 0 {
                            ui.label(
                                RichText::new("→")
                                    .color(self.theme.text_dim.gamma_multiply(0.5))
                                    .small(),
                            );
                            ui.add_space(2.0);
                        }

                        let (fill, text_color, icon) = match state {
                            PhaseState::Pending => (
                                self.theme.bg_input.gamma_multiply(0.8),
                                self.theme.text_dim,
                                "○",
                            ),
                            PhaseState::Ok => (
                                self.theme.success.gamma_multiply(0.15),
                                self.theme.success,
                                "✔",
                            ),
                            PhaseState::Failed => (
                                self.theme.error.gamma_multiply(0.15),
                                self.theme.error,
                                "✘",
                            ),
                        };

                        egui::Frame::default()
                            .fill(fill)
                            .corner_radius(egui::CornerRadius::same(12))
                            .inner_margin(egui::Margin::symmetric(10, 4))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("{icon} {phase}"))
                                        .color(text_color)
                                        .strong()
                                        .size(13.0),
                                );
                            });
                    }

                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if let Some(ms) = self.last_compile_ms {
                                egui::Frame::default()
                                    .fill(self.theme.accent.gamma_multiply(0.12))
                                    .corner_radius(egui::CornerRadius::same(8))
                                    .inner_margin(egui::Margin::symmetric(8, 3))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(format!("⏱ {ms} ms"))
                                                .color(self.theme.accent)
                                                .strong()
                                                .small(),
                                        );
                                    });
                            }
                        },
                    );
                });
            });
    }

    fn show_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::default()
                    .fill(self.theme.bg_panel)
                    .inner_margin(egui::Margin::symmetric(12, 5)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (dot_color, dot) = match self.compile_state {
                        CompileState::Idle => (self.theme.text_dim, "●"),
                        CompileState::Success => (self.theme.success, "●"),
                        CompileState::Failure => (self.theme.error, "●"),
                    };
                    ui.label(RichText::new(dot).color(dot_color).size(10.0));
                    ui.label(RichText::new(&self.status).color(self.theme.text).small());

                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if let Some(ms) = self.last_compile_ms {
                                ui.label(
                                    RichText::new(format!("⏱ {ms} ms"))
                                        .color(self.theme.accent)
                                        .small(),
                                );
                                ui.add(egui::Separator::default().spacing(8.0));
                            }
                            let error_color = if self.errors.is_empty() {
                                self.theme.text_dim
                            } else {
                                self.theme.error
                            };
                            ui.label(
                                RichText::new(format!("{} errores", self.errors.len()))
                                    .color(error_color)
                                    .small(),
                            );
                            ui.add(egui::Separator::default().spacing(8.0));
                            ui.label(
                                RichText::new(format!("{} tokens", self.tokens.len()))
                                    .color(self.theme.text_dim)
                                    .small(),
                            );
                            ui.add(egui::Separator::default().spacing(8.0));
                            ui.label(
                                RichText::new(format!("{} líneas", self.source.lines().count()))
                                    .color(self.theme.text_dim)
                                    .small(),
                            );
                        },
                    );
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
            job.wrap.max_width = f32::INFINITY;
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        };

        let gutter = self.line_number_gutter(font_size);

        egui::Frame::default()
            .fill(self.theme.bg_input)
            .stroke(egui::Stroke::new(
                1.0,
                self.theme.accent.gamma_multiply(0.2),
            ))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                egui::ScrollArea::both()
                    .id_salt("source_editor_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            ui.vertical(|ui| {
                                ui.add_space(2.0);
                                ui.label(gutter);
                            });

                            let editor_id = egui::Id::new("source_editor");
                            let editor = TextEdit::multiline(&mut self.source)
                                .id(editor_id)
                                .code_editor()
                                .desired_rows(48)
                                .desired_width(f32::INFINITY)
                                .font(TextStyle::Monospace)
                                .lock_focus(true)
                                .layouter(&mut layouter);
                            let resp = ui.add_sized(
                                egui::vec2(
                                    ui.available_width().max(400.0),
                                    ui.available_height(),
                                ),
                                editor,
                            );
                            if resp.has_focus() {
                                if let Some(target) = self.ac_cursor_target.take() {
                                    if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), editor_id) {
                                        use egui::epaint::text::cursor::CCursor;
                                        let cc = CCursor { index: target, prefer_next_row: false };
                                        state.cursor.set_char_range(Some(CCursorRange::one(cc)));
                                        egui::TextEdit::store_state(ui.ctx(), editor_id, state);
                                    }
                                }
                            }
                        });
                    });
            });
    }

    fn show_ast_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("ast_panel")
            .resizable(true)
            .default_width(520.0)
            .min_width(360.0)
            .frame(
                egui::Frame::default()
                    .fill(self.theme.bg_panel)
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("🌳 AST")
                            .color(self.theme.accent)
                            .strong()
                            .size(16.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.selectable_value(&mut self.ast_view_mode, AstViewMode::DebugText, "Debug");
                        ui.selectable_value(&mut self.ast_view_mode, AstViewMode::Tree, "Árbol");
                    });
                });
                ui.add_space(4.0);
                egui::Frame::default()
                    .fill(self.theme.bg_input)
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("🔍");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.ast_search_query)
                                    .hint_text("nodo, variable, function, call...")
                                    .desired_width(f32::INFINITY),
                            );
                            if ui.button("✕").on_hover_text("Limpiar búsqueda").clicked() {
                                self.ast_search_query.clear();
                            }
                        });
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
            .default_height(180.0)
            .min_height(90.0)
            .frame(
                egui::Frame::default()
                    .fill(self.theme.bg_panel)
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(self.theme.bg_input)
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            let error_label = if self.errors.is_empty() {
                                "✔ Errores".to_string()
                            } else {
                                format!("❗ Errores ({})", self.errors.len())
                            };
                            Self::tab_button(ui, &mut self.diagnostics_tab, DiagnosticsTab::Errors, &error_label, &self.theme);
                            Self::tab_button(ui, &mut self.diagnostics_tab, DiagnosticsTab::Tokens, &format!("🔤 Tokens ({})", self.tokens.len()), &self.theme);
                            Self::tab_button(ui, &mut self.diagnostics_tab, DiagnosticsTab::LlvmIr, "⚙ LLVM IR", &self.theme);
                        });
                    });
                ui.add_space(6.0);

                match self.diagnostics_tab {
                    DiagnosticsTab::Errors => self.show_errors_tab(ui),
                    DiagnosticsTab::Tokens => self.show_tokens_tab(ui),
                    DiagnosticsTab::LlvmIr => self.show_ir_tab(ui),
                }
            });
    }

    fn tab_button(ui: &mut egui::Ui, current: &mut DiagnosticsTab, tab: DiagnosticsTab, label: &str, theme: &Theme) {
        let selected = *current == tab;
        let fill = if selected {
            theme.accent.gamma_multiply(0.2)
        } else {
            egui::Color32::TRANSPARENT
        };
        let text_color = if selected { theme.accent } else { theme.text_dim };
        egui::Frame::default()
            .fill(fill)
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(12, 5))
            .show(ui, |ui| {
                ui.label(RichText::new(label).color(text_color).strong());
            })
            .response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
            .then(|| *current = tab);
    }

    fn show_errors_tab(&self, ui: &mut egui::Ui) {
        if self.errors.is_empty() {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("✔ Sin errores").color(self.theme.success).strong());
            });
            return;
        }

        let mut phase_errors: Vec<(&str, Vec<&CompilerError>, egui::Color32)> = Vec::new();

        for error in &self.errors {
            let label = match error.category {
                ErrorCategory::Lexical => "ANÁLISIS LÉXICO",
                ErrorCategory::Syntax => "ANÁLISIS SINTÁCTICO",
                ErrorCategory::Type => "VERIFICACIÓN DE TIPOS",
                ErrorCategory::Semantic => "ANÁLISIS SEMÁNTICO",
            };
            let color = match error.category {
                ErrorCategory::Lexical => egui::Color32::from_rgb(255, 140, 50),
                ErrorCategory::Syntax => egui::Color32::from_rgb(255, 80, 180),
                ErrorCategory::Type => egui::Color32::from_rgb(80, 180, 255),
                ErrorCategory::Semantic => egui::Color32::from_rgb(180, 130, 255),
            };

            if let Some((_, list, _)) = phase_errors.iter_mut().find(|(l, _, _)| *l == label) {
                list.push(error);
            } else {
                phase_errors.push((label, vec![error], color));
            }
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);

            for (phase_name, errors, color) in &phase_errors {
                egui::Frame::default()
                    .fill(color.gamma_multiply(0.08))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            egui::Frame::default()
                                .fill(color.gamma_multiply(0.25))
                                .corner_radius(egui::CornerRadius::same(6))
                                .inner_margin(egui::Margin::symmetric(10, 4))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!("{} ({})", phase_name, errors.len()))
                                            .color(*color)
                                            .strong()
                                            .size(14.0),
                                    );
                                });
                        });
                    });
                ui.add_space(6.0);

                for error in errors {
                    egui::Frame::default()
                        .fill(self.theme.error.gamma_multiply(0.06))
                        .stroke(egui::Stroke::new(1.0, self.theme.error.gamma_multiply(0.4)))
                        .corner_radius(egui::CornerRadius::same(8))
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                egui::Frame::default()
                                    .fill(color.gamma_multiply(0.2))
                                    .corner_radius(egui::CornerRadius::same(4))
                                    .inner_margin(egui::Margin::symmetric(6, 2))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new("ERROR")
                                                .color(*color)
                                                .strong()
                                                .small()
                                                .monospace(),
                                        );
                                    });
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(format!("{}:{}", error.line, error.column))
                                        .color(self.theme.text_dim)
                                        .small()
                                        .monospace(),
                                );
                            });
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(error.message.as_str())
                                    .color(self.theme.text)
                                    .monospace(),
                            );
                        });
                    ui.add_space(6.0);
                }
            }
        });
    }

    fn show_tokens_tab(&self, ui: &mut egui::Ui) {
        if self.tokens.is_empty() {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("Compila para ver los tokens.").color(self.theme.text_dim));
            });
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            egui::Grid::new("tokens_grid")
                .striped(true)
                .spacing(egui::vec2(18.0, 4.0))
                .show(ui, |ui| {
                    ui.add(egui::Label::new(
                        RichText::new("#").color(self.theme.text_dim).strong().small()
                    ));
                    ui.add(egui::Label::new(
                        RichText::new("token").color(self.theme.text_dim).strong().small()
                    ));
                    ui.add(egui::Label::new(
                        RichText::new("valor").color(self.theme.text_dim).strong().small()
                    ));
                    ui.add(egui::Label::new(
                        RichText::new("posición").color(self.theme.text_dim).strong().small()
                    ));
                    ui.end_row();

                    for (index, token) in self.tokens.iter().enumerate() {
                        let role = classify_highlight_role(&self.tokens, index);
                        let color = role_color(role, &self.theme);
                        ui.label(
                            RichText::new(index.to_string())
                                .color(self.theme.text_dim)
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            RichText::new(format!("{:?}", token.kind))
                                .color(color)
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            RichText::new(&token.value)
                                .color(self.theme.text)
                                .monospace()
                                .small(),
                        );
                        ui.label(
                            RichText::new(format!("{}:{}", token.line, token.column))
                                .color(self.theme.text_dim)
                                .monospace()
                                .small(),
                        );
                        ui.end_row();
                    }
                });
        });
    }

    fn show_ir_tab(&mut self, ui: &mut egui::Ui) {
        if let Some(ir) = &self.ir_text.clone() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("📄 {}", self.output_path.display()))
                        .color(self.theme.text_dim)
                        .small(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📋 Copiar IR").on_hover_text("Copiar LLVM IR al portapapeles").clicked() {
                        ui.ctx().copy_text(ir.clone());
                        self.status = "LLVM IR copiado al portapapeles".to_string();
                    }
                });
            });
            ui.add_space(6.0);
            let mut ir_display = ir.clone();
            egui::Frame::default()
                .fill(self.theme.bg_input)
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add(
                            TextEdit::multiline(&mut ir_display)
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .interactive(false),
                        );
                    });
                });
        } else {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("Compila para generar IR.").color(self.theme.text_dim));
            });
        }
    }

    fn show_terminal_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("terminal_panel")
            .resizable(true)
            .default_height(300.0)
            .min_height(120.0)
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(self.theme.terminal_bg)
                    .stroke(egui::Stroke::new(
                        1.0,
                        self.theme.accent.gamma_multiply(0.3),
                    ))
                    .corner_radius(egui::CornerRadius::same(10))
                    .show(ui, |ui| {
                        egui::Frame::default()
                            .fill(self.theme.bg_panel)
                            .corner_radius(egui::CornerRadius {
                                nw: 10,
                                ne: 10,
                                sw: 0,
                                se: 0,
                            })
                            .inner_margin(egui::Margin::symmetric(12, 7))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let dot = |c: egui::Color32| {
                                        RichText::new("●").color(c).size(12.0)
                                    };
                                    ui.label(dot(egui::Color32::from_rgb(255, 95, 86)));
                                    ui.label(dot(egui::Color32::from_rgb(255, 189, 46)));
                                    ui.label(dot(egui::Color32::from_rgb(39, 201, 63)));
                                    ui.add_space(8.0);
                                    ui.label(
                                        RichText::new("Salida del programa")
                                            .color(self.theme.text_dim)
                                            .small(),
                                    );

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if let Some(Ok(output)) = &self.exec_result
                                                && !output.stdout.is_empty()
                                            {
                                                let text = output.stdout.clone();
                                                if ui.button("📋 Copiar").on_hover_text("Copiar salida").clicked() {
                                                    ui.ctx().copy_text(text);
                                                    self.status = "Salida copiada".to_string();
                                                }
                                            }
                                            if ui.button("🖥 Terminal").on_hover_text("Abrir en terminal del sistema").clicked() {
                                                match runner::open_in_external_terminal(
                                                    &self.lli_path, &self.output_path,
                                                ) {
                                                    Ok(m) => self.status = m,
                                                    Err(e) => self.status = e,
                                                }
                                            }
                                            if ui.button("↻ Re-ejecutar").on_hover_text("Volver a ejecutar el IR").clicked() {
                                                self.exec_result = Some(
                                                    runner::run_program_clean(
                                                        &self.lli_path, &self.output_path,
                                                    ),
                                                );
                                            }
                                        },
                                    );
                                });
                            });

                        egui::Frame::default()
                            .fill(self.theme.terminal_bg)
                            .inner_margin(egui::Margin::symmetric(14, 12))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("terminal_panel_scroll")
                                    .auto_shrink([false, false])
                                    .min_scrolled_height(80.0)
                                    .show(ui, |ui| {
                                        ui.set_min_width(ui.available_width());
                                        self.render_terminal_body(ui);
                                    });
                            });
                    });
            });
    }

    fn render_terminal_body(&self, ui: &mut egui::Ui) {
        if !self.errors.is_empty() {
            self.terminal_prompt_line(ui, "compile — errores");
            for error in &self.errors {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{}:{}", error.line, error.column))
                            .color(self.theme.text_dim)
                            .small()
                            .monospace(),
                    );
                    ui.label(
                        RichText::new(&error.message)
                            .color(self.theme.error)
                            .monospace(),
                    );
                });
            }
            return;
        }

        match &self.exec_result {
            None => {
                self.terminal_prompt_line(ui, "esperando compilación…");
                ui.add(
                    egui::Label::new(
                        RichText::new("Compila para ejecutar y ver aquí la salida del programa.")
                            .color(self.theme.text_dim)
                            .monospace(),
                    )
                    .selectable(false),
                );
            }
            Some(Err(err)) => {
                self.terminal_prompt_line(ui, "run");
                ui.add(
                    egui::Label::new(
                        RichText::new(err.trim_end())
                            .color(self.theme.error)
                            .monospace(),
                    )
                    .selectable(true),
                );
            }
            Some(Ok(output)) => {
                self.terminal_prompt_line(ui, &output.command);

                let has_stdout = !output.stdout.trim().is_empty();
                let has_stderr = !output.stderr.trim().is_empty();

                if has_stdout {
                    ui.add(
                        egui::Label::new(
                            RichText::new(output.stdout.trim_end())
                                .color(self.theme.terminal_text)
                                .monospace(),
                        )
                        .selectable(true),
                    );
                }
                if has_stderr {
                    ui.add_space(4.0);
                    ui.add(
                        egui::Label::new(
                            RichText::new(output.stderr.trim_end())
                                .color(self.theme.error)
                                .monospace(),
                        )
                        .selectable(true),
                    );
                }
                if !has_stdout && !has_stderr {
                    ui.add(
                        egui::Label::new(
                            RichText::new("(sin salida)")
                                .color(self.theme.text_dim)
                                .italics()
                                .monospace(),
                        )
                        .selectable(false),
                    );
                }

                ui.add_space(10.0);
                self.exit_code_chip(ui, output);
            }
        }
    }

    /// Línea de prompt tipo shell: `➜ hulk <comando>`.
    fn terminal_prompt_line(&self, ui: &mut egui::Ui, command: &str) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(
                RichText::new("➜")
                    .color(self.theme.prompt)
                    .strong()
                    .monospace(),
            );
            ui.label(RichText::new("hulk").color(self.theme.accent).monospace());
            ui.label(
                RichText::new(command)
                    .color(self.theme.text_dim)
                    .monospace(),
            );
        });
        ui.add_space(6.0);
    }

    /// Chip con el código de salida del proceso.
    fn exit_code_chip(&self, ui: &mut egui::Ui, output: &runner::ProgramOutput) {
        let (fill, text_color, label) = if output.is_success() {
            (
                self.theme.success.gamma_multiply(0.18),
                self.theme.success,
                "● exit 0".to_string(),
            )
        } else {
            let code = match output.exit_code {
                Some(code) => format!("exit {code}"),
                None => "exit ?".to_string(),
            };
            (
                self.theme.error.gamma_multiply(0.18),
                self.theme.error,
                format!("● {code}"),
            )
        };
        egui::Frame::default()
            .fill(fill)
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(10, 3))
            .show(ui, |ui| {
                ui.label(RichText::new(label).color(text_color).monospace().small());
            });
    }

    // ── Autocompletado ──────────────────────────────────────────────────

    fn update_ac(&mut self) {
        let word = Self::last_word(&self.source);
        self.ac_word = word.clone();
        self.ac_prev_source = self.source.clone();

        if word.is_empty() || word.len() < 1 {
            self.ac_suggestions.clear();
            return;
        }

        let word_lower = word.to_lowercase();
        let mut suggestions: Vec<String> = Vec::new();

        for kw in HULK_KEYWORDS {
            if kw.starts_with(&word_lower) && *kw != word_lower {
                suggestions.push(kw.to_string());
            }
        }

        let ids = Self::source_identifiers(&self.source);
        for id in &ids {
            let id_lower = id.to_lowercase();
            if id_lower.starts_with(&word_lower)
                && id_lower != word_lower
                && !suggestions.contains(id)
            {
                suggestions.push(id.clone());
            }
        }

        suggestions.sort();
        suggestions.dedup();

        if suggestions.len() > 10 {
            suggestions.truncate(10);
        }

        self.ac_suggestions = suggestions;
        self.ac_index = 0;
    }

    fn last_word(source: &str) -> String {
        let s = source.trim_end();
        if s.is_empty() {
            return String::new();
        }
        let mut end = s.len();
        while end > 0 {
            let c = s[..end].chars().last().unwrap();
            if c.is_alphanumeric() || c == '_' {
                break;
            }
            end -= c.len_utf8();
        }
        if end == 0 {
            return String::new();
        }
        let mut start = end;
        while start > 0 {
            let c = s[..start].chars().last().unwrap();
            if !c.is_alphanumeric() && c != '_' {
                break;
            }
            start -= c.len_utf8();
        }
        s[start..end].to_string()
    }

    fn source_identifiers(source: &str) -> Vec<String> {
        let mut ids: Vec<String> = Vec::new();
        let mut current = String::new();
        for c in source.chars() {
            if c.is_alphanumeric() || c == '_' {
                current.push(c);
            } else {
                if current.len() >= 2
                    && !current.starts_with(|c: char| c.is_ascii_digit())
                {
                    ids.push(current.clone());
                }
                current.clear();
            }
        }
        if current.len() >= 2
            && !current.starts_with(|c: char| c.is_ascii_digit())
        {
            ids.push(current);
        }
        ids.sort();
        ids.dedup();
        ids
    }

    fn accept_ac(&mut self, suggestion: &str) {
        let word = &self.ac_word;
        if word.is_empty() {
            return;
        }
        if let Some(pos) = self.source.rfind(word) {
            let end = pos + suggestion.len();
            self.source.replace_range(pos..pos + word.len(), suggestion);
            self.ac_cursor_target = Some(end);
        }
        self.ac_suggestions.clear();
        self.ac_word.clear();
    }

    fn show_ac_popup(&mut self, ctx: &egui::Context) {
        if self.ac_suggestions.is_empty() || self.ac_word.is_empty() {
            return;
        }

        let suggestions = self.ac_suggestions.clone();
        let ac_index = self.ac_index;
        let theme = self.theme.clone();

        egui::Area::new(egui::Id::new("ac_popup"))
            .fixed_pos(egui::pos2(420.0, 520.0))
            .show(ctx, |ui| {
                egui::Frame::default()
                    .fill(theme.bg_panel)
                    .stroke(egui::Stroke::new(1.0, theme.accent))
                    .corner_radius(egui::CornerRadius::same(8))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 4],
                        blur: 16,
                        spread: 0,
                        color: egui::Color32::BLACK.gamma_multiply(0.4),
                    })
                    .inner_margin(egui::Margin::symmetric(4, 4))
                    .show(ui, |ui| {
                        ui.set_min_width(220.0);
                        for (i, suggestion) in suggestions.iter().enumerate() {
                            let selected = i == ac_index;
                            let fill = if selected {
                                theme.accent.gamma_multiply(0.2)
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            let text_color = if selected {
                                theme.accent
                            } else {
                                theme.text
                            };

                            let is_keyword = HULK_KEYWORDS.contains(&suggestion.as_str());
                            let prefix = if is_keyword { "⚡ " } else { "🏷 " };

                            let s = suggestion.clone();
                            egui::Frame::default()
                                .fill(fill)
                                .corner_radius(egui::CornerRadius::same(4))
                                .inner_margin(egui::Margin::symmetric(8, 3))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(prefix)
                                                .color(theme.text_dim)
                                                .small(),
                                        );
                                        ui.label(
                                            RichText::new(suggestion.as_str())
                                                .color(text_color)
                                                .monospace()
                                                .size(14.0),
                                        );
                                    });
                                })
                                .response
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                                .then(|| self.accept_ac(&s));
                        }
                    });
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
        if ctx.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command) {
            self.compile_source();
        }

        // Autocomplete: detect source changes and update suggestions
        if self.source != self.ac_prev_source {
            self.update_ac();
        }

        if !self.ac_suggestions.is_empty() {
            if ctx.input(|i| i.key_pressed(Key::Tab)) {
                let idx = self.ac_index;
                if idx < self.ac_suggestions.len() {
                    let s = self.ac_suggestions[idx].clone();
                    self.accept_ac(&s);
                }
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Tab));
            }
            if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                self.ac_index = (self.ac_index + 1) % self.ac_suggestions.len();
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowDown));
            }
            if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
                self.ac_index = if self.ac_index == 0 {
                    self.ac_suggestions.len() - 1
                } else {
                    self.ac_index - 1
                };
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowUp));
            }
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.ac_suggestions.clear();
                self.ac_word.clear();
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape));
            }
        }

        self.show_toolbar(ctx);
        self.show_pipeline_strip(ctx);
        self.show_status_bar(ctx);

        self.show_terminal_panel(ctx);

        if !self.focus_mode {
            self.show_ast_panel(ctx);
            self.show_diagnostics_bottom(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_editor(ui);
        });

        // Popup autocomplete sobre el editor
        self.show_ac_popup(ctx);

        self.show_help_windows(ctx);
    }
}