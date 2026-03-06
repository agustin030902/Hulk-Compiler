# Hulk Compiler Examples

Esta carpeta contiene archivos de prueba (`.hk`) para validar el compilador Hulk.

## Archivos de Ejemplo Válidos (OK)

### `calculator_ok.hk`
Ejemplo básico con operaciones aritméticas simples.
- Declara variables con valores numéricos
- Suma de dos números
- Imprime el resultado

### `arithmetic_operations.hk`
Demuestra todas las operaciones aritméticas básicas.
- Suma (+)
- Resta (-)
- Multiplicación (*)
- Llamadas a `print()` múltiples

### `unary_operator.hk`
Prueba el operador unario de negación (-).
- Negación de variable
- Negación de literal

### `string_operations.hk`
Declaración y impresión de strings.
- Literales de string
- Impresión de strings

### `boolean_literals.hk`
Declaración de booleanos.
- Literales `true` y `false`
- Impresión de booleanos

### `complex_expressions.hk`
Expresiones complejas con precedencia de operadores.
- Operadores combinados
- Paréntesis para cambiar precedencia

### `floats.hk`
Números de punto flotante.
- Literales con decimales
- Operaciones entre floats

### `builtin_math_ok.hk`
Uso de builtins y constantes matemáticas.
- `sin`, `cos`, `sqrt`, `exp`, `log`
- constantes `PI` y `E`

### `reassignment_ok.hk`
Reasignación de variables.
- `let x = ...;`
- `x = ...;`

### `power_ok.hk`
Operador de potencia `^` con precedencia y asociatividad.
- Potencia simple y encadenada (`2 ^ 3 ^ 2`)
- Combinación con builtins (`sin(...) ^ 2`)

## Archivos de Prueba de Error

### `error_type_mismatch_add.hk`
**Tipo de error:** Semantic - Type Mismatch
- Intenta sumar un string con un número
- Error esperado: `Operator '+' expects Number and Number, but got String and Number`

### `error_type_mismatch_mul.hk`
**Tipo de error:** Semantic - Type Mismatch
- Intenta multiplicar un booleano con un número
- Error esperado: Incompatibilidad de tipos

### `error_type_mismatch_div.hk`
**Tipo de error:** Semantic - Type Mismatch
- Intenta dividir un string entre un número
- Error esperado: El operador `/` requiere dos números

### `error_syntax_missing_semicolon.hk`
**Tipo de error:** Syntax
- Declaración sin punto y coma al final
- Error esperado: Error de análisis sintáctico

### `error_syntax_incomplete_expr.hk`
**Tipo de error:** Syntax
- Operador sin el segundo operando
- Error esperado: Expresión incompleta

### `error_syntax_unmatched_paren.hk`
**Tipo de error:** Syntax
- Paréntesis de apertura sin cerrar
- Error esperado: Paréntesis no emparejado

### `error_undefined_variable.hk`
**Tipo de error:** Semantic - Undefined Variable
- Intenta usar una variable que no fue declarada
- Error esperado: Variable no definida

### `error_lexical_invalid.hk`
**Tipo de error:** Lexical
- Tokens inválidos o malformados
- Error esperado: Token desconocido

### `builtin_math_type_error.hk`
**Tipo de error:** Semantic - Type Mismatch
- Llamada a `log` con tipos inválidos
- Error esperado: `Function 'log' expects (Number, Number), but got Number and String`

### `power_type_error.hk`
**Tipo de error:** Semantic - Type Mismatch
- Uso de `^` con `String` y `Number`
- Error esperado: `Operator '^' expects Number and Number, but got String and Number`

## Cómo ejecutar los ejemplos

### Ejecutar un archivo individual:
```bash
cargo run -- --input examples/calculator_ok.hk --emit-ir artifacts/output.txt
```

### Ejecutar todos los archivos `.hk`:
```bash
cargo run -- --run-all examples --emit-dir artifacts/batch
```

Esto generará un archivo `.txt` por cada `.hk` con:
- LLVM IR (si compila correctamente)
- Diagnóstico de errores (si hay problemas)
