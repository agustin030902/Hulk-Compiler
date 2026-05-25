# Hulk Compiler

Compilador en Rust con pipeline por fases y salida a LLVM IR.

```text
lexer -> parser (LR1) -> semantic -> codegen LLVM
```

Si una fase falla, se detiene el pipeline y se genera diagnóstico.

## Estado actual (resumen)

- Lenguaje basado en expresiones.
- Scope léxico en bloques `{ ... }` y `let ... in ...`.
- Anotaciones explícitas de tipo en `let` y bindings de `let-in`.
- Anotaciones de tipos en parámetros y retorno de funciones.
- Tipos nominales de usuario con `type`, herencia `inherits`, instanciación `new` y métodos con `self`.
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
- `let` con anotación de tipo: `let x: Number = 42;`
- asignación destructiva `:=`
- llamadas a funciones de usuario y builtins
- acceso a miembros por `.` (métodos y atributos privados dentro del tipo)
- instanciación de tipos con `new TypeName(...)`

### Anotación de tipos en variables

Sintaxis soportada:

```hulk
let x: Number = 42;
let y: String = "hola";
let ok: Boolean = true;

let msg = let base: Number = 10, sufijo: String = " pts" in "score=" @ base @ sufijo;
```

Reglas:

- Tipos válidos en anotación: `Number`, `Boolean`, `String`, `Unit`.
- El chequeo semántico valida que el tipo inferido del inicializador sea compatible con la anotación.
- Si el nombre de tipo anotado no existe o no conforma con el inicializador, se reporta error semántico/de tipo y el pipeline se detiene antes de codegen.

### Tipos de usuario (`type`)

Sintaxis soportada:

```hulk
type Point(x: Number, y: Number) {
  x = x;
  y = y;

  norm() => sqrt(self.x ^ 2 + self.y ^ 2);
  add(other: Point) => new Point(self.x + other.x, self.y + other.y);
  describe() => "(" @ self.x @ ", " @ self.y @ ")";
}

let p = new Point(3, 4);
print(p.describe() @ " norm=" @ p.norm());
```

Reglas principales:

- Los tipos se declaran con `type`.
- Un tipo puede heredar de otro con `inherits Parent(...)`; si no se especifica padre, hereda de `Object`.
- Los argumentos de `inherits Parent(...)` inicializan el constructor del padre.
- Los atributos se inicializan en el cuerpo del tipo.
- Los métodos se declaran dentro del tipo y reciben `self` implícitamente.
- Los atributos son privados fuera del tipo.
- La resolución de métodos busca primero en el tipo actual y luego en sus padres.

Ejemplo de herencia:

```hulk
type Entity(name: String) {
  name = name;
  label() => self.name;
}

type User(name: String, role: String) inherits Entity(name) {
  role = role;
  profile() => self.label() @ " [" @ self.role @ "]";
}

let entity: Entity = new User("Ada", "admin");
let user = new User("Grace", "operator");
print(user.profile());
print(entity.label());
```

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

function tan(x: Number): Number => sin(x) / cos(x);
```

Reglas principales:

- Se declaran globalmente.
- Se permite recursión.
- Se valida aridad de llamada.
- Se valida tipo de cada argumento.
- Se infiere tipo de retorno desde el cuerpo y su contexto de uso.

### Anotación de tipos en firmas de función

Sintaxis soportada:

```hulk
function tan(x: Number): Number => sin(x) / cos(x);
function pick(a, b: Number, c): Number => b;
function banner(prefix: String): String {
  prefix @ "!"
}
```

Reglas:

- Se puede anotar todos o solo algunos parámetros.
- La anotación de retorno es opcional.
- Tipos válidos en anotación: `Number`, `Boolean`, `String`, `Unit`.
- En declaración, el chequeo semántico valida consistencia del cuerpo con parámetros/retorno anotados.
- En invocación, el chequeo semántico valida que cada argumento conforma al tipo anotado del parámetro.

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
- `examples/type_annotations_ok.hulk`
- `examples/type_annotations_let_in_ok.hulk`
- `examples/function_type_annotations_ok.hulk`
- `examples/function_type_annotations_partial_ok.hulk`
- `examples/types_point_ok.hulk`
- `examples/inheritance_polymorphism_ok.hulk`
- `examples/avl_inheritance_ok.hulk`

### Con error (diagnósticos)

- `examples/builtin_math_type_error.hulk`
- `examples/power_type_error.hulk`
- `examples/destructive_assign_type_error.hulk`
- `examples/error_scope_leak.hulk`
- `examples/while_condition_type_error.hulk`
- `examples/type_annotations_type_mismatch_error.hulk`
- `examples/type_annotations_unknown_type_error.hulk`
- `examples/function_type_annotations_param_type_error.hulk`
- `examples/function_type_annotations_return_type_error.hulk`
- `examples/function_type_annotations_body_type_error.hulk`
- `examples/function_type_annotations_unknown_type_error.hulk`

## Notas

- Extensión recomendada: `.hulk`.
- La CLI aún acepta `.hk` por compatibilidad.
- `print` devuelve `Unit` (no imprimible como argumento de otro `print`).
- `:=` exige conservar tipo.
