//! Sistema de temas de la GUI.
//!
//! Cada tema define la paleta completa (fondos, texto, acentos y colores de
//! resaltado sintáctico). Se puede cambiar en caliente desde la barra de
//! herramientas; `apply` reconstruye los `Visuals` de egui a partir del tema.

use eframe::egui::{self, Color32, FontId, TextStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    CatppuccinMocha,
    Cyberpunk,
    VsDark,
}

impl ThemeKind {
    pub const ALL: [ThemeKind; 3] = [
        ThemeKind::CatppuccinMocha,
        ThemeKind::Cyberpunk,
        ThemeKind::VsDark,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeKind::CatppuccinMocha => "🌙 Catppuccin",
            ThemeKind::Cyberpunk => "⚡ Cyberpunk",
            ThemeKind::VsDark => "🌌 VS Dark",
        }
    }

    pub fn palette(self) -> Theme {
        match self {
            ThemeKind::CatppuccinMocha => Theme::catppuccin_mocha(),
            ThemeKind::Cyberpunk => Theme::cyberpunk(),
            ThemeKind::VsDark => Theme::vs_dark(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg_main: Color32,
    pub bg_panel: Color32,
    pub bg_input: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub success: Color32,
    pub error: Color32,
    // Resaltado sintáctico
    pub keyword: Color32,
    pub function: Color32,
    pub variable: Color32,
    pub number: Color32,
    pub string: Color32,
    pub boolean: Color32,
    pub operator: Color32,
    pub unknown: Color32,
}

impl Theme {
    pub fn catppuccin_mocha() -> Self {
        Self {
            bg_main: Color32::from_rgb(30, 30, 46),   // base
            bg_panel: Color32::from_rgb(24, 24, 37),  // mantle
            bg_input: Color32::from_rgb(17, 17, 27),  // crust
            text: Color32::from_rgb(205, 214, 244),   // text
            text_dim: Color32::from_rgb(147, 153, 178), // overlay1
            accent: Color32::from_rgb(137, 180, 250), // blue
            success: Color32::from_rgb(166, 227, 161), // green
            error: Color32::from_rgb(243, 139, 168),  // red
            keyword: Color32::from_rgb(203, 166, 247), // mauve
            function: Color32::from_rgb(250, 179, 135), // peach
            variable: Color32::from_rgb(186, 194, 222), // subtext1
            number: Color32::from_rgb(166, 227, 161), // green
            string: Color32::from_rgb(249, 226, 175), // yellow
            boolean: Color32::from_rgb(137, 180, 250), // blue
            operator: Color32::from_rgb(180, 190, 254), // lavender
            unknown: Color32::from_rgb(243, 139, 168), // red
        }
    }

    pub fn cyberpunk() -> Self {
        Self {
            bg_main: Color32::from_rgb(10, 10, 18),
            bg_panel: Color32::from_rgb(16, 16, 28),
            bg_input: Color32::from_rgb(22, 22, 38),
            text: Color32::from_rgb(220, 220, 235),
            text_dim: Color32::from_rgb(140, 140, 170),
            accent: Color32::from_rgb(0, 255, 255),
            success: Color32::from_rgb(0, 255, 140),
            error: Color32::from_rgb(255, 60, 60),
            keyword: Color32::from_rgb(255, 0, 200),
            function: Color32::from_rgb(255, 255, 80),
            variable: Color32::from_rgb(200, 200, 220),
            number: Color32::from_rgb(0, 255, 140),
            string: Color32::from_rgb(255, 140, 0),
            boolean: Color32::from_rgb(0, 200, 255),
            operator: Color32::from_rgb(180, 180, 200),
            unknown: Color32::from_rgb(255, 60, 60),
        }
    }

    pub fn vs_dark() -> Self {
        Self {
            bg_main: Color32::from_rgb(30, 30, 30),
            bg_panel: Color32::from_rgb(37, 37, 38),
            bg_input: Color32::from_rgb(45, 45, 45),
            text: Color32::from_rgb(212, 212, 212),
            text_dim: Color32::from_rgb(140, 140, 140),
            accent: Color32::from_rgb(86, 156, 214),
            success: Color32::from_rgb(120, 210, 150),
            error: Color32::from_rgb(244, 71, 71),
            keyword: Color32::from_rgb(197, 134, 192),
            function: Color32::from_rgb(220, 220, 170),
            variable: Color32::from_rgb(212, 212, 212),
            number: Color32::from_rgb(181, 206, 168),
            string: Color32::from_rgb(206, 145, 120),
            boolean: Color32::from_rgb(86, 156, 214),
            operator: Color32::from_rgb(212, 212, 212),
            unknown: Color32::from_rgb(244, 71, 71),
        }
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(16.0, egui::FontFamily::Monospace),
        );
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);

        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(self.text);
        visuals.panel_fill = self.bg_main;
        visuals.window_fill = self.bg_panel;
        visuals.faint_bg_color = self.bg_panel;
        visuals.extreme_bg_color = self.bg_input;
        visuals.code_bg_color = self.bg_input;
        visuals.selection.bg_fill = self.accent.gamma_multiply(0.45);
        visuals.hyperlink_color = self.accent;

        let rounding = egui::CornerRadius::same(6);
        visuals.widgets.noninteractive.bg_fill = self.bg_panel;
        visuals.widgets.noninteractive.bg_stroke.color = self.bg_input.gamma_multiply(2.2);
        visuals.widgets.noninteractive.corner_radius = rounding;
        visuals.widgets.inactive.bg_fill = self.bg_input;
        visuals.widgets.inactive.bg_stroke.color = self.text_dim.gamma_multiply(0.35);
        visuals.widgets.inactive.corner_radius = rounding;
        visuals.widgets.hovered.bg_fill = self.bg_input.gamma_multiply(1.6);
        visuals.widgets.hovered.bg_stroke.color = self.accent;
        visuals.widgets.hovered.corner_radius = rounding;
        visuals.widgets.active.bg_fill = self.accent.gamma_multiply(0.35);
        visuals.widgets.active.bg_stroke.color = self.accent;
        visuals.widgets.active.corner_radius = rounding;
        visuals.widgets.open.bg_fill = self.bg_input.gamma_multiply(1.4);
        visuals.widgets.open.corner_radius = rounding;
        visuals.window_corner_radius = egui::CornerRadius::same(10);

        style.visuals = visuals;
        ctx.set_style(style);
    }
}
