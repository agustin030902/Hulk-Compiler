# Binaries (`src/bin`)

Este directorio contiene binarios auxiliares del proyecto.

## 1) GUI (`src/bin/gui.rs`)

Interfaz de prueba rápida con `eframe/egui`.

### Qué permite
- Cargar ejemplos `.hulk` desde `examples/`
- Editar código fuente
- Compilar (lexer -> parser -> semantic -> LLVM IR)
- Ver errores, tokens, AST e IR
- Ejecutar el IR con `lli` y ver salida

### Ejecutar

```bash
cargo run --bin gui
```

## 2) CLI principal (`src/main.rs`)

Aunque vive fuera de `src/bin`, es el binario principal del compilador.

Comandos típicos:

```bash
cargo run -- --input examples/calculator_ok.hulk --emit-ir artifacts/output.txt
cargo run -- --run-all examples --emit-dir artifacts/batch
cargo run -- run examples/calculator_ok.hulk
```

## Notas
- La GUI usa `lli` para interpretar IR.
- Para binario nativo (`run`), se usa `clang` (ver `README.md` raíz).
- Las funciones de usuario ya se emiten con firmas tipadas inferidas (no solo numéricas).
