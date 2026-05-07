# Semantic

Este módulo implementa el análisis semántico del compilador Hulk.

Valida:
- scopes y redeclaraciones
- uso de variables antes de declarar
- compatibilidad de tipos en expresiones
- contratos de funciones (cantidad/tipos de parámetros y tipo de retorno)

## Estructura actual

```text
src/semantic/
  mod.rs
  analyzer.rs
  statement.rs
  helper/
    mod.rs
    types.rs
    scope.rs
    function.rs
  expr/
    mod.rs
    binary.rs
    block.rs
    builtin_call.rs
    destructive_assign.rs
    function_call.rs
    if_expr.rs
    let_in.rs
    literal.rs
    unary.rs
    variable.rs
    while_expr.rs
  tests/
    ...
```

## Tipos semánticos

`SemanticType` modela el tipo lógico del lenguaje:
- `Number`
- `Boolean`
- `String`
- `Unit`
- `Function(u32)`
- `Struct(u32)`
- `Unknown`

`Unknown` se usa durante inferencia y debe resolverse antes de codegen.

## Scope aislado en helper

El stack de scopes ya no vive como `Vec<HashMap<...>>` directo en `analyzer.rs`.
Ahora está abstraído en [`ScopeStack`](./helper/scope.rs):
- `push` / `pop`
- `lookup`
- `lookup_with_index`
- `contains_in_current`
- `assign_at`

Esto reduce responsabilidades de `SemanticAnalyzer` y centraliza la lógica de alcance.

## Firmas de función

Las funciones se guardan como [`FunctionSignature`](./helper/function.rs):
- `type_id`
- `param_types: Vec<SemanticType>`
- `return_type: SemanticType`

Ya no se guarda solo la aridad.

## Inferencia de tipos de función

`SemanticAnalyzer` usa un proceso por punto fijo:

1. Registra todas las funciones con tipos `Unknown`.
2. Ejecuta pasadas de inferencia (máximo `MAX_INFERENCE_PASSES`) con errores suprimidos.
3. Reaplica las firmas inferidas.
4. Ejecuta el chequeo final con errores habilitados.
5. Si quedan `Unknown`, reporta errores explícitos.

Esto permite inferir casos encadenados y recursivos, incluyendo patrones tipo:

```hulk
function id(x) => x;
function plus_one(y) => id(y) + 1;
```

Aquí `id` se resuelve por contexto de retorno/uso, no solo por su cuerpo aislado.

## Reglas principales

### Variables
- `let x = expr;` declara en scope actual.
- `x = expr;` requiere variable declarada (puede cambiar tipo, por diseño actual).
- `x := expr` requiere mismo tipo que la variable original.
- Bloques `{ ... }` y `let ... in ...` crean scope.

### Funciones
- Se registran globalmente antes de analizar cuerpos (soporta recursión).
- No se permite redeclarar función.
- En llamadas se valida:
  - existencia
  - aridad
  - tipo de cada argumento
- El tipo de retorno se usa para validar contextos (`+`, `@`, `if`, etc.).

### Expresiones
- `+ - * / ^` => `Number`
- `@` => `(String,String) | (String,Number) | (Number,String)`
- comparaciones numéricas => `Boolean`
- `== !=` requieren mismo tipo comparable
- `&& || !` sobre booleanos
- `while` devuelve `Unit`

## Convención de cambios

- Lógica compartida de estado/tipos en `helper/`.
- Reglas por nodo en `expr/*.rs`.
- Dispatch en `expr/mod.rs` y `statement.rs`.
- Cualquier feature nuevo debe venir con tests en `src/semantic/tests/`.

## Estado actual

La semántica ya entrega firmas concretas de función para que `codegen` emita LLVM tipado
(ya no limitado a funciones numéricas únicamente).
