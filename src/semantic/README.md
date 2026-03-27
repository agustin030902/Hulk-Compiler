# Semantic

Este modulo implementa el analisis semantico del compilador.

Su trabajo es recorrer el AST que sale del parser, validar scopes, redeclaraciones,
uso de variables, y compatibilidad de tipos antes de que el pipeline llegue a `codegen`.

## Estructura actual

```text
src/semantic/
  mod.rs
  README.md
  analyzer.rs
  statement.rs
  helper/
    mod.rs
    types.rs
  expr/
    mod.rs
    binary.rs
    block.rs
    builtin_call.rs
    destructive_assign.rs
    let_in.rs
    literal.rs
    unary.rs
    variable.rs
  tests/
    ...
```

## Rol de cada archivo

- `mod.rs`
  - Fachada publica del modulo.
  - Expone `SemanticAnalyzer` y `SemanticType`.

- `analyzer.rs`
  - Contiene la clase principal `SemanticAnalyzer`.
  - Guarda el estado compartido del analisis:
    - `scopes`
    - `errors`
  - Define operaciones base como:
    - `analyze`
    - `push_scope`
    - `pop_scope`
    - `lookup`
    - `find_scope_index`
    - `push_type_error`
    - `push_semantic_error`

- `statement.rs`
  - Maneja el dispatch de `Statement`.
  - Aqui vive la logica de:
    - `let`
    - `=`
    - expression statements
    - `Statement::Print` si algun dia la gramatica vuelve a construirlo

- `expr/mod.rs`
  - Punto de dispatch de `Expr`.
  - Solo enruta cada variant del AST al archivo correspondiente.

- `expr/*.rs`
  - Cada archivo contiene la logica de una categoria concreta del AST.
  - Esto es lo que reduce conflictos entre varias personas trabajando en paralelo.

- `helper/`
  - Solo debe contener tipos o utilidades compartidas por varias expresiones o statements.
  - Ahora mismo contiene `SemanticType`.
  - Si algo es especifico de una expresion, no debe ir aqui.

## Flujo interno

El flujo real es:

```text
Compiler::compile
  -> parser construye Program
  -> SemanticAnalyzer::analyze(&program, source)
  -> recorre program.statements
  -> statement.rs
  -> expr/mod.rs
  -> expr/<nodo>.rs
```

Durante el analisis:

- siempre existe al menos un scope base
- los `Block` hacen `push_scope()` y `pop_scope()`
- los `let ... in ...` hacen `push_scope()` y `pop_scope()`
- las variables se buscan del scope mas interno al mas externo
- los errores semanticos y de tipos se acumulan en `errors`

## Regla de organizacion

Usa esta regla para mantener la estructura consistente:

- Si el codigo depende de un nodo concreto del AST, va en `expr/<nodo>.rs` o `statement.rs`.
- Si el codigo es estado compartido del analizador, va en `analyzer.rs`.
- Si el codigo es una utilidad o tipo compartido por varios archivos del modulo, va en `helper/`.

## Como agregar una nueva expresion

Supongamos que agregas una nueva variante al AST, por ejemplo `Expr::IfElse`.

Hazlo en este orden:

1. Extiende el AST en `src/parser/expression.rs`.
2. Actualiza la gramatica/parser para que construya el nuevo nodo.
3. Crea un archivo nuevo en `src/semantic/expr/`, por ejemplo `if_else.rs`.
4. Declara el modulo en `src/semantic/expr/mod.rs`.
5. Agrega el dispatch del nuevo variant en `check_expr`.
6. Implementa la validacion semantica dentro de `impl SemanticAnalyzer`.
7. Si necesitas tipos o helpers reutilizables por varios nodos, agregalos en `src/semantic/helper/`.
8. Agrega o ajusta tests en `src/semantic/tests/`.
9. Luego replica el soporte en `codegen` si la expresion debe generar LLVM IR.

## Como repartir trabajo entre varias personas

La idea de esta refactorizacion es que el equipo pueda dividir ownership por nodo:

- una persona puede trabajar en `expr/binary.rs`
- otra en `expr/let_in.rs`
- otra en `expr/builtin_call.rs`

Los archivos con mas posibilidad de conflicto siguen siendo:

- `expr/mod.rs`
- `statement.rs`
- `analyzer.rs`
- `helper/*`

Por eso, cuando agreguen una expresion nueva, intenten que una sola persona toque:

- el cambio de dispatch
- el archivo nuevo de la expresion
- sus tests

y que el resto del trabajo vaya en archivos separados.

## Convencion para helpers

Antes de crear algo en `helper/`, preguntate esto:

- Lo usan varias expresiones o statements: entonces si puede vivir en `helper/`.
- Solo lo usa una expresion: debe vivir en el archivo de esa expresion.
- Es estado principal del analizador: debe vivir en `analyzer.rs`.

Esa regla evita que `helper/` se convierta otra vez en un archivo grande disfrazado.
