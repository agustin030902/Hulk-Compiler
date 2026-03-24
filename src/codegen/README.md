# Codegen

Este modulo implementa la ultima fase del compilador: convertir el AST validado a LLVM IR.

Hoy solo existe un backend concreto: [`llvm`](./llvm), expuesto por el trait [`CodegenBackend`](./mod.rs).

## Que recibe codegen

`codegen` no recibe texto fuente ni tokens. Tampoco recibe un AST "tipado" distinto al del parser.

Lo que entra es esto:

```rust
pub trait CodegenBackend {
    fn generate(&mut self, program: &Program) -> Result<String, Vec<CompilerError>>;
}
```

O sea:

- Recibe un `Program` del parser.
- Ese `Program` ya paso por `semantic`.
- Si `lexer`, `parser` o `semantic` fallan, `codegen` ni siquiera se ejecuta.

El orden real esta en `src/compiler/mod.rs`:

```text
source
  -> lexer.lex()
  -> parser.parse_program(...)
  -> semantic.analyze(...)
  -> llvm_backend.generate(&program)
```

Importante: aunque `semantic` ya valida scopes y tipos, `codegen` vuelve a comprobar varias cosas. Eso sirve como red de seguridad, pero esos errores de `codegen` salen con linea/columna `1,1` porque esta fase no usa spans para diagnosticos.

## AST que entra

El backend trabaja directamente sobre estas piezas:

- `Program { statements: Vec<Statement> }`
- `Statement::Let`
- `Statement::Assign`
- `Statement::Expr`
- `Statement::Print` existe en el AST, pero la gramatica actual no lo construye
- `Expr::Literal`
- `Expr::Variable`
- `Expr::Unary`
- `Expr::Binary`
- `Expr::BuiltinCall`
- `Expr::DestructiveAssign`
- `Expr::LetIn`
- `Expr::Block`

La gramatica actual parsea `print(...)` como `Expr::BuiltinCall(BuiltinFunction::Print)`, no como `Statement::Print`.

## Tipos internos que usa el backend

Aunque el lenguaje tiene `Number`, `Boolean` y `String`, el backend los baja a estos tipos LLVM internos:

- `Number` -> `double`
- `Boolean` -> `i1`
- `String` -> `i8*`

Internamente eso vive como:

- `ValueType::Double`
- `ValueType::Bool`
- `ValueType::StringPtr`

Cada subexpresion devuelve un `ValueRef`:

```rust
struct ValueRef {
    value_type: ValueType,
    repr: String,
}
```

`repr` es el nombre LLVM que representa el valor actual, por ejemplo `%t7`, `1`, `3.14`, o un `getelementptr`.

## Flujo interno de `LlvmBackend`

Cuando llamas `generate(&program)`, el backend hace esto:

1. `reset()`
2. Limpia instrucciones previas, globals, errores, contadores y scopes
3. Crea el scope base
4. Recorre `program.statements` con `emit_program()`
5. Cada statement termina llamando de forma recursiva a `emit_expr()`
6. Si hubo errores, devuelve `Err(Vec<CompilerError>)`
7. Si todo sale bien, `compose_module()` arma el modulo LLVM completo y devuelve el `String`

El backend mantiene varios estados durante la emision:

- `body_lines`: instrucciones dentro de `main`
- `global_lines`: strings globales y constantes auxiliares
- `scopes`: stack de scopes con variables visibles
- `temp_counter`: para generar `%t0`, `%t1`, `%t2`, ...
- `string_counter`: para generar `@.str.0`, `@.str.1`, ...

## Como maneja los scopes

`LlvmBackend` tiene su propia tabla de simbolos:

```rust
HashMap<String, VariableInfo>
```

Cada variable guarda:

- `ptr_name`: el puntero LLVM donde vive el valor
- `value_type`: el tipo actual (`double`, `i1`, `i8*`)

Reglas actuales:

- `let` declara en el scope actual
- se permite shadowing en scopes internos
- `lookup_var()` busca de adentro hacia afuera
- `Block` hace `push_scope()` al entrar y `pop_scope()` al salir
- `let ... in ...` tambien hace `push_scope()`/`pop_scope()`
- `=` busca la variable existente mas cercana
- `:=` tambien busca la variable existente mas cercana, pero no deja cambiarle el tipo

