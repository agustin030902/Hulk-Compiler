//! # Hulk Compiler — núcleo del compilador de HULK
//!
//! <div style="display:flex;gap:6px;flex-wrap:wrap;margin:10px 0 18px 0;">
//!   <span style="background:#78dc3c;color:#0d110c;font-weight:bold;padding:3px 10px;border-radius:10px;font-size:0.85em;">Rust 2024</span>
//!   <span style="background:rgba(120,220,60,.15);color:#78dc3c;border:1px solid #78dc3c;padding:3px 10px;border-radius:10px;font-size:0.85em;">LLVM IR</span>
//!   <span style="background:rgba(120,220,60,.15);color:#78dc3c;border:1px solid #78dc3c;padding:3px 10px;border-radius:10px;font-size:0.85em;">LALRPOP LR(1)</span>
//!   <span style="background:rgba(120,220,60,.15);color:#78dc3c;border:1px solid #78dc3c;padding:3px 10px;border-radius:10px;font-size:0.85em;">logos</span>
//!   <span style="background:rgba(120,220,60,.15);color:#78dc3c;border:1px solid #78dc3c;padding:3px 10px;border-radius:10px;font-size:0.85em;">suite 115/115 ✔</span>
//! </div>
//!
//! Compilador del lenguaje **HULK** (*Havana University Language for Kompilers*)
//! escrito íntegramente en Rust. Compila programas HULK a **LLVM IR** en formato
//! texto, que luego se convierte en un ejecutable nativo mediante `clang`.
//!
//! Este crate expone el compilador como librería, compartida por dos binarios:
//!
//! - **`Hulk-Compiler`** (CLI): `./hulk programa.hulk` compila y produce `./output`.
//! - **`gui`**: entorno gráfico interactivo (`egui/eframe`) con editor
//!   resaltado, vista del AST, pipeline visual y ejecución integrada.
//!
//! ## El lenguaje HULK
//!
//! HULK es un lenguaje **basado en expresiones**: bloques, `let-in` e `if/else`
//! devuelven el valor de su última expresión. Es orientado a objetos con
//! herencia simple, interfaces estructurales (protocols) e inferencia de tipos.
//!
//! ```text
//! type Circle(r: Number) {
//!     radius: Number = r;
//!     area(): Number { PI * self.radius ^ 2; }
//! }
//!
//! define square(x: Number): Number -> x * x;          // macro (call-by-name)
//!
//! let double: (Number) -> Number =                     // lambda con closure
//!     function (x: Number): Number -> x * 2 in
//! let a: Number[] = new Number[5]{ i -> i * i } in {   // arreglo con init
//!     print(double(a[3]) + square(2));                 // 22
//!     print(new Circle(1).area());                     // 3.14159...
//! };
//! ```
//!
//! ### Características soportadas
//!
//! | Categoría | Features |
//! |-----------|----------|
//! | Básicos | expresiones aritméticas/booleanas, strings con `@`/`@@`, `let-in`, `if/elif/else`, `while`, `for`, funciones, recursión, comentarios `//` |
//! | OOP | `type` con herencia (`inherits`), atributos privados, métodos con `self`, `base()`, dispatch dinámico, `is` / `as` |
//! | Interfaces | `protocol` estructurales con herencia (`extends`) y varianza (covariante en retornos, contravariante en parámetros) |
//! | Iteración | protocolos duales `Iterable`/`Enumerable`, `range(a, b)`, splat notation `T*` |
//! | Arreglos | literales `{1, 2, 3}`, `new T[n]`, `new T[n]{ i -> expr }`, matrices `T[][]`, indexación como *lvalue*, `size()` |
//! | Lambdas | closures reales con captura por valor, tipos función `(Number) -> Number`, orden superior y composición |
//! | Macros | `define` con expansión call-by-name higiénica a nivel de AST |
//! | Inferencia | fixpoint iterativo de firmas entre funciones mutuamente recursivas |
//!
//! ## Arquitectura: pipeline de cuatro fases
//!
//! ```text
//!  fuente ──▶ [lexer] ──▶ [parser] ──▶ [expansión de macros]
//!    .hulk    tokens       AST            AST plano
//!                                            │
//!                ┌───────────────────────────┘
//!                ▼
//!          [semantic] ──▶ [codegen] ──▶ LLVM IR ──▶ clang ──▶ ejecutable
//!           tipos OK       texto .ll
//! ```
//!
//! El modelo es **fail-fast**: cada fase consume la salida de la anterior y, en
//! cuanto una produce errores, el pipeline se detiene y devuelve diagnósticos.
//!
//! - [`lexer`] — tokenización con `logos`; los errores no abortan (emite
//!   [`lexer::TokenKind::Unknown`] como token de continuidad).
//! - [`parser`] — gramática LR(1) con LALRPOP; define el [AST](parser::expression)
//!   y el [expansor de macros](parser::macro_expander).
//! - [`semantic`] — dos pasadas: inferencia de firmas (fixpoint) + recolección
//!   de símbolos y verificación de tipos ([`semantic::SemanticAnalyzer`]).
//! - [`codegen`] — backend LLVM que emite IR como texto
//!   ([`codegen::llvm::LlvmBackend`]): sin dependencia de versión de LLVM y
//!   fácil de inspeccionar.
//! - [`compiler`] — orquestación del pipeline ([`compiler::Compiler`]).
//! - [`error`] — diagnóstico unificado ([`error::CompilerError`]).
//!
//! ## Uso como librería
//!
//! ```no_run
//! use hulk_compiler::compiler::{CompileOptions, Compiler};
//!
//! let mut compiler = Compiler::new();
//! let report = compiler.compile("print(40 + 2);", &CompileOptions::default());
//!
//! if report.errors.is_empty() {
//!     // IR LLVM listo para `clang archivo.ll -lm -o programa`
//!     println!("{}", report.llvm_ir.expect("IR generado"));
//! } else {
//!     for error in &report.errors {
//!         eprintln!("({},{}) {}", error.line, error.column, error.message);
//!     }
//! }
//! ```
//!
//! ## Contrato de la CLI
//!
//! `./hulk <archivo>` compila y enlaza con `clang`, dejando el ejecutable en
//! `./output`. El código de salida refleja la primera categoría de error:
//!
//! | Exit code | Significado |
//! |-----------|-------------|
//! | `0` | compilación exitosa |
//! | `1` | error léxico (`LEXICAL`) |
//! | `2` | error sintáctico (`SYNTACTIC`) |
//! | `3` | error semántico o de tipos (`SEMANTIC`) |
//!
//! ## Decisiones de diseño destacadas
//!
//! - **IR como texto** en lugar de `inkwell`: evita el acople a una versión de
//!   LLVM y hace el IR trivialmente inspeccionable (la validación la hace `clang`).
//! - **Números unificados a `double`** (f64), al estilo de JavaScript/Lua.
//! - **Dispatch dinámico por cascada de type-tag** con búsqueda en la jerarquía
//!   completa (`type_parents`), incluida la herencia de implementaciones.
//! - **Objetos** con layout `[type_id i64][campos del padre][campos propios]` y
//!   subtipado en runtime vía la tabla global `@hulk_type_parents`.
//! - **Closures** como bloques de heap `[fnptr][capturas…]` con captura por valor.
//!
//! La justificación completa de cada decisión está en el `REPORT.md` del
//! repositorio (~1000 líneas de reporte técnico).

#![doc(html_logo_url = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 64 64'%3E%3Crect width='64' height='64' rx='14' fill='%2378dc3c'/%3E%3Ctext x='32' y='45' font-size='38' text-anchor='middle' font-family='Menlo,monospace' font-weight='bold' fill='%230d110c'%3EH%3C/text%3E%3C/svg%3E")]
#![doc(html_favicon_url = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 64 64'%3E%3Crect width='64' height='64' rx='14' fill='%2378dc3c'/%3E%3Ctext x='32' y='45' font-size='38' text-anchor='middle' font-family='Menlo,monospace' font-weight='bold' fill='%230d110c'%3EH%3C/text%3E%3C/svg%3E")]

pub mod codegen;
pub mod compiler;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod semantic;
