# Reporte Técnico: Compilador del Lenguaje HULK

## 1. Introducción

### 1.1 Propósito del Documento

Este reporte documenta el diseño, la arquitectura y las decisiones de implementación de un compilador completo para el lenguaje de programación HULK, desarrollado en el marco de las asignaturas de Compilación y Lenguajes de Programación.

### 1.2 Descripción del Lenguaje HULK

HULK es un lenguaje de tipado estático con inferencia de tipos, diseñado con influencias de Rust, Python, Go y C#. Su rasgo más distintivo es adoptar un sistema de interfaces estructurales al estilo Go —donde la conformidad es implícita— en lugar del sistema nominal clásico de Java o C#. Esta decisión, como veremos, tiene implicaciones profundas en toda la arquitectura del compilador.

Entre sus características principales se encuentran:

- **Tipado estático con inferencia opcional**: El programador puede omitir anotaciones de tipo; el compilador las deduce.
- **Interfaces estructurales (protocols)**: Un tipo satisface una interfaz si implementa los métodos requeridos, sin necesidad de una declaración `implements`.
- **Herencia simple**: Con sintaxis `type Child inherits Parent`.
- **Iteradores mediante protocolos `Iterable` y `Enumerable`**: El bucle `for` soporta iteración directa o indirecta a través de un iterador separado.
- **Métodos virtuales y dispatch dinámico**: La resolución de métodos ocurre en tiempo de ejecución mediante una etiqueta de tipo almacenada en cada objeto.
- **Operadores `is` y `as`**: Para verificación y conversión segura de tipos en ejecución.
- **Hoisting**: Las declaraciones de tipos, interfaces y funciones pueden aparecer en cualquier orden dentro del archivo.

### 1.3 Alcance del Proyecto

El compilador abarca todas las etapas clásicas: análisis léxico, sintáctico, semántico y generación de código LLVM IR, que luego es compilado a ejecutable mediante `clang`. Se incluye también una interfaz gráfica interactiva construida con `egui/eframe`. El proyecto se implementa íntegramente en Rust.

## 2. Arquitectura General del Compilador

### 2.1 Descripción General

El compilador sigue un pipeline clásico de cuatro etapas independientes: **análisis léxico**, **análisis sintáctico**, **análisis semántico** y **generación de código**. Cada fase consume la salida de la anterior y produce datos estructurados para la siguiente. El modelo es **fail-fast**: en cuanto una fase detecta un error, el flujo se interrumpe y se devuelve un reporte de diagnóstico al usuario, sin continuar hacia etapas posteriores.

Una virtud de esta arquitectura es que permite probar cada fase de forma independiente. De hecho, el proyecto incluye suites de pruebas separadas para lexer, parser, semántica y codegen, lo que facilita enormemente la depuración.

### 2.2 Organización del Código

El proyecto se organiza en siete módulos principales dentro de `src/`, cada uno correspondiente a una fase o un componente auxiliar:

- `lexer/`: Tokenización mediante el generador `logos`.
- `parser/`: Análisis sintáctico con LALRPOP y definición del AST.
- `semantic/`: Análisis semántico en dos pasadas (recolección de símbolos + verificación de tipos).
- `codegen/`: Generación de código LLVM IR en formato texto.
- `compiler/`: Orquestación del pipeline completo.
- `error/`: Definición unificada de errores del compilador.

La separación en módulos refleja fielmente la arquitectura por capas. Cada módulo define una interfaz pública clara y oculta los detalles internos. Esto hizo posible, por ejemplo, reemplazar el backend de generación de código sin afectar al resto del compilador.

### 2.3 ¿Por qué Rust?

**Ventajas que aportó Rust:**

- **Seguridad de memoria sin GC**: El compilador maneja estructuras de datos complejas (AST, tablas de símbolos, grafos de tipos). Rust garantiza que no haya use-after-free ni data races, problemas comunes en compiladores escritos en C/C++. Y al no tener garbage collector, no hay pausas impredecibles durante la compilación.
- **Tipado expresivo**: Los `enum` de Rust con datos asociados permiten modelar el AST de forma natural y exhaustiva. El compilador fuerza a manejar todos los casos en los `match`, previniendo errores por omisión.
- **Rendimiento predecible**: Comparable a C++, crucial para compilar programas grandes sin demoras.
- **Ecosistema**: `logos` y `lalrpop` son bibliotecas Rust maduras para generación de lexers y parsers. La integración con Cargo hace que el proceso de build sea reproducible.

**Desventajas que enfrentamos:**

- **Curva de aprendizaje**: El equipo tuvo que aprender Rust y sus conceptos de ownership/borrowing antes de poder escribir el compilador.
- **Tiempos de compilación**: El proyecto, aunque modesto, ya muestra tiempos de compilación no triviales debido a las macros de lalrpop.
- **Ergonomía para ASTs mutables**: El árbol sintáctico se presta a patrones de programación con referencias mutables complejas. En varios puntos tuvimos que usar `Clone` donde en otros lenguajes usaríamos referencias compartidas.

**Balance**: Rust fue una elección acertada. La seguridad que aporta y la expresividad de sus tipos compensaron la curva de aprendizaje inicial. En retrospectiva, la alternativa más seria habría sido OCaml, cuyo sistema de tipos algebraicos es aún más natural para ASTs, pero hubiera requerido integrar herramientas de build diferentes.

---

### 3. Análisis Léxico (Lexer)

### 3.1 ¿Qué problema resuelve el lexer?

El lexer transforma el código fuente —una secuencia de caracteres— en una secuencia de **tokens**: unidades significativas como palabras clave, identificadores, operadores y literales. Es la primera línea de defensa contra errores: si un archivo contiene caracteres no válidos, el lexer lo detecta antes de que el parser intente interpretarlos.

### 3.2 ¿Por qué usar `logos` y no un lexer manual?

La decisión fundamental aquí fue entre escribir un lexer manual o usar un generador. Evaluamos tres opciones:

| Opción                                      | Beneficio                                                                                | Costo                                                                                             |
| -------------------------------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| **Lexer manual**                       | Control total, sin dependencias externas                                                 | Propenso a errores, mucho código boilerplate (gestión de estados, lookahead, manejo de errores) |
| **Regex manual** (con crate `regex`) | Más simple que un DFA manual                                                            | Sigue requiriendo integrar las regex, manejar prioridades, retroceder en errores                  |
| **Logos** (generador automático)      | Generación de DFA óptimo, prioridad de reglas declarativa, manejo de errores integrado | Dependencia externa, menos flexibilidad para casos muy complejos                                  |

Elegimos `logos` porque el balance beneficio-costo era claramente favorable. `logos` permite definir los tokens declarativamente como atributos de Rust, y el DFA generado es óptimo.

**¿Hubo casos donde `logos` fue insuficiente?** Sí, principalmente en la priorización de reglas. `logos` usa el orden de definición para desempatar cuando dos reglas coinciden. Tuvimos que experimentar con diferentes órdenes hasta conseguir el comportamiento deseado (por ejemplo, que `true` se reconozca como booleano y no como identificador). En un lexer manual esto se controla explícitamente; en `logos` se controla por orden, lo que puede ser confuso al principio.

### 3.3 Manejo de errores léxicos

El lexer debe reportar errores con posición precisa. `logos` facilita esto porque cada token lleva información de posición (offset, línea, columna). Sin embargo, hay un detalle sutil: cuando el lexer encuentra un carácter no reconocido, debe decidir si produce un error y continúa, o si aborta. Nuestra implementación opta por **producir un token `Unknown` y continuar**, porque queremos reportar **todos** los errores léxicos en una sola pasada, no detenernos en el primero.

### 3.4 Strings y secuencias de escape

El manejo de strings ilustra bien la tensión entre simplicidad y corrección. Los strings en HULK se delimitan con comillas dobles y soportan secuencias de escape (`\"`, `\n`, `\t`). El lexer realiza el unescaping directamente: la función `unescape_string_contents` procesa las secuencias de escape y produce un `TokenKind::String(value)` con el contenido ya transformado. Si una secuencia de escape es inválida, el lexer reporta un error léxico y produce un token `Unknown`, permitiendo la recuperación de errores.

**¿Por qué hacerlo en el lexer?** Porque el unescaping requiere validación (rechazar secuencias inválidas) y transformación (convertir `\n` en un salto de línea real). Al hacerlo en el lexer, el parser recibe strings ya procesados y no necesita lógica adicional de transformación. Además, el lexer puede reportar errores de escape con la posición exacta del carácter problemático usando la información de span que ya maneja.

---

## 4. Análisis Sintáctico (Parser)

### 4.1 El problema del análisis sintáctico

El parser toma la secuencia de tokens del lexer y construye un Árbol de Sintaxis Abstracta (AST). La dificultad radica en que la gramática de un lenguaje como HULK es inherentemente ambigua: la expresión `a + b * c` podría interpretarse como `(a + b) * c` o `a + (b * c)`. El parser debe resolver estas ambigüedades de acuerdo con las reglas de precedencia y asociatividad del lenguaje.

### 4.2 ¿Por qué LALRPOP y no otro enfoque?

Evaluamos tres alternativas principales para el parser:

**1. Recursive Descent manual.** Es el enfoque más común en compiladores académicos porque es fácil de entender e implementar. Sin embargo, tiene problemas serios con la gestión de precedencia y asociatividad: para implementar correctamente las reglas de precedencia en un recursive descent, hay que escribir una función por cada nivel de precedencia, y manejar la asociatividad izquierda requiere bucles explícitos. El código resultante es extenso y frágil.

