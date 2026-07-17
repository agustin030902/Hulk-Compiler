# 🚂 Compiler — Orquestación del Pipeline

[`Compiler`](mod.rs) encadena las fases con política **fail-fast**: en cuanto
una produce errores, se interrumpe el flujo y se escribe un reporte de
diagnóstico en `output_path` en lugar del IR.

![Pipeline completo](docs/compiler-pipeline.svg)

## Flujo de `compile(source, options)`

| # | Fase | Falla ⇒ | Exit code |
|---|------|---------|-----------|
| 1 | [`Lexer`](../lexer/README.md) | `LEXICAL` | `1` |
| 2 | [`Parser`](../parser/README.md) | `SYNTACTIC` | `2` |
| 3 | [`MacroExpander`](../parser/macro_expander.rs) | `SEMANTIC` | `3` |
| 4 | [`SemanticAnalyzer`](../semantic/README.md) | `SEMANTIC` | `3` |
| 5 | [`LlvmBackend`](../codegen/README.md) — recibe el análisis del paso 4 | `SEMANTIC` | `3` |

El resultado completo queda en [`CompileReport`](mod.rs): tokens, AST, IR,
ruta de salida y errores — los consumidores (CLI, GUI) muestran cada
artefacto por separado aunque el pipeline se haya interrumpido.

## Consumidores

- **CLI** ([`src/main.rs`](../main.rs)): `./hulk archivo.hulk` escribe
  `temp.ll`, invoca `clang -Wno-override-module temp.ll -lm -o output` y
  mapea la primera categoría de error al exit code del contrato.
- **GUI** ([`src/bin/gui/`](../bin/gui/README.md)): usa el mismo `Compiler`
  y muestra tokens/AST/IR/errores en paneles.

## Nota de diseño

El `Compiler` pasa su `SemanticAnalyzer` ya corrido a
`generate(&program, &analyzer)`: **el programa se analiza exactamente una
vez** y codegen consume las tablas resultantes, nunca las re-deriva.
