# 🎨 GUI — Hulk Compiler Studio

Entorno gráfico del compilador (`eframe/egui`), organizado en módulos con
responsabilidad única sobre la librería `hulk_compiler`.

![Arquitectura de la GUI](docs/gui-architecture.svg)

## Flujo

1. El usuario edita en el **editor** (resaltado en vivo con el lexer real del
   compilador + gutter de líneas que marca errores en rojo).
2. `▶ Compilar` (o `Ctrl/Cmd+Enter`) llama a `Compiler::compile` — el mismo
   pipeline que la CLI — y cronometra.
3. El [`CompileReport`](../../compiler/mod.rs) alimenta todos los paneles a la
   vez: la franja de **pipeline** se colorea según la fase alcanzada, el
   **AST** se renderiza como árbol con búsqueda, y las pestañas muestran
   errores (tarjetas con badge), tokens (tabla coloreada por rol) e IR.
4. Si la compilación fue exitosa, se ejecuta el programa (`lli` en
   macOS/Linux, `clang → exe` en Windows) y la salida va a la terminal
   integrada.

## Módulos

| Módulo | Rol |
|--------|-----|
| `main.rs` | Punto de entrada de eframe |
| `app.rs` | Estado (`HulkGui`) + layout de paneles + loop de UI |
| `theme.rs` | 4 paletas en vivo: 💚 Hulk Smash · 🌙 Catppuccin · ⚡ Cyberpunk · 🌌 VS Dark |
| `highlight.rs` | Resaltado sintáctico usando el **lexer real** (lo que se colorea es lo que se tokeniza) |
| `ast_view.rs` | Árbol interactivo del AST con búsqueda y resaltado de coincidencias |
| `runner.rs` | Ejecución del IR, snippets de demo, ejemplos y extensión VSCode |

## Detalles con flow

- **Snippets ⚡**: demos de un click por feature (lambdas, macros, arrays,
  protocolos, generadores), verificadas end-to-end contra el compilador.
- El tema se aplica en caliente reconstruyendo los `Visuals` de egui
  (`Theme::apply`); el resaltado y el AST usan los mismos colores del tema.
- La franja de pipeline deduce la fase que falló de la **primera categoría de
  error** del reporte — misma regla que el exit code de la CLI.