## Como maneja cada statement

### 1. `Statement::Let`

Entrada ejemplo:

```hulk
let x = 42;
```

Proceso:

1. Evalua la expresion del lado derecho
2. Crea un `alloca` del tipo resultante
3. Hace `store`
4. Guarda la variable en el scope actual

Si el nombre ya existe en el scope actual, produce error.

### 2. `Statement::Assign`

Entrada ejemplo:

```hulk
x = expr;
```

Proceso:

1. Busca la variable ya declarada
2. Evalua `expr`
3. Si el tipo coincide, hace `store` en el mismo puntero
4. Si el tipo cambia, crea un `alloca` nuevo y reemplaza la entrada en la tabla del scope donde se encontro la variable

Ese detalle es importante: la reasignacion con `=` hoy permite cambio de tipo porque asi lo permite `semantic`.

### 3. `Statement::Expr`

Entrada ejemplo:

```hulk
1 + 2;
rand();
{ let x = 1; x + 1 };
```

Proceso:

- Se emite la expresion
- El valor calculado se ignora si nadie lo usa despues

Esto permite programas basados en expresiones aunque no impriman nada.

### 4. `Statement::Print`

La implementacion existe, imprime el valor y devuelve el mismo `ValueRef`.

Pero hoy la gramatica actual no construye este variant. La ruta real para imprimir pasa por `BuiltinFunction::Print`.

## Como maneja cada expresion existente hoy

### Literales

#### `Literal::Integer`

- Se convierte directamente a `double`
- Ejemplo: `7` termina como `"7.0"`

#### `Literal::Float`

- Se representa como `double`
- `format_double()` limpia ceros sobrantes

#### `Literal::Boolean`

- `true` -> `i1 1`
- `false` -> `i1 0`

#### `Literal::String`

Proceso:

1. Se crea una constante global privada `@.str.N`
2. Se escapan bytes con `escape_llvm_string()`
3. En el cuerpo se genera un `getelementptr` al primer byte
4. El resultado final es un `i8*`

Ejemplo conceptual:

```llvm
@.str.0 = private unnamed_addr constant [5 x i8] c"hola\00"
%t0 = getelementptr inbounds [5 x i8], [5 x i8]* @.str.0, i64 0, i64 0
```

### Variables

`Expr::Variable`:

1. Busca la variable visible mas cercana
2. Genera `load`
3. Devuelve un `ValueRef` con el tipo conocido en la tabla del backend

### Unary

#### `UnaryOp::Neg`

Entrada:

```hulk
-expr
```

Salida:

- exige `double`
- genera `fneg double`

#### `UnaryOp::Not`

Entrada:

```hulk
!expr
```

Salida:

- exige `i1`
- genera `xor i1 <valor>, true`

### Binary aritmetico

#### `+`, `-`, `*`, `/`

Todos requieren `Number`.

Mapeo:

- `+` -> `fadd`
- `-` -> `fsub`
- `*` -> `fmul`
- `/` -> `fdiv`

#### `^`

- Requiere `Number`
- Se baja a `call double @llvm.pow.f64(double left, double right)`

La asociatividad a derecha ya viene resuelta desde parser. `codegen` solo emite el arbol que recibe.

### Binary de comparacion

#### `<`, `>`, `<=`, `>=`

- Requieren `Number`
- Devuelven `Boolean`
- Usan `fcmp`

Mapeo:

- `<` -> `fcmp olt`
- `>` -> `fcmp ogt`
- `<=` -> `fcmp ole`
- `>=` -> `fcmp oge`

### Equality

#### `==` y `!=`

Los operandos deben tener el mismo tipo.

Mapeo por tipo:

- `Number` -> `fcmp oeq` / `fcmp one`
- `Boolean` -> `icmp eq` / `icmp ne`
- `String` -> `strcmp(...)` seguido de comparacion contra `0`

Para strings, la idea es:

```llvm
%cmp = call i32 @strcmp(i8* left, i8* right)
%eq = icmp eq i32 %cmp, 0
```

