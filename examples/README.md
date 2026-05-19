# Hulk Compiler Examples

Esta carpeta contiene archivos de prueba (`.hulk`) para validar el compilador Hulk. La CLI acepta también `.hk` como alias legacy, pero todos los ejemplos se distribuyen con la nueva extensión.

## Archivos de Ejemplo Válidos (OK)

### `recursive_number_fibonacci.hulk`
Recursión numérica con `if/elif/else`.
- `fib(n)` clásico
- Llamadas recursivas múltiples
- Concatenación de `String @ Number` para mostrar resultados

### `recursive_string_countdown.hulk`
Recursión que retorna `String`.
- Construye una cadena `"0 -> 1 -> ... -> n"`
- Mezcla de recursión + concatenación
- Retorno de función no numérico

### `recursive_number_gcd.hulk`
Recursión numérica para máximo común divisor (algoritmo de Euclides).
- `mod(x, y)` implementado por restas recursivas
- `gcd(a, b)` con llamada recursiva `gcd(b, mod(a, b))`
- Ejemplo clásico de recursión compuesta entre funciones

### `recursive_number_power.hulk`
Potencia entera recursiva.
- `pow_int(base, exp)` con caso base `exp == 0`
- Multiplicación recursiva decreciendo exponente
- Función numérica pura con dos parámetros

### `recursive_number_sum_to_n.hulk`
Suma acumulada recursiva.
- `sum_to(n)` calcula `1 + 2 + ... + n`
- Caso base `n == 0`
- Recursión lineal simple para validar inferencia de retorno numérico

### `recursive_number_factorial_tail.hulk`
Factorial usando recursión con acumulador.
- `fact_tail(n, acc)` como versión tail-style
- `fact(n)` como wrapper para iniciar acumulador en `1`
- Ejemplo de recursión con estado explícito por parámetros

### `recursive_mutual_even_odd.hulk`
Recursión mutua entre funciones.
- `is_even` llama a `is_odd` y viceversa
- Caso base en `n == 0`
- Demuestra referencias cruzadas entre firmas globales

### `recursive_string_repeat.hulk`
Recursión de `String` con múltiples parámetros.
- `repeat_with_sep(n, text)` construye una secuencia separada por comas
- Usa `if/elif/else` recursivo
- Combina concatenación de strings y decremento numérico

### `mochila_funcionalidades.hulk`
Ejemplo “mochila” con varias funcionalidades actuales del compilador.
- Recursión numérica y recursión `String`
- `while` como expresión (`Unit`)
- Asignación destructiva `:=`
- Bloques con scope léxico
- `let ... in ...`
- Builtins (`sin`, `cos`, `sqrt`, `exp`, `log`, `rand`)
- `print(...)` como expresión

### `types_point_ok.hulk`
Ejemplo de tipos nominales con instanciación y métodos.
- Declaración de `type Point` con atributos y métodos
- Uso de `new Point(...)`
- Llamadas a métodos por `.` y uso de `self`

### `calculator_ok.hulk`
Ejemplo básico con operaciones aritméticas simples.
- Declara variables con valores numéricos
- Suma de dos números
- Imprime el resultado

### `arithmetic_operations.hulk`
Demuestra todas las operaciones aritméticas básicas.
- Suma (+)
- Resta (-)
- Multiplicación (*)
- Llamadas a `print()` múltiples

### `unary_operator.hulk`
Prueba el operador unario de negación (-).
- Negación de variable
- Negación de literal

### `string_operations.hulk`
Declaración y impresión de strings.
- Literales de string
- Impresión de strings

### `boolean_literals.hulk`
Declaración de booleanos.
- Literales `true` y `false`
- Impresión de booleanos

### `complex_expressions.hulk`
Expresiones complejas con precedencia de operadores.
- Operadores combinados
- Paréntesis para cambiar precedencia

### `floats.hulk`
Números de punto flotante.
- Literales con decimales
- Operaciones entre floats

### `builtin_math_ok.hulk`
Uso de builtins y constantes matemáticas.
- `sin`, `cos`, `sqrt`, `exp`, `log`
- constantes `PI` y `E`

### `reassignment_ok.hulk`
Reasignación de variables.
- `let x = ...;`
- `x = ...;`

### `power_ok.hulk`
Operador de potencia `^` con precedencia y asociatividad.
- Potencia simple y encadenada (`2 ^ 3 ^ 2`)
- Combinación con builtins (`sin(...) ^ 2`)

### `rand_ok.hulk`
Builtin `rand()` para generar números pseudoaleatorios.
- `rand()` retorna un `Number` entre `0` y `1`
- Uso en asignaciones y `print(...)`

### `expression_statement_ok.hulk`
Statements de expresión en un lenguaje basado en expresiones.
- Expresiones como `42;`, `x;`, `(1 + 2) * 3;`
- No imprimen por sí mismas, salvo que uses `print(...)`

### `block_scope_ok.hulk`
Bloques como expresiones con scope léxico.
- Shadowing dentro del bloque
- El valor del bloque es la última expresión (sin `;` obligatorio)
- Variables internas no se filtran afuera