**2. Combinators (Nom/Combine).** Permiten escribir parsers en un estilo funcional cercano a la gramática. Son populares en la comunidad Rust. Sin embargo, operan con parsing PEG, que no es LR. Esto significa que pueden tener ambigüedades difíciles de detectar (el operador `|` en PEG es ordered choice, no unión no determinista). Además, la generación de mensajes de error es pobre.

**3. LALRPOP (LR(1) automático).** Elegimos esta opción. La ventaja decisiva es que LALRPOP maneja automáticamente la precedencia, asociatividad y los conflictos LR. El diseñador del lenguaje especifica la gramática de forma declarativa y el generador produce un parser determinista.

**La desventaja de LALRPOP** es que los mensajes de error del generador son crípticos. Cuando hay un conflicto shift/reduce o reduce/reduce, LALRPOP produce trazas difíciles de interpretar. Esto alargó la depuración de la gramática. También descubrimos que LALRPOP no maneja bien ciertos patrones de recursión indirecta, lo que nos obligó a refactorizar partes de la gramática.

### 4.3 Precedencia y asociatividad: un diseño estratificado

La gramática de HULK organiza las expresiones en niveles de precedencia mediante estratificación de reglas. Pero a diferencia de un diseño clásico donde toda expresión sigue una única cadena lineal, HULK divide las expresiones en dos caminos: **cadena cerrada** (`BaseExpr`) y **cadena abierta** (`ExtExpr`).

**Cadena cerrada** — expresiones puramente binarias sin cola de control de flujo:

```
BaseExpr → Assignment → LogicalOr → LogicalAnd → Equality → AsExpr → Comparison → Term → Factor → Unary → Power → Primary
```

**Cadena abierta** — expresiones binarias cuyo último operando es un constructo de control de flujo (`FlowAtom`):

```
ExtExpr → OrTail → AndTail → EqTail → AsTail → CmpTail → AddTail → MulTail → UnaryTail → PowTail → FlowAtom
```

Donde `FlowAtom = IfExpr | WhileExpr | ForExpr | LetIn`.

Cada nivel `*Tail` toma la versión cerrada como operando izquierdo y la siguiente nivel `*Tail` como operando derecho. Por ejemplo:

```
AddTail → Term "+" MulTail → Factor "*" UnaryTail → PowerTail → FlowAtom
```

**Un ejemplo concreto**: `5 + if (true) 3 else 10`. El parser intenta `BaseExpr` primero, parsea `5` como `Term`, ve `+` y espera un `Factor` cerrado — pero `IfExpr` no está en `Primary`. Entonces intenta `ExtExpr → AddTail → Term "+" MulTail → ... → FlowAtom → IfExpr`. El `if` se parsea como `FlowAtom`, y la expresión completa es `5 + (if (true) 3 else 10) = 8`.

Este diseño permite que expresiones de control de flujo aparezcan como operandos de cualquier operador binario sin necesidad de paréntesis: `x + if (c) 1 else 0`, `y * while (cond) { body }`, etc.

**La asociatividad** se controla mediante la estructura de recursión. Los operadores izquierdo-asociativos (`+`, `-`, `*`, `/`, `%`, `@`, `@@`) usan recursión izquierda: `Term: Term "+" Factor`. Los operadores derecho-asociativos (`^`) usan recursión derecha: `Power: Primary "^" Unary`. LALRPOP maneja la recursión izquierda directamente, lo cual es una ventaja frente a generadores PEG que requieren transformación.

**El operador `:=`** (asignación destructiva) acepta una `Expr` completa como lado derecho, no solo una expresión cerrada. Esto permite `evens := evens + if (i % 2 == 0) 1 else 0` sin necesidad de paréntesis.

### 4.4 Hoisting en el parser

HULK permite que las declaraciones de tipos, interfaces y funciones aparezcan en cualquier orden. Esto es posible porque la gramática separa explícitamente las declaraciones de las sentencias:

```
Program = Declaration* Statements EOF
```

Todas las `Declaration` (tipos, interfaces, funciones) se recolectan antes que las `Statements`. Esto significa que el parser produce un AST donde las declaraciones están en listas separadas, y el análisis semántico puede procesarlas en el orden que necesite, independientemente del orden textual.

**¿Es esta la única forma de implementar hoisting?** No. Otra alternativa sería parsear el programa completo en una sola lista y luego hacer una pasada de reordenamiento sobre el AST. Elegimos la separación en la gramática porque es más limpia: la propiedad de "hoisting" queda explícita en la estructura del AST, no implícita en una transformación posterior.

### 4.5 El bucle `for`: del AST al código generado

El bucle `for` de HULK ha evolucionado desde el diseño original. Inicialmente se desazucaraba completamente durante el parseo (transformándose en `while` + `let-in` en la fase de parsing). En la implementación actual, el `for` es una expresión de primera clase en el AST, representada por `ForExpr`, y el desazúcar se realiza en la generación de código (codegen).

**¿Por qué cambiar el diseño original?** La versión inicial (desazucar en el parser) tenía una limitación: no podía distinguir entre tipos `Iterable` (con `next()` y `current()` directos) y tipos `Enumerable` (con un método `iter()` que retorna un iterador separado). Al desazucar en el parser, el `for` siempre asumía que el objeto de iteración era directamente el iterador. Con la introducción de la interfaz `Enumerable`, el compilador necesita decidir en codegen si llamar a `iter()` antes de iterar.

**El flujo actual es:**

1. **Parser**: `for (x in expr) { body }` se parsea como `ForExpr { id: "x", iter: expr, body }`.
2. **Análisis semántico**: `check_for_expr` determina el tipo del elemento verificando primero si el tipo tiene `current()` (Iterable) y luego si tiene `iter()` (Enumerable).
3. **Codegen**: `emit_for_expr` decide si llamar a `iter()` o usar el objeto directamente, y luego genera el código equivalente a:

```
let __hulk_iter__ = (si tiene iter: expr.iter() sino expr) in
  while (__hulk_iter__.next())
    let x = __hulk_iter__.current() in { body }
```

**¿Por qué esta arquitectura?** Separar el desazúcar en codegen permite que el semántico pueda verificar ambos protocolos sin generar código intermedio. Y mantener el `ForExpr` en el AST facilita futuras optimizaciones (como desenrollado de bucles o transformaciones específicas para `for`).

El nombre `__hulk_iter__` se elige con doble guion bajo porque HULK no permite que las variables del usuario comiencen con `_`. Esto garantiza que no puede haber colisión con ninguna variable escrita por el programador.

---

## 5. Análisis Semántico

### 5.1 ¿Qué hace el análisis semántico?

El análisis semántico verifica que el programa sea **significativo** más allá de su estructura sintáctica. Se asegura de que:

- Las variables estén declaradas antes de usarse.
- Los tipos sean compatibles en asignaciones y llamadas a funciones.
- Los métodos existan en los objetos sobre los que se invocan.
- Las interfaces estén correctamente implementadas.
- No haya ciclos de herencia.

Es, con diferencia, la fase más compleja del compilador. También es la que más decisiones de diseño concentra.

### 5.2 La arquitectura de dos pasadas: ¿por qué no basta una?

El análisis semántico se organiza en dos pasadas principales: **Collect** (recolección de símbolos) y **Check** (verificación de tipos). ¿Por qué no hacer todo en una sola pasada?

La razón es que dos características de HULK hacen imposible el análisis en un solo recorrido:

1. **Hoisting**: Una función `f` puede llamar a `g` aunque `g` se declare después textualmente. Si verificáramos los cuerpos de las funciones apenas las viéramos, al llegar a `f`, `g` aún no estaría registrada y el compilador reportaría un error de símbolo no encontrado. La solución es registrar primero **todas** las funciones (sin verificar sus cuerpos) y luego verificar los cuerpos en una segunda pasada.
2. **Inferencia de tipos**: Para inferir el tipo de retorno de una función `f` que llama a `g`, necesitamos conocer el tipo de retorno de `g`. Pero si `g` también llama a `f`, ninguna puede verificarse antes que la otra. La solución es inferir iterativamente.

Estos dos problemas —hoisting e inferencia mutuamente recursiva— fuerzan la arquitectura de dos pasadas.

**Alternativas consideradas**:

- **Single-pass con forward declarations**: El programador declara los prototipos antes de usarlos. Esto es lo que hace C. Lo descartamos porque va contra la filosofía de HULK de minimizar la ceremonia.
- **Single-pass con análisis de grafos**: Se construye el grafo de dependencias entre funciones y se ordenan topológicamente. No funciona con dependencias cíclicas (funciones mutuamente recursivas). También descartado.
- **Two-pass (elegido)**: La primera pasada registra todo; la segunda verifica. Simple, efectivo, y además se alinea con la inferencia iterativa.

### 5.3 La pasada de recolección de símbolos (Collect)

La primera pasada recorre las declaraciones del programa y construye las tablas de símbolos. No verifica los cuerpos de las funciones ni las expresiones.

**¿Qué registra exactamente?** Cinco categorías de símbolos:

