//! GUI del compilador HULK (binario `gui`).
//!
//! Módulos:
//! - `app`: estado de la aplicación, layout de paneles y loop de eframe.
//! - `theme`: paletas seleccionables (Catppuccin, Cyberpunk, VS Dark).
//! - `highlight`: resaltado sintáctico usando el lexer real del compilador.
//! - `ast_view`: árbol interactivo del AST con búsqueda.
//! - `runner`: ejecución del IR (lli/clang), ejemplos y extensión VSCode.

mod app;
mod ast_view;
mod highlight;
mod runner;
mod theme;

use app::HulkGui;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1500.0, 900.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Hulk Compiler Studio",
        options,
        Box::new(|cc| Ok(Box::new(HulkGui::new(cc)))),
    )
}
