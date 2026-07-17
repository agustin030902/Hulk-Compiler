//! Sistema de temas de la GUI.
//!
//! Cada tema define la paleta completa (fondos, texto, acentos y colores de
//! resaltado sintáctico). Se puede cambiar en caliente desde la barra de
//! herramientas; `apply` reconstruye los `Visuals` de egui a partir del tema.

use eframe::egui::{self, Color32, FontId, TextStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    HulkSmash,
    CatppuccinMocha,
    Cyberpunk,
    VsDark,
    Dracula,
    Nord,
    TokyoNight,
}

impl ThemeKind {
    pub const ALL: [ThemeKind; 7] = [
        ThemeKind::HulkSmash,
        ThemeKind::CatppuccinMocha,
        ThemeKind::Cyberpunk,
        ThemeKind::VsDark,
        ThemeKind::Dracula,
        ThemeKind::Nord,
        ThemeKind::TokyoNight,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeKind::HulkSmash => "💚 Hulk Smash",
            ThemeKind::CatppuccinMocha => "🌙 Catppuccin",
            ThemeKind::Cyberpunk => "⚡ Cyberpunk",
            ThemeKind::VsDark => "🌌 VS Dark",
            ThemeKind::Dracula => "🧛 Dracula",
            ThemeKind::Nord => "❄️ Nord",
            ThemeKind::TokyoNight => "🌃 Tokyo Night",
        }
    }

    pub fn palette(self) -> Theme {
        match self {
            ThemeKind::HulkSmash => Theme::hulk_smash(),
            ThemeKind::CatppuccinMocha => Theme::catppuccin_mocha(),
            ThemeKind::Cyberpunk => Theme::cyberpunk(),
            ThemeKind::VsDark => Theme::vs_dark(),
            ThemeKind::Dracula => Theme::dracula(),
            ThemeKind::Nord => Theme::nord(),
            ThemeKind::TokyoNight => Theme::tokyo_night(),
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
    // Consola / terminal integrada
    pub terminal_bg: Color32,
    pub terminal_text: Color32,
    pub prompt: Color32,
}

impl Theme {
    /// Tema insignia: verde radiactivo sobre negro verdoso, con púrpura
    /// (los pantalones de Hulk) para las palabras clave.
    pub fn hulk_smash() -> Self {
        Self {
            bg_main: Color32::from_rgb(13, 17, 12),
            bg_panel: Color32::from_rgb(10, 13, 9),
            bg_input: Color32::from_rgb(7, 10, 7),
            text: Color32::from_rgb(214, 235, 208),
            text_dim: Color32::from_rgb(120, 145, 115),
            accent: Color32::from_rgb(120, 220, 60),
            success: Color32::from_rgb(140, 240, 80),
            error: Color32::from_rgb(255, 92, 92),
            keyword: Color32::from_rgb(190, 130, 255),
            function: Color32::from_rgb(255, 214, 90),
            variable: Color32::from_rgb(200, 224, 195),
            number: Color32::from_rgb(120, 220, 160),
            string: Color32::from_rgb(235, 200, 120),
            boolean: Color32::from_rgb(110, 200, 255),
            operator: Color32::from_rgb(160, 200, 150),
            unknown: Color32::from_rgb(255, 92, 92),
            terminal_bg: Color32::from_rgb(6, 9, 6),
            terminal_text: Color32::from_rgb(198, 240, 190),
            prompt: Color32::from_rgb(120, 220, 60),
        }
    }

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
            terminal_bg: Color32::from_rgb(17, 17, 27), // crust
            terminal_text: Color32::from_rgb(205, 214, 244), // text
            prompt: Color32::from_rgb(166, 227, 161), // green
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
            terminal_bg: Color32::from_rgb(6, 6, 12),
            terminal_text: Color32::from_rgb(220, 220, 235),
            prompt: Color32::from_rgb(0, 255, 255),
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
            terminal_bg: Color32::from_rgb(24, 24, 24),
            terminal_text: Color32::from_rgb(212, 212, 212),
            prompt: Color32::from_rgb(120, 210, 150),
        }
    }

    pub fn dracula() -> Self {
        Self {
            bg_main: Color32::from_rgb(40, 42, 54),
            bg_panel: Color32::from_rgb(33, 34, 44),
            bg_input: Color32::from_rgb(25, 26, 33),
            text: Color32::from_rgb(248, 248, 242),
            text_dim: Color32::from_rgb(98, 114, 164),
            accent: Color32::from_rgb(189, 147, 249),
            success: Color32::from_rgb(80, 250, 123),
            error: Color32::from_rgb(255, 85, 85),
            keyword: Color32::from_rgb(255, 121, 198),
            function: Color32::from_rgb(80, 250, 123),
            variable: Color32::from_rgb(248, 248, 242),
            number: Color32::from_rgb(189, 147, 249),
            string: Color32::from_rgb(241, 250, 140),
            boolean: Color32::from_rgb(139, 233, 253),
            operator: Color32::from_rgb(255, 121, 198),
            unknown: Color32::from_rgb(255, 85, 85),
            terminal_bg: Color32::from_rgb(25, 26, 33),
            terminal_text: Color32::from_rgb(248, 248, 242),
            prompt: Color32::from_rgb(80, 250, 123),
        }
    }

    pub fn nord() -> Self {
        Self {
            bg_main: Color32::from_rgb(46, 52, 64),
            bg_panel: Color32::from_rgb(59, 66, 82),
            bg_input: Color32::from_rgb(67, 76, 94),
            text: Color32::from_rgb(236, 239, 244),
            text_dim: Color32::from_rgb(129, 161, 193),
            accent: Color32::from_rgb(136, 192, 208),
            success: Color32::from_rgb(163, 190, 140),
            error: Color32::from_rgb(191, 97, 106),
            keyword: Color32::from_rgb(180, 142, 173),
            function: Color32::from_rgb(136, 192, 208),
            variable: Color32::from_rgb(216, 222, 233),
            number: Color32::from_rgb(180, 142, 173),
            string: Color32::from_rgb(163, 190, 140),
            boolean: Color32::from_rgb(129, 161, 193),
            operator: Color32::from_rgb(129, 161, 193),
            unknown: Color32::from_rgb(191, 97, 106),
            terminal_bg: Color32::from_rgb(46, 52, 64),
            terminal_text: Color32::from_rgb(236, 239, 244),
            prompt: Color32::from_rgb(163, 190, 140),
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            bg_main: Color32::from_rgb(26, 27, 38),
            bg_panel: Color32::from_rgb(22, 22, 30),
            bg_input: Color32::from_rgb(31, 35, 53),
            text: Color32::from_rgb(169, 177, 214),
            text_dim: Color32::from_rgb(86, 95, 137),
            accent: Color32::from_rgb(122, 162, 247),
            success: Color32::from_rgb(158, 206, 106),
            error: Color32::from_rgb(247, 118, 142),
            keyword: Color32::from_rgb(187, 154, 247),
            function: Color32::from_rgb(122, 162, 247),
            variable: Color32::from_rgb(169, 177, 214),
            number: Color32::from_rgb(255, 158, 100),
            string: Color32::from_rgb(158, 206, 106),
            boolean: Color32::from_rgb(125, 207, 255),
            operator: Color32::from_rgb(137, 221, 255),
            unknown: Color32::from_rgb(247, 118, 142),
            terminal_bg: Color32::from_rgb(26, 27, 38),
            terminal_text: Color32::from_rgb(169, 177, 214),
            prompt: Color32::from_rgb(122, 162, 247),
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