1. **Tipos** (`type`): Se registra el nombre, la relación de herencia, y los parámetros del constructor. Se detectan ciclos de herencia recorriendo la cadena de padres en busca del hijo.
2. **Interfaces** (`interface`): Similar a tipos, pero se valida que una interfaz solo pueda extender otras interfaces (no tipos concretos). Esto es intencional: las interfaces definen comportamiento, no estructura.
3. **Funciones globales**: Se registran con nombres de parámetros y, si existen, anotaciones de tipo. Sin anotación, se registran como `Unknown`.
4. **Métodos de tipos**: Se registran con su firma y se valida que no haya sobrescritura con firma incompatible con el padre.
5. **Métodos de interfaces**: Similar a métodos de tipos, pero con la validación adicional de que todos los parámetros tengan anotaciones de tipo explícitas (una interfaz no puede tener métodos con tipos inferidos, porque la interfaz es el contrato público).

**¿Qué pasa si hay errores en esta fase (ej: tipo redeclarado)?**
La estrategia es: registrar el error pero continuar la recolección. Queremos reportar todos los errores de una sola vez, no detenernos en el primero. Sin embargo, hay que tener cuidado: si se registra un tipo con errores, las fases posteriores pueden generar errores deriva. Por eso, cuando un tipo tiene un error grave (ej: herencia circular), se marca como no disponible para que las fases posteriores no lo usen.

### 5.4 Verificación del bucle `for`: Iterable vs Enumerable

Una de las áreas más interesantes del análisis semántico es la verificación del bucle `for`. El compilador debe determinar si el tipo sobre el que se itera es válido para la iteración.

El algoritmo en `check_for_expr` (for_expr_checker.rs) resuelve el tipo del elemento siguiendo una estrategia en dos pasos:

1. **Busca `current()` directamente**: Si el tipo tiene un método `current()`, es un tipo `Iterable` directo (como `Range`). El tipo del elemento es el tipo de retorno de `current()`.
2. **Busca `iter()` como fallback**: Si el tipo no tiene `current()`, busca un método `iter()` que retorne un `Iterable`. Si lo encuentra, sigue la cadena: obtiene el tipo retornado por `iter()`, y luego busca `current()` en ese tipo retornado. El tipo del elemento es el tipo de retorno de `current()` en el iterador.
3. **Error**: Si no se encuentra ninguno de los dos protocolos, se reporta: "Type X is not iterable or enumerable."

Esta estrategia en cascada es clave porque permite que dos tipos de objetos funcionen en el `for`:

- **Tipos `Iterable`**: Tienen `next()` y `current()` directamente. El tipo en sí es el iterador.
- **Tipos `Enumerable`**: Tienen `iter()` que retorna un iterador separado. El tipo delega la iteración a un objeto distinto.

La verificación no se limita a comprobar la existencia de los métodos, sino que también valida sus firmas:

1. `current()` debe tomar **cero parámetros** y retornar el tipo del elemento.
2. `next()` debe tomar **cero parámetros** y retornar `Boolean`.
3. `iter()` debe tomar **cero parámetros** y retornar un tipo que a su vez tenga `next()` y `current()`.

Si un método existe pero tiene una firma incorrecta (por ejemplo, `next(x: Number)` o `current() => 42` con retorno `Number`), se reporta un error descriptivo que indica exactamente qué falla.

Además, la resolución de métodos **recorre la cadena de herencia** mediante `lookup_method_in_hierarchy`. Si un tipo `Iterable_Number` extiende `Iterable` y no redeclara `next()`, el compilador busca `next()` en el padre `Iterable`. Esto permite que las interfaces auto-generadas por la notación splat (`T*`) funcionen correctamente sin redeclarar todos los métodos.

La separación entre `Iterable` y `Enumerable` refleja un diseño bien fundamentado: un tipo `Enumerable` puede crear múltiples iteradores independientes (cada llamada a `iter()` crea un nuevo iterador con su propio estado), mientras que un `Iterable` tiene un solo estado de iteración. Esto es análogo a la diferencia entre `Iterator` e `IntoIterator` en Rust, o entre `Iterable` y `Collection` en Java.

### 5.5 Inferencia de tipos: el algoritmo iterativo

La inferencia de tipos en HULK no es Hindley-Milner puro. En lugar de usar un algoritmo de unificación global (como el algoritmo W), hemos optado por un enfoque **iterativo** que ejecuta el TypeChecker completo hasta 8 veces, deteniéndose cuando las firmas de funciones convergen.

**¿Por qué este enfoque y no Hindley-Milner?**

1. **Simplicidad**: Hindley-Milner requiere construir y resolver un sistema de ecuaciones de tipos. Nuestro enfoque iterativo reutiliza el TypeChecker que ya tenemos, añadiendo solo la lógica de `merge_types` para actualizar firmas.
2. **Inferencia bidireccional natural**: Nuestro sistema propaga tipos tanto hacia arriba (desde las expresiones hacia los contextos) como hacia abajo (desde los contextos esperados hacia las expresiones). Esto es más flexible que HM puro y permite, por ejemplo, que el tipo de retorno esperado de una función influya en los tipos de sus parámetros.
3. **Anotaciones opcionales**: La combinación de tipos anotados explícitamente y tipos inferidos es más fácil de manejar con un enfoque iterativo que con HM, donde las anotaciones interfieren con el proceso de unificación.

**Desventajas del enfoque iterativo**:

1. **Ineficiencia**: Ejecutar el TypeChecker hasta 8 veces sobre el mismo programa es costoso. Para programas pequeños no se nota, pero para programas grandes podría ser un problema.
2. **Límite arbitrario**: El máximo es de 8 iteraciones. En teoría podría no ser suficiente para algunos programas con cadenas de dependencias muy largas.
3. **No exhaustivo**: A diferencia de HM, que siempre encuentra el tipo principal (most general type), nuestro algoritmo puede dejar tipos como `Unknown` si no logra inferirlos.

Sin embargo, para el alcance académico del proyecto, el enfoque iterativo es más que suficiente. La mayoría de los programas de prueba convergen en 2 o 3 iteraciones.

### 5.6 El corazón de la unificación: `merge_types`

La función `merge_types` es el núcleo de la inferencia. Toma dos tipos —el tipo actual de un símbolo (posiblemente `Unknown`) y el tipo inferido desde el contexto— y produce un tipo unificado, o un error si son incompatibles.

```
merge_types(current, inferred):
  - Si current es Unknown → el tipo es inferred (se aprende)
  - Si inferred es Unknown → el tipo es current (no hay nueva información)
  - Si son iguales → ese tipo
  - Si uno es Null y el otro es nulable → el tipo nulable
  - En cualquier otro caso → error
```

La regla de nulabilidad merece explicación. En HULK, `Null` puede asignarse a tipos que son punteros o referencias: `String`, `Struct`, `Function`. No puede asignarse a `Number`, `Boolean` o `Unit`. Esto refleja una decisión de diseño: en lugar de hacer que todos los tipos sean nulables , HULK permite nulabilidad solo para tipos que naturalmente son punteros. Esto evita la necesidad de unwrapping explícito y reduce los errores por null pointer.

### 5.7 Varianza en interfaces: ¿por qué es necesaria?

Cuando un tipo implementa una interfaz, las firmas de los métodos no tienen que coincidir exactamente. El sistema de varianza define las reglas de compatibilidad:

- **Covarianza en retorno**: Si la interfaz declara `walk(): Animal`, el tipo puede implementar `walk(): Dog` (Dog es subtipo de Animal). Esto es seguro porque quien espera un `Animal` puede recibir un `Dog` (todo `Dog` es un `Animal`).
- **Contravarianza en parámetros**: Si la interfaz declara `feed(animal: Dog)`, el tipo podría implementar `feed(animal: Animal)`. Esto parece contraintuitivo, pero es correcto: quien llama a `feed` pasando un `Dog` está pasando un `Animal`, y el método espera `Animal`. Sin embargo, en nuestra implementación actual, la contravarianza se valida pero rara vez se usa en la práctica, porque la mayoría de interfaces y tipos usan los mismos tipos en los parámetros.

**¿Por qué no exigir igualdad exacta?** La razón es el **principio de sustitución de Liskov**: si un `DogWalker` es un `Walker`, debe poder usarse en cualquier contexto donde se espere un `Walker`. Sin varianza, esto se rompe: un `DogWalker` que retorna `Dog` no podría ser un `Walker` que retorna `Animal`, aunque todo `Dog` es un `Animal`.

**¿Qué alternativas existen?**

