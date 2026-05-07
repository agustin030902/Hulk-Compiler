# Hulk Compiler

Compilador en Rust con pipeline por fases y salida a LLVM IR.

```text
lexer -> parser (LR1) -> semantic -> codegen LLVM
```

Si una fase falla, se detiene el pipeline y se genera diagnóstico.

## Estado actual (resumen)

- Lenguaje basado en expresiones.
- Scope léxico en bloques `{ ... }` y `let ... in ...`.
- `while` como expresión (`Unit`).
- Funciones globales con recursión.
- Semántica con inferencia de tipos para parámetros y retorno.
- Codegen LLVM con firmas tipadas reales por función (ya no forzado a `double`).

## Estructura del proyecto

```text
src/
  lexer/
  parser/
  semantic/
  codegen/
    llvm/
  compiler/
  error/
  runner/
  bin/
  main.rs
examples/
```

## Características del lenguaje

### Tipos

- `Number`
- `Boolean`
- `String`
- `Unit`

### Expresiones soportadas

- Literales numéricos, booleanos y string
- Variables
- Operadores unarios: `-`, `!`
- Operadores binarios:
  - aritméticos: `+ - * / ^`
  - concatenación: `@`
  - comparación: `< > <= >= == !=`
  - lógicos: `&& ||`
- `if / elif / else`
- `while (cond) { ... }`
- bloques `{ ... }`
- `let ... in ...`
- asignación destructiva `:=`
- llamadas a funciones de usuario y builtins

### Builtins

- `print(x)`
- `sin(x)`, `cos(x)`, `sqrt(x)`, `exp(x)`
- `log(base, value)`
- `rand()`
- constantes: `PI`, `E`

## Funciones de usuario

Formas soportadas:

```hulk
function f(x, y) => x + y;

function g(x) {
  print(x);
  x + 1
}
```

Reglas principales:

- Se declaran globalmente.
- Se permite recursión.
- Se valida aridad de llamada.
- Se valida tipo de cada argumento.
- Se infiere tipo de retorno desde el cuerpo y su contexto de uso.

Ejemplo de inferencia contextual:

```hulk
function id(x) => x;
function plus_one(y) => id(y) + 1;
print(plus_one(41));
```

## Semántica

La fase semántica:

- valida scopes de variables
- valida reglas de operadores y builtins
- mantiene firmas de funciones (`param_types`, `return_type`)
- ejecuta inferencia por pasadas (fixed-point)
- reporta error si queda algún tipo `Unknown`

Detalles: [`src/semantic/README.md`](src/semantic/README.md)

## Codegen LLVM

La fase de codegen:

- carga firmas inferidas por semántica
- emite funciones LLVM tipadas por firma
- genera `main` con las sentencias globales
- emite llamadas tipadas (`call`) para funciones de usuario

Detalles: [`src/codegen/README.md`](src/codegen/README.md)

## Ejecutar

### Compilar un archivo

```bash
cargo run -- --input examples/calculator_ok.hulk --emit-ir artifacts/output.ll
```

### Compilar todos los ejemplos de una carpeta

```bash
cargo run -- --run-all examples --emit-dir artifacts/batch
```

### Generar/ejecutar binario nativo

```bash
cargo run -- run examples/calculator_ok.hulk
```

## GUI

Hay GUI de prueba en `src/bin/gui.rs`:

```bash
cargo run --bin gui
```

Más info: [`src/bin/README.md`](src/bin/README.md)

## Tests

Ejecutar suite completa:

```bash
cargo test -q
```

Por fase:

```bash
cargo test -q lexer::
cargo test -q parser::
cargo test -q semantic::
cargo test -q codegen::
cargo test -q compiler::
```

## Ejemplos recomendados

### Funciones recursivas

- `examples/function_inline_recursion.hulk`
- `examples/recursive_number_fibonacci.hulk`
- `examples/recursive_string_countdown.hulk`

### Showcase (mochila)

- `examples/mochila_funcionalidades.hulk`

Este ejemplo combina recursión, `while`, `let-in`, bloques, `:=`, builtins y `print`.

### Otros válidos

- `examples/function_block_body.hulk`
- `examples/function_calls_composition.hulk`
- `examples/builtin_math_ok.hulk`
- `examples/block_scope_ok.hulk`
- `examples/while_ok.hulk`

### Con error (diagnósticos)

- `examples/builtin_math_type_error.hulk`
- `examples/power_type_error.hulk`
- `examples/destructive_assign_type_error.hulk`
- `examples/error_scope_leak.hulk`
- `examples/while_condition_type_error.hulk`

## Notas

- Extensión recomendada: `.hulk`.
- La CLI aún acepta `.hk` por compatibilidad.
- `print` devuelve `Unit` (no imprimible como argumento de otro `print`).
- `:=` exige conservar tipo.
