# Hulk Compiler

Compilador en Rust con arquitectura por fases y salida en `.txt`.

Flujo actual:

```text
lexer -> parser (LR1) -> checkSemantic -> LLVM IR
```

## Resumen directo

- Si todo está bien: el `.txt` contiene LLVM IR.
- Si hay errores: el `.txt` contiene diagnóstico (tipo, fase, línea, columna, explicación).
- Se trabaja por fases con comportamiento `fail-fast`:
  - Si falla `lexer`, no corre `parser`.
  - Si falla `parser`, no corre `semantic`.
  - Si falla `semantic`, no corre `LLVM`.

## Arquitectura

```text
src/
  lexer/                  # análisis léxico con logos
  parser/                 # parser LR(1) con lalrpop + AST
  semantic/               # chequeo de tipos y reglas semánticas
  codegen/
    llvm/                 # backend LLVM IR
    mod.rs                # trait para backends futuros
  compiler/               # orquestación del pipeline
  error/                  # modelo unificado de errores
  main.rs                 # CLI
```

## Lenguaje soportado ahora (modo calculadora)

- Sentencias:
  - `let x = <expr>;`
  - `print(<expr>);`
- Expresiones:
  - `int`, `float`, `string`, `boolean`
  - variables
  - `+ - * /`
  - paréntesis

Regla semántica importante:
- operaciones aritméticas solo con números.

## Comandos

### 1) Compilar un solo archivo

```bash
cargo run -- --input examples/calculator_ok.hk --emit-ir artifacts/output.txt
```

Si no pasas `--input`, usa el source por defecto del `main.rs`.

### 2) Comando que pediste: ejecutar todos los `.hk` de `examples`

```bash
cargo run -- --run-all examples --emit-dir artifacts/batch
```

También funciona así:

```bash
cargo run -- --run-all examples --emit-ir artifacts/batch
```

Resultado:
- genera un `.txt` por cada `.hk`.
- ejemplo: `examples/demo.hk` -> `artifacts/batch/demo.txt`.

### 3) Ayuda de CLI

```bash
cargo run -- --help
```

## Formato del `.txt` de errores

Formato:

```text
[TipoError] [Fase] line X, column Y: explicación
```

Ejemplo real:

```text
Hulk Compiler Diagnostics
========================
1. [Type] [Semantic] line 1, column 9: Operator '+' expects Number and Number, but got String and Number.
2. [Type] [Semantic] line 2, column 9: Operator '+' expects Number and Number, but got Boolean and Number.
```

Tipos de error:
- `Lexical`
- `Syntax`
- `Type`
- `Semantic`

## Ejecutar LLVM IR

Cuando el `.txt` sea IR válido:

```bash
lli artifacts/output.txt
```

Alternativa nativa:

```bash
clang -x ir artifacts/output.txt -o artifacts/program
./artifacts/program
```

## Extensibilidad

Para agregar otro backend (ej. WASM/bytecode):
- implementa `CodegenBackend` en `src/codegen/mod.rs`.
- conéctalo en `src/compiler/mod.rs`.

Así mantienes lexer/parser/semantic encapsulados y reutilizables.