- **Invarianza (Java antes de 5)**: Las firmas deben coincidir exactamente. Más simple pero menos flexible. Java introdujo wildcards (`? extends T`, `? super T`) precisamente para solucionar esta limitación.
- **Declaración de varianza (C#)**: El diseñador de la interfaz declara si un parámetro es `in` (contravariante) o `out` (covariante). Más explícito pero más verboso.

Nuestra implementación usa varianza estructural automática, que es la opción más flexible. Esto significa que un tipo que "casi" implementa una interfaz (pero con tipos más específicos en retornos o más generales en parámetros) puede ser aceptado sin modificaciones.

### 5.8 El sistema de scopes

El manejo de scopes (ámbitos) es más interesante de lo que parece a simple vista. Usamos una pila de `HashMap<String, SemanticType>`, donde cada `push()` crea un nuevo ámbito y `pop()` lo destruye.

**La decisión clave fue: ¿cómo manejar las asignaciones?** En HULK, tanto `=` como `:=` modifican una variable existente, pero con restricciones estrictas de tipos. Si una variable se declara como `let x = 5`, su tipo es `Number`. Luego `x := "hola"` **y** `x = "hola"` son ambos errores de tipos. El tipo de una variable se fija en su declaración y ninguna forma de asignación puede cambiarlo.

La diferencia entre `=` y `:=` radica en otros aspectos:

- **`=` (asignación regular)**: Permite asignar a variables ya declaradas en cualquier scope accesible. No puede declarar variables nuevas.
- **`:=` (asignación destructiva)**: Modifica variables dentro de bloques anidados (`let-in`, `while`, `for`). El tipo del nuevo valor debe ser compatible con el tipo original, verificado por `types_compatible()`.

Ambos verifican compatibilidad de tipos antes de permitir la asignación. Si el tipo existente no es `Unknown` (inferido) y el tipo del nuevo valor no es compatible, se reporta un error de tipo descriptivo.

Esto se implementa en `assign_in_scope` y `destructive_assign_expr_checker`: cuando se modifica una variable, se busca el scope original donde se declaró y se verifica que el tipo del nuevo valor sea compatible con el tipo original mediante `types_compatible()`. Si no lo es, se reporta un error de tipo.

**Alternativa considerada**: Permitir que `:=` cambie el tipo de la variable (tipado como en Python). Esto habría complicado el sistema de tipos y debilitado las garantías estáticas. Lo descartamos porque HULK es un lenguaje de tipado estático.

---

## 6. Generación de Código

### 6.1 El desafío de generar código máquina

La generación de código es la fase que transforma el AST verificado en instrucciones ejecutables. Es un salto cualitativo: pasamos de una representación de alto nivel (árbol con tipos) a instrucciones de muy bajo nivel (carga, almacenamiento, saltos). La dificultad principal es que el AST es un grafo de expresiones anidadas, mientras que el código máquina es una secuencia lineal de instrucciones. Todo compilador debe resolver esta "linealización" del AST.

### 6.2 ¿Por qué LLVM y no código máquina directamente?

Podríamos haber generado código x86-64 directamente, como se hace en muchos compiladores académicos. ¿Por qué usar LLVM?

1. **Portabilidad**: LLVM IR puede compilarse a x86, ARM, RISC-V, etc. Nosotros solo probamos en x86-64, pero el mismo IR funciona en otras arquitecturas.
2. **SSA (Static Single Assignment)**: LLVM IR está en forma SSA, donde cada variable se asigna exactamente una vez. Esto simplifica muchas optimizaciones y el análisis de flujo de datos. Implementar y mantener una representación SSA propia es complejo. Al utilizar LLVM IR, esta representación forma parte natural de la infraestructura proporcionada por LLVM.
3. **Optimizaciones**:LLVM proporciona una amplia colección de optimizaciones (propagación de constantes, eliminación de código muerto, simplificación de expresiones, entre otras). Nuestro compilador genera IR relativamente simple y delega estas optimizaciones a LLVM.
4. **Validación**: LLVM incluye mecanismos de verificación que detectan errores de tipos, inconsistencias en SSA y otras violaciones de las restricciones del IR. Esto nos sirvió como verificación adicional de que nuestro IR era correcto.

**¿Por qué no usar `inkwell` (bindings oficiales de LLVM para Rust)?**

`inkwell` proporciona bindings seguros a la API de C de LLVM, permitiendo construir el IR programáticamente en lugar de generarlo como texto. Evaluamos esta opción y la descartamos por dos razones:

- **Complejidad de instalación**: LLVM debe estar instalado en el sistema y las versiones deben coincidir exactamente con las que espera `inkwell`. Esto es problemático en entornos educativos con diferentes sistemas operativos.
- **Curva de aprendizaje**: La API de LLVM es enorme y compleja. Generar IR como texto es más sencillo de depurar: el desarrollador puede leer el IR generado y entender qué está mal.

La desventaja de generar texto es que perdemos la validación en tiempo de compilación del IR. Un error en el texto solo se detecta cuando `clang` intenta compilarlo. Esto significa que los errores de generación de código son más difíciles de depurar.

### 6.3 Layout de objetos en memoria

Cuando el programador escribe `new Point(3, 4)`, el compilador debe decidir cómo se representa ese objeto en memoria. Nuestra decisión fue usar un layout con **etiqueta de tipo al inicio**:

```
[type_id (i64)] [campos del padre...] [campos propios...]
```

Cada objeto es un bloque de memoria contiguo asignado con `malloc`. El primer campo es un entero de 64 bits que identifica el tipo concreto del objeto. Luego vienen los campos, ordenados por herencia: primero los del padre, luego los del hijo.

**¿Por qué este layout y no otro?**

Las alternativas consideradas fueron:

1. **Etiqueta al inicio (elegido)**: Simple, el dispatch dinámico puede leer el type_id con una sola indirección (`load i64`). El field access requiere calcular offsets, pero es rápido.
2. **Estructuras tipadas de LLVM** :LLVM permite definir estructuras como `%Point = type { double, double }` y acceder a sus campos mediante `getelementptr`, delegando el cálculo exacto del layout al compilador. Sin embargo, nuestro diseño representa todos los objetos mediante punteros genéricos (`i8*`) para simplificar la generación de código y el manejo uniforme de la herencia. Como consecuencia, optamos por gestionar explícitamente el layout de los objetos en lugar de depender de estructuras LLVM tipadas.
3. **Tablas virtuales (vtables)**: Otra alternativa es almacenar en cada objeto un puntero a una tabla de métodos virtuales. Este enfoque, utilizado por muchos lenguajes orientados a objetos, permite implementar dispatch dinámico en tiempo constante. Sin embargo, requiere estructuras adicionales en tiempo de ejecución y una lógica más compleja para construir y mantener las tablas virtuales. Dado que nuestro lenguaje posee un modelo de objetos relativamente simple, consideramos que esta complejidad adicional no estaba justificada.

**¿Cómo se calculan los offsets?** Recursivamente, procesando primero el padre (que puede tener su propio padre) y luego los campos propios. El tamaño de cada campo depende de su tipo: `double` → 8 bytes, `bool` → 1 byte, punteros → 8 bytes. Aplicamos alineación natural (cada campo se alinea a su tamaño) para evitar penalizaciones de rendimiento.

### 6.4 Dispatch dinámico: el problema de los métodos virtuales

Cuando se llama a un método a través de una interfaz (ej: `walker.walk()`), el compilador no sabe en tiempo de compilación qué tipo concreto es `walker`. Podría ser un `DogWalker`, un `CatWalker`, o cualquier otro tipo que implemente `Walker`. La resolución debe ocurrir en tiempo de ejecución.

Nuestra solución es la **cascada por type tag**:

1. Extraer el `type_id` del objeto.
2. Comparar secuencialmente contra cada tipo concreto conocido.
3. Saltar a la implementación correspondiente.
4. Usar un nodo `phi` para fusionar el resultado.

**¿Es eficiente?** No. El dispatch es O(n) donde n es el número de tipos concretos que implementan la interfaz. Para programas pequeños (decenas de tipos) es aceptable, pero para cientos o miles sería un problema de rendimiento.

**¿Qué alternativas hay?**

1. **Tablas virtuales (vtables)**: Cada objeto tiene un puntero a una tabla de punteros a función. El dispatch es O(1): se indexa la vtable con un índice fijo (conocido en compilación) y se salta al método. Es la estrategia utilizada por C++, Java, C#, Swift, aunque cada uno emplea variantes propias de esta idea. Más eficiente pero más compleja:

   - Requiere generar vtables para cada tipo (o reutilizar las del padre).
   - Con herencia múltiple, hay que gestionar múltiples vtables (el problema del "thunk adjustment").
   - El layout del objeto es más complejo (puntero extra para la vtable).
2. **Monomorfización**: Se genera código diferente para cada combinación de tipos. Es la solución de C++ templates y Rust generics. Es la más eficiente en tiempo de ejecución (todo se resuelve en compilación), pero causa explosión de código y no soporta polimorfismo dinámico verdadero (no puedes tener una lista de `Walker`s de diferentes tipos concretos).

Elegimos la cascada por simplicidad de implementación. En un compilador académico, la claridad del código del compilador es más importante que la eficiencia del código generado. Además, la cascada es más fácil de entender y depurar que las vtables o la monomorfización.

**Una limitación importante**: Actualmente, la cascada no maneja correctamente la herencia de implementaciones. Si `DogWalker` extiende `WalkerBase`, y alguien llama a `walk()` a través de la interfaz en un `DogWalker` concreto, la cascada busca el método en `DogWalker`. Pero si `DogWalker` no implementa `walk()` (lo hereda de `WalkerBase`), la cascada no encontrará el método y llamará al stub de la interfaz, que probablemente no haga lo correcto. Esto es un bug conocido que requiere refactorización del sistema de dispatch.

### 6.5 Generación del bucle `for`: del AST a LLVM

El `for` se genera en `emit_for_expr` (codegen/llvm/backend/emit/expr/for_expr.rs). La lógica es la siguiente:

1. **Detectar el protocolo**: La función `has_iter_method` determina si el objeto tiene un método `iter()`. Para esto resuelve el tipo de la expresión mediante `resolve_expr_type_id`, que maneja variables, expresiones `new`, llamadas a métodos, llamadas a funciones y cualquier otra expresión evaluándola para obtener su tipo. Una vez obtenido el `type_id`, se busca `iter()` en la jerarquía de métodos. Si existe, se llama a `iter()` para obtener un iterador. Si no, el objeto es el iterador directamente.
2. **Generar el `while` equivalente**: El `for` se traduce a un `while` que llama a `next()` como condición, y dentro del cual se llama a `current()` para obtener el elemento actual.
3. **Crear el scope**: Se crea un scope con el iterador (`__hulk_iter__`) y el elemento (`x`).

La generación produce un patrón que puede visualizarse como:

```
; Si tiene iter():
%iter_obj = call i8* @hulk_typeN_iter(i8* %original_obj)

; Si NO tiene iter():
%iter_obj = %original_obj  (el objeto es su propio iterador)

; Luego el while:
while (%iter_obj.next()) {
    let x = %iter_obj.current() in { body }
}
```

El beneficio de desazucarar en codegen (y no en el parser) es que el compilador puede decidir en el último momento si necesita llamar a `iter()`, basándose en la información de tipos que ya tiene disponible.

### 6.6 Subtipado en tiempo de ejecución

Los operadores `is` y `as` requieren verificar la jerarquía de tipos en tiempo de ejecución. Para esto generamos:

1. Un arreglo global `@hulk_type_parents` que mapea cada TypeId a su padre.
2. Una función `hulk_is_subtype(child, parent)` que recorre la cadena de padres.

La función es O(d) donde d es la profundidad de la herencia. Como la herencia en HULK es simple (un solo padre), la profundidad máxima es el número de tipos en la cadena, que suele ser pequeño.

**Alternativa**: Una alternativa consiste en asignar identificadores que codifiquen explícitamente la posición de cada tipo dentro de la jerarquía. Por ejemplo, es posible numerar los nodos mediante recorridos del árbol y almacenar intervalos que permitan determinar relaciones de ancestro-descendiente en tiempo constante. Con este enfoque, la comprobación de subtipado podría realizarse en O(1). Sin embargo, estas representaciones requieren recalcular la numeración cuando cambia la estructura de la jerarquía y añaden complejidad adicional al compilador. Dado que el coste O(d) resulta suficientemente pequeño para los tamaños de programa considerados en este proyecto, optamos por la solución basada en la cadena de padres por su simplicidad y facilidad de implementación.

### 6.7 Jerarquía de valores en LLVM

Una decisión de diseño importante fue unificar todos los números como `double` (f64) en LLVM. Esto significa que `42` (entero) y `3.14` (flotante) se representan igual.

**Ventaja**: Simplicidad. No hay que generar conversiones entre i64 y f64 en operaciones aritméticas mixtas. El sistema de tipos semántico distingue entre literales enteros y flotantes, pero al bajar a LLVM se unifican.

**Desventaja**: Pérdida de precisión. Los enteros mayores a 2^53 no pueden representarse exactamente como f64. Además, las operaciones enteras (como el incremento de un contador) se hacen con aritmética de punto flotante, que es más lenta y sujeta a errores de redondeo.

**¿Por qué no usar i64 para enteros y f64 para flotantes?** Requeriría sobrecarga de operadores o coerciones implícitas, y complicaría el sistema de tipos. Decidimos que la simplicidad valía la pena para un compilador académico. En un compilador de producción, definitivamente usaríamos representaciones separadas.

---

## 7. Características Avanzadas del Lenguaje HULK

### 7.1 Interfaces Estructurales (Protocols)

#### 7.1.1 El problema de la reutilización de comportamiento

En todo lenguaje de programación orientado a objetos surge la necesidad de compartir comportamiento entre tipos que no están relacionados por herencia. Por ejemplo, tanto `Perro` como `Gato` pueden `caminar()`, pero no tiene sentido que `Perro` herede de `Gato` ni viceversa.

La solución clásica es la **interfaz**: un contrato que declara métodos sin implementarlos. Un tipo que firma el contrato (explícitamente con `implements` o implícitamente por estructura) puede usarse donde se espere la interfaz.

#### 7.1.2 La decisión fundamental: estructural vs nominal

La pregunta que enfrentamos fue: **¿la conformidad con una interfaz debe ser explícita (Java, C#) o implícita (Go, TypeScript)?**

**Sistema nominal** (Java, C#): Un tipo debe declarar `implements Interfaz` para satisfacerla. Ventajas:

- **Documentación explícita**: Queda claro que `DogWalker` fue diseñado para implementar `Walker`.
- **Intención del diseñador**: Si accidentalmente un tipo tiene métodos que coinciden con una interfaz pero no fue diseñado para ello, el sistema nominal evita el "acoplamiento accidental".
- **Compilación más simple**: La verificación es una comprobación de tablas (¿está `DogWalker` en la lista de implementadores de `Walker`?).

Desventajas:

- **Acoplamiento**: El tipo debe conocer las interfaces que implementa en el momento de su definición. No se puede añadir una interfaz a un tipo existente sin modificar el tipo.
- **Código boilerplate**: Declaraciones `implements` repetitivas.
- **Menor flexibilidad**: Bibliotecas externas no pueden implementar interfaces definidas por el usuario sin modificación.

**Sistema estructural** (HULK, Go): Un tipo satisface una interfaz si tiene los métodos requeridos con las firmas compatibles. Ventajas:

- **Desacoplamiento máximo**: El tipo no necesita conocer las interfaces. Se pueden definir interfaces después de haber escrito los tipos.
- **Polimorfismo accidental**: Si dos interfaces requieren el mismo método, un tipo las satisface a ambas.
- **Flexibilidad**: Cualquier tipo existente puede satisfacer interfaces nuevas sin modificación.

Desventajas:

- **Menos documentación**: No queda explícito qué interfaces satisface un tipo.
- **Posibles ambigüedades**: Si dos interfaces tienen métodos con el mismo nombre pero semántica diferente, el sistema no puede distinguirlas (aunque esto rara vez ocurre en la práctica).
- **Verificación más compleja**: El compilador debe buscar métodos con firma compatible en el tipo y en toda su cadena de herencia.

**Nuestra elección**: Estructural, por las siguientes razones:

1. **Filosofía del lenguaje**: HULK busca minimizar la ceremonia. El tipado es estático pero la sintaxis es ligera. Las interfaces estructurales encajan con esta filosofía: no hay que declarar explícitamente la conformidad.
2. **Flexibilidad en el desarrollo**: Los estudiantes pueden primero escribir los tipos concretos y luego definir interfaces que los agrupen, sin tener que modificar los tipos originales.
3. **Simplicidad conceptual**: El "duck typing" ("si camina como pato y suena como pato, entonces es un pato") es intuitivo y fácil de entender.

Sin embargo, somos conscientes de las desventajas. La más importante es la **falta de intencionalidad**: en un equipo grande de desarrollo, la declaración explícita `implements` es valiosa porque documenta la intención del diseñador. En HULK, un tipo puede satisfacer una interfaz por accidente, lo que puede llevar a comportamientos inesperados. Go, que usa el mismo enfoque, ha tenido que desarrollar herramientas externas para verificar que un tipo implementa una interfaz intencionalmente.

#### 7.1.3 Varianza: la pieza técnica más delicada

La varianza es el aspecto técnicamente más sutil de las interfaces estructurales. Determina cómo se relacionan los tipos de los métodos de la interfaz con los tipos de los métodos de la implementación.

Nuestra implementación permite:

- **Covarianza en retornos**: Si la interfaz retorna `Animal`, la implementación puede retornar `Dog` (un subtipo). Quien espera un `Animal` está contento recibiendo un `Dog`.
- **Contravarianza en parámetros**: Si la interfaz acepta `Dog`, la implementación puede aceptar `Animal` (un supertipo). El implementador debe ser **más tolerante** que la interfaz: debe aceptar al menos lo que la interfaz acepta, posiblemente más.

**¿Por qué es correcta la covarianza en retornos?** Porque quien espera un `Animal` está contento recibiendo un `Dog`. Todo `Dog` es un `Animal`, así que no hay sorpresas.

**¿Por qué es correcta la contravarianza en parámetros?** Porque si la interfaz dice "acepto `Dog`", el llamante pasa un `Dog`. Si la implementación acepta `Animal`, también acepta `Dog` (porque `Dog` es `Animal`). La intuición es que el implementador debe ser **más tolerante** que la interfaz: debe aceptar al menos lo que la interfaz acepta, posiblemente más.

**¿Qué pasa si no respetamos estas reglas?**

Si permitimos covarianza en parámetros (un `DogWalker` que acepta `Dog` cuando la interfaz acepta `Animal`), el llamante podría pasar un `Cat` (porque la interfaz acepta `Animal`), pero el `DogWalker` solo sabe manejar `Dog`. Esto rompería la seguridad de tipos en tiempo de ejecución.

Si permitimos contravarianza en retornos (un `DogWalker` que retorna `Animal` cuando la interfaz promete `Dog`), el llamante esperaría un `Dog` pero recibiría un `Animal` genérico, pudiendo llamar a métodos que `Animal` no tiene.

#### 7.1.4 Herencia de interfaces

Las interfaces pueden extender otras interfaces:

```
interface Runner extends Walker {
    run(): String;
}
```

Esto significa que un `Runner` debe tener todos los métodos de `Walker` más los suyos propios. La implementación de `Runner` también debe implementar `Walker`.

**¿Por qué herencia de interfaces y no composición?** Podríamos haber definido `Runner` independientemente y luego requerir que un tipo implemente ambas interfaces. Pero la herencia es más conveniente: permite que una función que espera un `Walker` acepte también un `Runner` (porque `Runner` extiende `Walker`).

#### 7.1.5 Limitaciones de la implementación actual

1. **No hay métodos con implementación por defecto** (como los default methods de Java). Esto significa que cada tipo que implementa una interfaz debe proporcionar todas las implementaciones, incluso si son idénticas para todos los tipos.
2. **No hay métodos estáticos en interfaces**. Todos los métodos de interfaz requieren un receptor.
3. **La verificación de conformidad es O(n*m)**: por cada interfaz y cada tipo, se recorren todos los métodos. Para programas pequeños no es problema, pero no escala.

---

### 7.2 Iteradores y Bucle `for`

#### 7.2.1 ¿Por qué dos protocolos: `Iterable` y `Enumerable`?

El bucle `for` en la mayoría de lenguajes está acoplado a un concepto de "iterable". En Python, cualquier objeto con `__iter__` y `__next__` es iterable. En Java, cualquier objeto que implemente `Iterable` puede usarse en un `for-each`. En Rust, cualquier tipo que implemente `IntoIterator` puede usarse en un `for`.

HULK adopta un diseño de **dos protocolos** que refleja una distinción fundamental en programación orientada a objetos:

- **`Iterable`**: Un tipo que **es** su propio iterador. Tiene `next(): Boolean` y `current(): Object` directamente. El tipo y el iterador son la misma cosa. Ejemplo: `Range`.
- **`Enumerable`**: Un tipo que **puede crear** iteradores independientes. Tiene `iter(): Iterable` que retorna un objeto iterador separado. Cada llamada a `iter()` crea un nuevo iterador con su propio estado. Ejemplo: una lista que puede ser recorrida múltiples veces.

**¿Por qué esta distinción?** Porque resuelve un problema real de diseño:

```
let r = range(1, 10);
for (x in r) { print(x); }  // Primera iteración
for (x in r) { print(x); }  // ¿Segunda iteración?
```

Si `Range` fuera `Enumerable`, la segunda iteración funcionaría porque `r.iter()` crearía un nuevo iterador. Pero como `Range` es `Iterable`, el segundo `for` intentaría reusar el mismo iterador, que ya está agotado. Esta distinción fuerza al programador a pensar en la semántica de su tipo: ¿es un generador de una sola pasada (Iterable) o una colección reutilizable (Enumerable)?

**¿Por qué dos métodos y no uno solo que devuelva Option/Maybe?** Porque HULK no tiene tipos Option. Podríamos haber añadido un tipo `Maybe` o hacer que `next()` devuelva el elemento o null. Pero la opción de dos métodos (uno para avanzar y otro para obtener el valor actual) es más simple y se alinea con el estilo de interfaces mínimo de HULK.

#### 7.2.2 El desazucarado y sus implicaciones

Como explicamos en la Sección 4.5, el `for` se desazucara en la generación de código (codegen) a `while` + `let-in`. Esto tiene implicaciones profundas en cómo se comporta el iterador.

**Problema: materialización completa vs generación incremental.** Consideremos:

```
for (x in range(1, 1000000)) {
    // procesar x
}
```

Una implementación ingenua podría generar primero una colección con todos los números del rango y luego iterar sobre ella. En ese caso, el programa necesitaría memoria proporcional al tamaño del rango antes de comenzar a procesar los elementos.

HULK adopta una estrategia diferente: los elementos se generan de forma incremental conforme el bucle los solicita. El tipo `Range` únicamente mantiene el estado necesario para producir el siguiente valor:

```
type Range(min: Number, max: Number) {
    min = min;
    max = max;
    current = min - 1;
    next(): Boolean => { self.current := self.current + 1; self.current < self.max; };
    current(): Number => self.current;
}
```

De esta forma, el consumo de memoria es constante , independientemente del tamaño del rango. El iterador solo almacena unos pocos valores (`min`, `max` y `current`) y calcula cada elemento cuando el bucle avanza.

**¿Por qué generar los elementos bajo demanda?** Principalmente por eficiencia de memoria. El coste espacial pasa de O(n) a O(1), permitiendo trabajar con secuencias muy grandes sin necesidad de almacenarlas completamente en memoria.

**¿Es necesario que el iterador sea mutable?** Sí, porque tiene que recordar en qué posición está. Esto significa que no se puede compartir un iterador entre múltiples bucles concurrentes. Pero HULK no tiene concurrencia, así que no es un problema práctico.

#### 7.2.3 El protocolo `Enumerable`: iteración indirecta

El protocolo `Enumerable` resuelve un problema que `Iterable` no puede manejar: tipos que necesitan crear iteradores independientes para cada recorrido. La interfaz `Enumerable` tiene un solo método: `iter(): Iterable`.

**¿Qué resuelve esta separación?** Considere una lista:

```
type List<T> {
    // ...
    iter(): Iterable => new ListIterator(self);
}
```

Si `List` fuera `Iterable`, tendría un solo estado de iteración. Dos `for` loops sobre la misma lista interferirían entre sí. Con `Enumerable`, cada `for` llama a `iter()` que crea un `ListIterator` nuevo, con su propio puntero a la posición actual. Los dos bucles son completamente independientes.

**Analogía con otros lenguajes**:

- **Rust**: `Iterable` ≈ `Iterator` (un solo `next()`), `Enumerable` ≈ `IntoIterator` (tiene `into_iter()` que crea un iterador).
- **Java**: `Iterable` ≈ `Collection` (tiene `iterator()`), `Enumerable` ≈ `Iterable` (tiene `iterator()`).
- **Python**: `Iterable` ≈ `Iterator` (tiene `__next__`), `Enumerable` ≈ `Iterable` (tiene `__iter__`).

**La cadena de resolución en el compilador**: Cuando el compilador encuentra `for (x in expr)`, primero verifica si `expr` tiene `current()` (es `Iterable`). Si no, busca `iter()` (es `Enumerable`), obtiene el tipo retornado por `iter()`, y luego verifica que ese tipo retornado tenga `current()`. El tipo del elemento es el tipo de retorno de `current()` en el iterador.

Esta estrategia en cascada es clave porque permite que tanto `Range` (Iterable) como una lista (Enumerable) funcionen en el mismo `for`, sin que el programador tenga que escribir código diferente.

#### 7.2.4 Splat notation (`T*`): el problema de la especialización

La interfaz `Iterable` tiene `current(): Object`. Pero si escribimos:

```
function sum(numbers: Number*): Number {
    for (x in numbers) { total := total + x; }
}
```

El sistema de tipos debe saber que los elementos de `numbers` son `Number`, no `Object`. De lo contrario, no podría verificar que `total + x` es válido (la suma requiere dos números).

**Solución**: La notación `T*` (splat) no es un simple azúcar sintáctico. El compilador, durante la fase de `SymbolCollector`, escanea todas las anotaciones `T*` y genera automáticamente interfaces `Iterable_T` que extienden `Iterable` con `current(): T` sobrecargado:

```
interface Iterable_Number extends Iterable {
    current(): Number;  // Refina el tipo de retorno
}
```

**¿Por qué no usar genéricos? :**  La notación `T*` puede interpretarse como una forma limitada de especialización estática. En lugar de implementar una interfaz genérica `Iterable<T>`, el compilador genera una interfaz concreta para cada tipo utilizado en notación splat. Este enfoque es menos expresivo que los genéricos verdaderos, pero simplifica enormemente la implementación. No es necesario introducir variables de tipo, algoritmos de sustitución ni mecanismos de inferencia para parámetros genéricos.

**Limitación**: Si el programador escribe `Number*` en 10 lugares diferentes, se genera una sola interfaz `Iterable_Number`. Pero si escribe `Number*` y `String*`, se generan dos interfaces diferentes. Para tipos de usuario, se genera una interfaz por cada combinación de `T*` que aparezca en el programa. Esto podría llevar a muchas interfaces si el programa usa muchos tipos diferentes en notación splat.

**Alternativa considerada**: Implementar genéricos reales (`Iterable<T>`). Esto habría requerido:

1. Variables de tipo en el sistema semántico (`SemanticType::TypeVar(u32)`).
2. Unificación con sustitución de variables.
3. Monomorfización en la generación de código.

El costo de implementación era demasiado alto para el alcance del proyecto. La splat notation es un compromiso práctico que cubre el caso de uso más común (iterar sobre tipos concretos) sin la complejidad de los genéricos.

#### 7.2.5 El tipo `Range` como built-in

`Range` es un tipo especial: no está escrito en HULK, sino que se construye programáticamente en Rust dentro del módulo `builtins.rs`. Creamos su declaración (`TypeDecl`) con los campos y métodos adecuados, y la inyectamos en el programa durante el análisis semántico.

**¿Por qué no permitir que `Range` se escriba en HULK?** Porque `Range` es esencial para el bucle `for`. Si `Range` fuera un tipo de biblioteca, tendría que estar disponible en todos los programas. Podríamos haberlo incluido como un archivo de biblioteca estándar que se importa automáticamente, pero HULK no tiene sistema de módulos. La solución más práctica fue hacerlo built-in.

---

### 7.3 Inferencia de Tipos

#### 7.3.1 La tensión entre inferencia y anotaciones

Uno de los debates clásicos en diseño de lenguajes es: **¿cuánta información de tipos debe escribir el programador?**

En un extremo están los lenguajes con tipado completamente explícito (Java, C++ antiguo, donde hay que escribir el tipo de cada variable). En el otro extremo están los lenguajes con inferencia completa (ML, Haskell, donde rara vez se escribe un tipo).

HULK adopta una posición intermedia: la inferencia es la opción por defecto, pero se pueden añadir anotaciones opcionales cuando se desea documentar o restringir.

**¿Por qué no inferencia completa?** Porque la inferencia completa (tipo Hindley-Milner) tiene limitaciones bien conocidas:

- No soporta polimorfismo de subtipado bien.
- No soporta sobrecarga.
- Los mensajes de error son difíciles de entender.
- La implementación es compleja (algoritmos W o J).

**¿Por qué no tipo explícito siempre?** Porque va contra la filosofía de HULK de minimizar la ceremonia. El programador no debería tener que escribir `x: Number = 5` cuando `5` claramente es un `Number`.

#### 7.3.2 Propagación de tipos mediante fijación iterativa

El sistema de inferencia de HULK se basa en un proceso iterativo de propagación de tipos que combina información ascendente y descendente:

* **Propagación ascendente** : el tipo de una expresión se deduce a partir de sus subexpresiones.
* **Propagación descendente** : el contexto (por ejemplo, el tipo esperado de retorno de una función) restringe los tipos posibles de las subexpresiones.

Por ejemplo:

```
function add(x, y) => x + y;
```

La operación `+` fija que ambos operandos deben ser `Number`, lo que permite inferir que `add : (Number, Number) → Number`.

Cuando no existe suficiente información contextual, el sistema introduce un tipo temporal `Unknown`, que representa una variable aún no resuelta durante la inferencia

#### 7.3.3 Resolución iterativa y puntos de fijación

El algoritmo de inferencia se ejecuta mediante iteraciones sucesivas sobre el AST hasta alcanzar un punto fijo (fixpoint), en el cual no se producen más cambios en los tipos inferidos.

Consideremos:

```
function f(x) => g(x) + 1;
function g(y) => f(y) * 2;
```

Este ejemplo introduce dependencia mutua entre funciones. En este caso, el sistema requiere múltiples iteraciones para propagar información de tipos entre ambas definiciones.

El proceso no garantiza convergencia en todos los casos sin anotaciones adicionales, especialmente cuando existen ciclos sin información inicial suficiente.

En la práctica, el sistema converge rápidamente en programas típicos donde al menos una de las definiciones contiene información suficiente para iniciar la propagación.

#### 7.3.4 Ventajas y limitaciones de nuestro enfoque

**Ventajas**:

- **Simple**: Reutiliza el TypeChecker que ya existe.
- **Predecible**: El programador puede entender por qué el compilador infiere ciertos tipos.
- **Anotaciones opcionales**: Funciona tanto con como sin anotaciones.

**Limitaciones**:

- **No encuentra el tipo principal**: A diferencia de Hindley-Milner, nuestro algoritmo puede dejar tipos como `Unknown`.
- **No maneja polimorfismo**: No hay variables de tipo, así que funciones como `id(x) => x` no pueden ser polimórficas.
- **Límite arbitrario**: 8 pasadas, elegido por experiencia. Podría no ser suficiente.

---

### 7.4 Hoisting

#### 7.4.1 ¿Por qué es importante?

El hoisting es una característica de ergonomía: permite que las declaraciones aparezcan en cualquier orden. El programador puede organizar el código de la forma que le resulte más legible, sin preocuparse por dependencias de orden.

**Sin hoisting**:

```
type Perro { ... }  // Debe declararse antes
function usarPerro() => new Perro();  // Después
```

**Con hoisting**:

```
function usarPerro() => new Perro();  // Se usa antes
type Perro { ... }  // Se declara después
```

En lenguajes sin hoisting (C, Java, Pascal), el programador debe organizar el código en un orden específico, a menudo usando archivos de cabecera o prototipos. Esto es tedioso y propenso a errores.

#### 7.4.2 ¿Cómo se implementa?

Técnicamente, el hoisting requiere que el compilador **separe la recolección de nombres de la verificación de cuerpos**. Nuestra implementación hace esto en dos niveles:

1. **Parser**: Las declaraciones y las sentencias se recolectan por separado en el AST. El `Program` tiene listas separadas: `types`, `interfaces`, `functions` y `statements`.
2. **Análisis semántico**: `SymbolCollector` registra todos los nombres sin verificar cuerpos. Luego `TypeChecker` verifica los cuerpos con todos los nombres ya disponibles.

**¿Es esto caro?** Sí, requiere dos recorridos completos del AST. Pero es el precio de la flexibilidad. Sin hoisting, podríamos hacer todo en un solo recorrido.

#### 7.4.3 Alternativas

**Forward declarations** (C): El programador declara prototipos antes de usar las funciones. No es hoisting real; es una solución manual que requiere disciplina.

**Múltiples archivos con orden de compilación** (Java, C#): El compilador procesa los archivos en un orden que garantiza que las dependencias estén resueltas. No es aplicable a un lenguaje de un solo archivo.

**Análisis de dependencias** (Rust): El compilador analiza las dependencias entre items y las ordena automáticamente. Es más complejo que nuestro enfoque, pero permite single-pass.

Nuestro enfoque de dos pasadas es el más simple que funciona correctamente con hoisting completo.

---

### 7.5 Herencia de Tipos

#### 7.5.1 Decisiones de diseño

La herencia en HULK es simple (un solo padre) con sintaxis explícita `inherits`. Las decisiones clave fueron:

**1. Herencia simple, no múltiple**. La herencia múltiple (como en C++) introduce problemas graves: el "diamante" (dónde dos padres heredan del mismo abuelo), ambigüedad en nombres de métodos y campos, y complejidad en el layout de objetos. Para compartir comportamiento entre tipos no relacionados, HULK usa interfaces (que pueden ser múltiples).

**2. Atributos privados por defecto**. Solo los métodos del propio tipo pueden acceder a `self.x`. Esto es más restrictivo que lenguajes como C++ (donde se puede declarar `protected`) pero es más simple y fomenta el encapsulamiento. Si un subtipo necesita acceder al estado del padre, el padre debe proveer métodos públicos.

**3. Métodos públicos y virtuales por defecto**. Un método puede ser sobrescrito por subtipos. No hay forma de declarar un método como "final" o "no virtual". Esto es menos flexible que C++ (`virtual` explícito) o Java (`final`), pero simplifica el lenguaje.

**4. Constructor delegado**. El hijo especifica cómo inicializar al padre: `inherits Point(x, y)`. Esto es más explícito que la herencia de constructores de Java (donde se llama a `super(x, y)` dentro del constructor), y evita la ambigüedad de qué argumentos pasar al padre.

#### 7.5.2 El layout de objetos con herencia

Cuando un tipo hereda de otro, los campos del padre se incluyen antes que los del hijo en el layout de memoria. Esto es esencial para que el subtipado funcione correctamente: un puntero a un hijo puede tratarse como puntero al padre porque el layout del padre está al inicio.

**Ejemplo**: `Point3D inherits Point`:

```
Offset 0:  type_id (i64)           ← Point3D = 5
Offset 8:  self.x (double)         ← de Point
Offset 16: self.y (double)         ← de Point
Offset 24: self.z (double)         ← de Point3D
```

Si tratamos un `Point3D*` como `Point*`, el código que accede a `self.x` en offset 8 sigue funcionando porque `x` está en la misma posición relativa.

**Problema**: Este layout no funciona con herencia múltiple. Si Point3D heredara de Point y de Color, no podríamos tener ambos padres al offset 0. La herencia múltiple requiere "ajuste de punteros" (pointer adjustment), donde el puntero se desplaza según el padre desde el que se accede. Esto es otra razón por la que elegimos herencia simple.

#### 7.5.3 Self y llamadas a métodos heredados

Cuando un método del hijo sobrescribe un método del padre, el método del padre puede seguir siendo útil. En muchos lenguajes, se puede llamar al método del padre con `super.metodo()`. HULK no tiene `super`, lo que es una limitación conocida. El programador no puede reutilizar la implementación del padre y extenderla; debe reimplementar todo.

**¿Por qué no implementar `super`?** Requiere que el compilador distinga entre llamadas a métodos de instancia y llamadas a métodos de la superclase. La generación de código tendría que saltarse el dispatch dinámico y llamar directamente al método del padre. No es técnicamente difícil, pero no lo consideramos prioritario.

---

### 7.6 Otras Características

#### 7.6.1 Operadores `is` y `as`

`is` verifica si un objeto es de un tipo determinado en tiempo de ejecución. `as` realiza una conversión segura: si el objeto es del tipo destino, retorna el objeto convertido; si no, retorna `null`.

**Implementación**: Ambos usan la función `hulk_is_subtype` en LLVM IR. `is` simplemente compara el `type_id` del objeto contra el tipo consultado. `as` hace la misma verificación y utiliza un `phi` node en LLVM IR para retornar el puntero original si el subtype check es verdadero, o `null` si es falso.

**¿Son estas operaciones seguras?** `is` es completamente segura (solo lee). `as` es segura porque retorna `null` si la conversión falla; el programador debe verificar el resultado antes de usarlo. Esto sigue el patrón de lenguajes como TypeScript o C#.

#### 7.6.2 Funciones Built-in y Constantes

Las funciones matemáticas (`sin`, `cos`, `sqrt`, `exp`) se traducen directamente a las funciones correspondientes de la biblioteca matemática de C. `log` toma dos argumentos (base, valor) y calcula `ln(valor) / ln(base)` usando la función `log` de C internamente. No realizamos ninguna comprobación de dominio (ej: `sqrt(-1)` produce NaN, no un error).

`print` usa `printf` con formatos especializados según el tipo:

- Números: `%g` (notación general de punto flotante).
- Strings: `%s`.
- Booleanos: `%d` (0/1). Los booleanos se imprimen como enteros 0 o 1, no como "true"/"false". Aunque se definieron constantes `@.bool.true` y `@.bool.false` en el IR, actualmente no se usan en la impresión.

`rand` usa la función `rand()` de C, normalizando el resultado al intervalo [0, 1).

**¿Por qué no construir una biblioteca estándar más grande?** Porque el proyecto se enfoca en el compilador, no en la biblioteca. Las funciones built-in son las mínimas necesarias para que el lenguaje sea usable.

---

## 8. Limitaciones del Proyecto

### 8.1 Limitaciones del Lenguaje

**break/continue**: No implementados. El programador no puede interrumpir un bucle desde dentro. Para salir de un bucle, debe usar una condición compuesta en el `while` o reestructurar el código.

**Impacto**: Obliga a escribir bucles con condiciones de salida complejas o a usar variables de estado adicionales.

**Pattern Matching**: No implementado. Aunque lenguajes como Rust o Haskell muestran lo poderoso que es el pattern matching, su implementación requiere compilar patrones a árboles de decisión, verificar cobertura exhaustiva, y manejar patrones anidados. Es una omisión importante pero justificable dentro del alcance académico.

**Genéricos (Type Parameters)**: No implementados. La splat notation (`T*`) es un sustituto parcial para el caso específico de iteradores, pero no hay una forma general de escribir `List<T>`, `Option<T>` o `Pair<A, B>`.

**Consecuencia**: El programador no puede escribir funciones polimórficas sobre tipos arbitrarios. Cada función debe trabajar con tipos concretos.

**Módulos**: No implementados. Todo el programa debe estar en un solo archivo. No hay forma de organizar el código en módulos reutilizables.

**Sobrecarga de Operadores**: No implementada. `+` siempre es suma numérica, no puede redefinirse para strings o tipos definidos por el usuario. La concatenación de strings usa `@`.

**Manejo de Errores**: No hay excepciones, ni tipo `Result`, ni `panic`. Si un programa falla en tiempo de ejecución (ej: división por cero, acceso a campo nulo), el comportamiento es indefinido (el sistema lo maneja mediante señal SIGFPE o segfault).

**Números unificados como f64**: Como se discutió, enteros grandes pierden precisión y las operaciones son más lentas.

### 8.2 Limitaciones de Implementación

**Optimización nula**: El compilador no realiza ninguna optimización propia. Todo el IR se genera de forma directa y se delega a `clang` para optimización. Esto significa que código como `let x = 2 + 3 in print(x)` generará IR con `fadd 2.0, 3.0` y luego `printf`, en lugar de plegar la constante a `5` en compilación.

**Dispatch O(n)**: La cascada de type_id no escala. Con 100 tipos concretos implementando una interfaz, cada llamada a método requiere hasta 100 comparaciones y saltos.

**Memory leaks**: Todos los objetos se asignan con `malloc` y nunca se liberan con `free`. No hay garbage collector ni ARC. Para programas pequeños esto es aceptable (el SO libera la memoria al terminar), pero para programas de larga duración sería un problema grave.

**Strings ineficientes**: La concatenación con `@` usa `asprintf`, que asigna memoria en cada operación. No hay string builder ni optimización de concatenaciones en cadena.

**Pruebas incompletas**: Aunque hay ~120 archivos de ejemplo en `examples/` y más de 100 pruebas unitarias en el código fuente, no cubren todos los casos borde (ej: herencia profunda, dispatch con herencia de implementaciones, muchos tipos implementando una interfaz, varianza con tipos diferentes).

## 9. Conclusiones y Trabajo Futuro

### 9.1 Logros del Proyecto

El compilador de HULK es un proyecto completo que demuestra:

1. **Comprensión del pipeline de compilación**: Desde el lexer hasta la generación de código LLVM, cada fase está implementada y funciona correctamente para los casos de prueba.
2. **Diseño de lenguaje**: HULK no es solo un ejercicio técnico; es un lenguaje con personalidad propia que toma decisiones de diseño explícitas (interfaces estructurales, herencia simple, inferencia iterativa, splat notation).
3. **Características avanzadas**: El sistema de interfaces estructurales con varianza, el protocolo `Iterable` con generación automática de interfaces, el dispatch dinámico por type tag, y la inferencia bidireccional son características que van más allá de lo esperado en un compilador académico típico.
4. **Calidad de implementación**: El código está organizado en módulos separados, tiene pruebas unitarias y de integración, y produce mensajes de error con posición precisa.

### 9.2 Lecciones Aprendidas

**Sobre la arquitectura**: La separación en dos pasadas (Collect + Check) fue una decisión acertada que simplificó el manejo del hoisting y la inferencia. El desazucarado del `for` se realizó inicialmente en el parser, pero luego fue migrado a codegen para soportar la distinción entre `Iterable` y `Enumerable`. Esta migración fue más trabajosa de lo esperado, pero valió la pena porque hizo que la arquitectura fuera más flexible y extensible.

**Sobre la generación de código**: Emitir LLVM IR como texto fue más accesible que usar `inkwell`, pero nos privó de validación temprana. Varios bugs se descubrieron solo cuando `clang` rechazaba el IR generado, y depurar esos errores era más difícil que si hubiéramos tenido bindings de tipos.

**Sobre las interfaces estructurales**: La implementación de varianza (covarianza en retornos, contravarianza en parámetros) fue la parte más desafiante del proyecto, pero también la más gratificante. Demostró que un lenguaje académico puede implementar características de lenguajes modernos sin una complejidad desmedida.

**Sobre la inferencia**: La inferencia iterativa, aunque limitada, fue mucho más fácil de implementar que Hindley-Milner y cubre la mayoría de los casos de uso de un lenguaje de este tamaño. La principal lección fue que la simplicidad de implementación es un criterio de diseño válido cuando el alcance del proyecto lo justifica.

### 9.3 Trabajo Futuro

#### Optimización del compilador

1. **Optimizaciones propias**: Implementar plegado de constantes, eliminación de código muerto, y propagación de copias a nivel de AST antes de generar IR. Por ejemplo, `let x = 5 in x + 3` debería producir directamente `8`, no `fadd 5, 3`.
2. **VTables**: Reemplazar la cascada de type tag por tablas virtuales. Cada tipo tendría una vtable con punteros a función, y el dispatch sería O(1). Esto requeriría añadir un puntero a la vtable en el layout de objetos, pero mejoraría drásticamente el rendimiento.
3. **Mejora del IR**: Generar IR que LLVM pueda optimizar mejor. Por ejemplo, usar `alloca` + `store` + `load` para variables es correcto pero impide muchas optimizaciones. Usar registros SSA directamente (con `phi` nodes) produciría código más optimizable.

#### Extensiones del lenguaje

4. **Genéricos**: Implementar parámetros de tipo con monomorfización. Permitiría escribir `List<T>`, `Option<T>`, etc., y funciones genéricas como `first<T>(list: T*): T`.
5. **break/continue**: Añadir etiquetas de salto en la generación de código. No es técnicamente difícil; simplemente no lo priorizamos.
6. **Pattern Matching**: Implementar match sobre tipos con patrones simples (desestructuración de objetos). Sería la característica más compleja pero también la más potente.
7. **Módulos**: Permitir que un programa se divida en varios archivos con importación explícita.
8. **super**: Añadir la palabra clave `super` para llamar a métodos del padre desde un método sobrescrito.

#### Mejora del sistema de tipos

9. **Hindley-Milner completo**: Reemplazar la inferencia iterativa por Hindley-Milner con algoritmo W. Esto daría inferencia completa y polimorfismo paramétrico verdadero.
10. **Tipos algebraicos**: Añadir tipos suma (enum) y producto (struct/tuple) con pattern matching.
11. **Tipos nulables explícitos**: En lugar de la nulabilidad implícita actual, usar `T?` para tipos nulables, como en Swift o TypeScript.

#### Runtime

12. **Garbage Collector**: Implementar un GC simple (mark-and-sweep o reference counting con cycle detection) para evitar memory leaks.
13. **String builder**: Optimizar la concatenación de strings usando un builder que evite asignaciones repetidas.
14. **Biblioteca estándar**: Implementar colecciones (List, Map, Set), algoritmos (sort, filter, map), y E/S.

---

## 10. Referencias y Bibliografía

### Frameworks y Bibliotecas

- **Logos 0.14**: Generador de lexer para Rust. Documentación en https://docs.rs/logos/
- **LALRPOP 0.22**: Generador de parser LR(1) para Rust. Documentación en https://lalrpop.github.io/lalrpop/
- **egui/eframe**: Framework GUI inmediato para Rust. https://github.com/emilk/egui
- **LLVM Language Reference**: Documentación oficial del IR. https://llvm.org/docs/LangRef.html

### Teoría de Compiladores

- **Aho, Lam, Sethi, Ullman**: "Compilers: Principles, Techniques, and Tools" (2nd ed.). El "Dragon Book". Referencia fundamental para el diseño del pipeline.

### Lenguajes Influyentes

- **Go**: Inspiración para las interfaces estructurales.
- **Swift**: Inspiración para el manejo de protocolos con varianza y la sintaxis de `as`/`is`.
- **Python**: Inspiración para la sintaxis general del lenguaje y el bucle `for-in`.
- **Haskell**: Inspiración para la naturaleza expresiva del lenguaje y el `let-in` con múltiples bindings.

---
