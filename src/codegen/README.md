# Codegen

Este módulo convierte el AST validado a LLVM IR.

Backend actual:
- `llvm` (`src/codegen/llvm`)

Interfaz pública:

```rust
pub trait CodegenBackend {
    fn generate(&mut self, program: &Program) -> Result<String, Vec<CompilerError>>;
}
```

## Estructura actual

```text
src/codegen/
  mod.rs
  README.md
  llvm/
    mod.rs
    backend.rs
    statement.rs
    helper/
      mod.rs
      state.rs
      module_writer.rs
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
    tests.rs
```

## Tipos internos de LLVM

`ValueType` modela los tipos emitidos:
- `Double` (`double`)
- `Bool` (`i1`)
- `StringPtr` (`i8*`)
- `Unit` (`i8`)
- `Function` (`i8*`, reservado)
- `Struct(u32)` (`i8*`, reservado)

`ValueRef` combina:
- `value_type`
- `repr` (registro/constante LLVM)

## Flujo de generación

En `LlvmBackend::generate`:

1. `reset()` limpia estado.
2. `load_function_signatures(program)` corre semántica y carga firmas tipadas de funciones.
3. `emit_program(program)` emite funciones globales y luego `main`.
4. `compose_module()` arma el módulo final.

Si cualquier etapa produce errores, retorna `Err(Vec<CompilerError>)`.

## Funciones de usuario: tipado correcto

Codegen ya no asume `double` para todas las funciones.

Ahora:
- cada función usa la firma inferida por semántica (`param_types` + `return_type`)
- la definición LLVM usa esos tipos reales
- cada `call` valida tipos de argumentos contra la firma
- el tipo de retorno de la llamada también es tipado

Ejemplos válidos actuales:
- función numérica recursiva (`double -> double`)
- función que retorna `String` (`i8*`)
- recursión que devuelve `String`

## Scopes en codegen

`LlvmBackend` mantiene scopes independientes para variables locales:

```text
Vec<HashMap<String, VariableInfo>>
```

`VariableInfo` contiene:
- puntero LLVM (`ptr_name`)
- tipo (`value_type`)

Bloques y `let-in` hacen `push_scope` / `pop_scope`.

## Emisión por nodo

### Statements
- `Let`: evalúa, `alloca`, `store`, registra símbolo.
- `Assign`: busca símbolo y reasigna (si cambia tipo, crea almacenamiento nuevo en ese scope).
- `Expr`: evalúa y descarta si no se usa.
- `Print`: soporte interno (la gramática suele usar `BuiltinCall::Print`).

### Expr
- `Literal`: números/bools/strings (strings como global + `getelementptr`).
- `Variable`: `load` desde `ptr_name`.
- `Unary`: `fneg` o `xor` según tipo.
- `Binary`: aritmética, comparación, igualdad, lógica, concat con `asprintf`.
- `If`: bloques y `phi` para resultados no-`Unit`.
- `While`: control flow + retorno `Unit`.
- `LetIn`/`Block`: scope temporal.
- `FunctionCall`: `call` tipado por firma inferida.

## Runtime declarations emitidas

El módulo LLVM declara:
- `printf`, `asprintf`, `strcmp`, `rand`, `time`, `srand`
- intrinsics: `llvm.sin.f64`, `llvm.cos.f64`, `llvm.sqrt.f64`, `llvm.exp.f64`, `llvm.log.f64`, `llvm.pow.f64`

Además define `@main` e inyecta seed de `rand()` con `time()`.

## Notas de diseño

- Semántica sigue siendo la fuente de verdad de tipos.
- Codegen mantiene validaciones defensivas para evitar IR inválido.
- Errores de codegen reportan línea/columna `1,1` (no usan spans del source).

## Estado actual

Con la integración de firmas tipadas desde semántica, el backend LLVM ya soporta
funciones de usuario más allá del caso numérico fijo inicial.