### `destructive_assign_ok.hulk`
Asignación destructiva `:=` que sobrescribe variables declaradas.
- Devuelve el valor asignado (es una expresión)
- Mantiene el tipo declarado

### `print_expr_ok.hulk`
`print` como expresión de efecto lateral que devuelve `Unit`.
- Puede usarse dentro de `let ... in ...`
- El valor de retorno es `Unit` y puede ignorarse o almacenarse

### `while_ok.hulk`
Loop `while` como expresión.
- La condición debe evaluar a `Boolean`
- El cuerpo es un bloque `{ ... }`
- El valor de retorno del `while` es `Unit`

### `let_in_ok.hulk`
Ligaduras locales con `let ... in ...`.
- Varias ligaduras separadas por coma
- Scope limitado al cuerpo (`in ...`)
- Devuelve el valor de la última expresión del cuerpo

### `let_in_shadow.hulk`
Shadowing dentro de `let-in`.
- La ligadura más interna es la que se usa en el cuerpo
- La variable externa mantiene su valor original

## Archivos de Prueba de Error

### `error_type_mismatch_add.hulk`
**Tipo de error:** Semantic - Type Mismatch
- Intenta sumar un string con un número
- Error esperado: `Operator '+' expects Number and Number, but got String and Number`

### `error_type_mismatch_mul.hulk`
**Tipo de error:** Semantic - Type Mismatch
- Intenta multiplicar un booleano con un número
- Error esperado: Incompatibilidad de tipos

### `error_type_mismatch_div.hulk`
**Tipo de error:** Semantic - Type Mismatch
- Intenta dividir un string entre un número
- Error esperado: El operador `/` requiere dos números

### `error_syntax_missing_semicolon.hulk`
**Tipo de error:** Syntax
- Declaración sin punto y coma al final
- Error esperado: Error de análisis sintáctico

### `error_syntax_incomplete_expr.hulk`
**Tipo de error:** Syntax
- Operador sin el segundo operando
- Error esperado: Expresión incompleta

### `error_syntax_unmatched_paren.hulk`
**Tipo de error:** Syntax
- Paréntesis de apertura sin cerrar
- Error esperado: Paréntesis no emparejado

### `error_undefined_variable.hulk`
**Tipo de error:** Semantic - Undefined Variable
- Intenta usar una variable que no fue declarada
- Error esperado: Variable no definida

### `error_lexical_invalid.hulk`
**Tipo de error:** Lexical
- Tokens inválidos o malformados
- Error esperado: Token desconocido

### `builtin_math_type_error.hulk`
**Tipo de error:** Semantic - Type Mismatch
- Llamada a `log` con tipos inválidos
- Error esperado: `Function 'log' expects (Number, Number), but got Number and String`

### `power_type_error.hulk`
**Tipo de error:** Semantic - Type Mismatch
- Uso de `^` con `String` y `Number`
- Error esperado: `Operator '^' expects Number and Number, but got String and Number`

### `rand_invalid_args.hulk`
**Tipo de error:** Syntax
- Llamada inválida `rand(1)`
- Error esperado: `rand` solo admite 0 argumentos

### `destructive_assign_type_error.hulk`
**Tipo de error:** Semantic - Type Mismatch
- Reasigna con `:=` cambiando el tipo de la variable
- Error esperado: mensaje de tipo incompatible en `:=`

### `identifier_invalid.hulk`
**Tipo de error:** Lexical
- Usa identificadores inválidos (`_x`, `8ball`)
- Error esperado: reporte léxico de tokens desconocidos

### `let_in_type_error.hulk`
**Tipo de error:** Semantic - Type Mismatch
- Usa `let a = true in a + 1`
- Error esperado: `Operator '+' expects Number and Number`

### `let_in_parser_error.hulk`
**Tipo de error:** Syntax
- Falta la palabra clave `in` en una expresión `let`
- Error esperado: error de análisis sintáctico en `let`

### `error_scope_leak.hulk`
**Tipo de error:** Semantic - Undefined Variable
- Usa una variable declarada dentro de un bloque fuera de su scope
- Error esperado: variable no declarada

### `while_condition_type_error.hulk`
**Tipo de error:** Semantic - Type Mismatch
- Usa un `while` con condición no booleana
- Error esperado: `While condition expects Boolean, but got Number`

### `unit_type_error.hulk`
**Tipo de error:** Semantic - Type Mismatch
- Usa un valor `Unit` en una operación aritmética
- Error esperado: `Operator '+' expects Number and Number, but got Unit and Number`

## Cómo ejecutar los ejemplos

### Ejecutar un archivo individual:
```bash
cargo run -- --input examples/calculator_ok.hulk --emit-ir artifacts/output.txt
```

### Ejecutar todos los archivos `.hulk`:
```bash
cargo run -- --run-all examples --emit-dir artifacts/batch
```

Esto generará un archivo `.txt` por cada `.hulk` con:
- LLVM IR (si compila correctamente)
- Diagnóstico de errores (si hay problemas)
