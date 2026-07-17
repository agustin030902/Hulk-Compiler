# 🖥️ Binaries (`src/bin`)

El proyecto compila **dos binarios** sobre la misma librería
[`hulk_compiler`](../lib.rs) — así el compilador se compila una sola vez y
ambos consumen exactamente el mismo pipeline.

## 1) GUI (`src/bin/gui/`)

**Hulk Compiler Studio**: entorno gráfico con `eframe/egui`, organizado en
módulos ([ver su README](gui/README.md)):

```bash
cargo run --bin gui
```

- Editor con resaltado en vivo (usa el lexer real), gutter de líneas con
  marcas de error y autocompletado.
- Pipeline visual (Lexer → Parser → Semántica → Codegen → Run) coloreado
  según el resultado, con cronómetro de compilación.
- Paneles de AST (árbol con búsqueda), tokens, IR, errores y terminal.
- 4 temas en vivo (💚 Hulk Smash por defecto) y snippets de demo por feature.

## 2) CLI principal (`src/main.rs`)

Es el binario del contrato del corrector (vive fuera de `src/bin` por ser el
binario por defecto del paquete):

```bash
make build          # → ./hulk
./hulk programa.hulk   # escribe temp.ll, invoca clang, produce ./output
./output               # ejecuta el programa compilado
```

| Exit code | Significado |
|-----------|-------------|
| `0` | OK — ejecutable en `./output` |
| `1` | error léxico (`LEXICAL` en stderr) |
| `2` | error sintáctico (`SYNTACTIC`) |
| `3` | error semántico (`SEMANTIC`) |

## Nota

Ninguno de los dos binarios se documenta con rustdoc (`doc = false` en
`Cargo.toml`): en filesystems case-insensitive las docs del binario
`Hulk_Compiler` pisarían las de la librería `hulk_compiler`.