### Logicos

#### `&&` y `||`

- Requieren `Boolean`
- Usan instrucciones LLVM `and` y `or`

Importante: hoy no hay short-circuit. Ambos operandos se evaluan siempre antes de aplicar `and` u `or`.

### Concatenacion `@`

Casos soportados:

- `String @ String`
- `String @ Number`
- `Number @ String`

Implementacion actual:

1. Elige un formato global:
   - `@.fmt.concat.ss` -> `"%s%s"`
   - `@.fmt.concat.sn` -> `"%s%g"`
   - `@.fmt.concat.ns` -> `"%g%s"`
2. Reserva un slot local `alloca i8*`
3. Llama a `asprintf(i8** slot, i8* fmt, ...)`
4. Hace `load i8*, i8** slot`
5. Devuelve el `i8*` resultante

Eso hace que el resultado sea un string heap-allocated por libc.

### `BuiltinFunction::Print`

Esta es la forma real en que se imprime hoy.

Ejemplo:

```hulk
print(x)
```

Proceso:

1. Evalua el argumento
2. Llama a `emit_print_value()`
3. Devuelve el mismo valor, para que `print(...)` siga siendo expresion

Mapeo por tipo:

- `Number` -> `printf("%g\n", value)`
- `String` -> `printf("%s\n", value)`
- `Boolean` -> `zext i1 -> i32`, luego `printf("%d\n", value)`

Importante: los booleanos se imprimen como `0` y `1`, no como `false` y `true`.

### Builtins matematicas

#### `sin(expr)`

- exige `Number`
- `call double @llvm.sin.f64(double arg)`

#### `cos(expr)`

- exige `Number`
- `call double @llvm.cos.f64(double arg)`

#### `sqrt(expr)`

- exige `Number`
- `call double @llvm.sqrt.f64(double arg)`

#### `exp(expr)`

- exige `Number`
- `call double @llvm.exp.f64(double arg)`

#### `log(base, value)`

- exige dos `Number`
- se implementa como `ln(value) / ln(base)`

Secuencia:

```llvm
%ln_base = call double @llvm.log.f64(double base)
%ln_value = call double @llvm.log.f64(double value)
%result = fdiv double %ln_value, %ln_base
```

#### `rand()`

Proceso:

1. `call i32 @rand()`
2. `sitofp` a `double`
3. division entre `2147483647.0`

Devuelve un `Number` normalizado aproximadamente entre `0` y `1`.

Ademas, `compose_module()` siembra el generador al inicio de `main`:

```llvm
%t_seed_raw = call i64 @time(i64* null)
%t_seed_i32 = trunc i64 %t_seed_raw to i32
call void @srand(i32 %t_seed_i32)
```

### `Expr::Block`

Ejemplo:

```hulk
{
    let x = 9;
    let y = 1;
    x + y
}
```

Proceso:

1. Crea un scope nuevo
2. Emite sus statements en orden
3. Guarda el ultimo `ValueRef` producido
4. Sale del scope
5. Devuelve ese ultimo valor

Si ningun statement produce valor, `codegen` emite error: `Block expression must produce a value`.

### `Expr::LetIn`

Ejemplo:

```hulk
let a = 1, b = 2 in a + b
```

Proceso:

1. Crea un scope nuevo
2. Evalua bindings de izquierda a derecha
3. Cada binding hace `alloca + store`
4. Inserta cada nombre en el scope local
5. Emite el cuerpo
6. Sale del scope
7. Devuelve el valor del cuerpo

### `Expr::DestructiveAssign`

Ejemplo:

```hulk
x := x + 1
```

Proceso:

1. Busca la variable ya declarada
2. Evalua la expresion del lado derecho
3. Exige que el tipo sea exactamente el mismo
4. Hace `store` sobre el mismo puntero
5. Devuelve el valor asignado

Diferencia clave respecto a `=`:

- `=` es statement y puede cambiar el tipo
- `:=` es expresion y no puede cambiar el tipo

## Runtime y modulo LLVM que se genera

`compose_module()` arma un modulo con:

- declaraciones externas:
  - `printf`
  - `asprintf`
  - `strcmp`
  - `rand`
  - `time`
  - `srand`
  - intrinsics LLVM (`sin`, `cos`, `sqrt`, `exp`, `log`, `pow`)
- strings globales de formato
- strings literales del programa
- `define i32 @main()`

No hay funciones de usuario todavia. Todo se emite dentro de `main`.

## Resumen rapido de expresiones soportadas hoy

| Categoria | Variantes actuales | Como se bajan |
| --- | --- | --- |
| Literales | integer, float, boolean, string, `PI`, `E` | `double`, `i1`, `i8*`, globals |
| Variables | identificadores | `load` desde el puntero guardado |
| Unary | `-`, `!` | `fneg`, `xor i1 ... true` |
| Aritmetica | `+`, `-`, `*`, `/`, `^` | `fadd`, `fsub`, `fmul`, `fdiv`, `llvm.pow.f64` |
| Comparacion | `<`, `>`, `<=`, `>=` | `fcmp` |
| Igualdad | `==`, `!=` | `fcmp`, `icmp`, `strcmp` |
| Logica | <code>&amp;&amp;</code>, <code>&#124;&#124;</code> | <code>and</code>, <code>or</code> |
| Concat | `@` | `asprintf` |
| Builtins | `print`, `sin`, `cos`, `sqrt`, `exp`, `log`, `rand` | `printf`, intrinsics LLVM, libc |
| Scope expresivo | `Block`, `LetIn` | `push/pop scope` + valor final |
| Mutacion expresiva | `:=` | `store` con tipo fijo |

## Quirks y limitaciones actuales

Estas son cosas importantes para entender el estado real de la fase:

- `Statement::Print` sigue en el AST, pero la gramatica actual no lo usa.
- `codegen` recibe un AST validado, pero aun asi repite varias comprobaciones semanticas.
- Los errores producidos por `codegen` salen con linea/columna `1,1`.
- Los enteros se bajan como `double`; no existe un tipo entero separado en IR.
- `&&` y `||` no tienen short-circuit.
- `print(Boolean)` imprime `0` o `1`.
- `@` usa `asprintf`, asi que crea memoria dinamica que hoy no se libera.
- La igualdad de strings depende de `strcmp`.
- Todo vive dentro de `main`; todavia no hay funciones del usuario ni llamadas internas del lenguaje.
- Si un bloque esta vacio, `semantic` lo trata como `Unknown`, pero `codegen` luego lo rechaza porque exige un valor final.

## Si quieres agregar una expresion nueva

Para extender el lenguaje sin romper el pipeline, normalmente hay que tocar estas capas:

1. `src/parser/expression.rs`
   - agregar el variant nuevo al AST
2. `src/parser/grammar.lalrpop`
   - parsearlo y ubicarlo en la precedencia correcta
3. `src/semantic/mod.rs`
   - definir su tipo resultante y sus restricciones
4. `src/codegen/llvm/mod.rs`
   - agregar el caso en `emit_expr()` o `emit_statement()`
   - crear helpers si hace falta
   - declarar runtime/intrinsics extra en `compose_module()` si dependes de algo nuevo
5. tests
   - parser
   - semantic
   - compiler/codegen

Checklist mental:

- Que tipo devuelve la expresion
- Si crea scope o reusa scope actual
- Si necesita `alloca`, `load`, `store` o solo instrucciones SSA
- Si necesita runtime externo
- Si puede fallar por tipo o por aridad
- Si su valor debe poder usarse como subexpresion

## Idea corta para leer el backend de arriba hacia abajo

Si quieres entender rapido el archivo `src/codegen/llvm/mod.rs`, este es el recorrido mas util:

1. `generate()`
2. `reset()`
3. `emit_program()`
4. `emit_statement()`
5. `emit_expr()`
6. helpers:
   - `emit_builtin_call()`
   - `emit_literal()`
   - `emit_variable()`
   - `emit_block_expr()`
   - `emit_let_in_expr()`
   - `emit_destructive_assign()`
   - `emit_unary()`
   - `emit_binary()`
   - `emit_concat()`
7. `compose_module()`

Ese orden refleja casi exactamente el flujo real de ejecucion.
