# 🔤 Lexer — Análisis Léxico

Convierte el texto fuente en una secuencia de tokens usando el generador
[`logos`](https://docs.rs/logos). Es la primera fase del pipeline y la única
que ve el texto crudo.

![Pipeline del lexer](docs/lexer-pipeline.svg)

## Flujo

1. **Escaneo** — `logos` recorre el fuente saltando espacios en blanco y
   comentarios de línea (`// …`). Las palabras clave llevan `priority = 3`
   para dominar sobre el patrón de identificadores (`true` es booleano, no
   identificador).
2. **Clasificación** — cada lexema se convierte en una de las **64 variantes**
   de [`TokenKind`](token.rs): palabras clave, operadores (incluidos `->`,
   `:=`, `@@`, `[` `]`), literales y puntuación.
3. **Casos especiales**:
   - *Strings*: los escapes `\"`, `\n`, `\t` se procesan aquí
     (`unescape_string_contents`); un escape inválido es error léxico.
   - *Identificadores inválidos*: `123abc` se detecta con un patrón dedicado
     (`InvalidNumberIdent`) para dar un mensaje preciso.
4. **Recuperación de errores** — un error léxico **no aborta**: se reporta el
   [`CompilerError`](../error/README.md) y se emite `TokenKind::Unknown` como
   token de continuidad, para que el parser pueda seguir y reportar más
   errores en una sola pasada.
5. **Posiciones** — cada [`Token`](token.rs) conserva línea, columna y offsets
   de byte (`start..end`); los usan los diagnósticos y el resaltado del
   editor de la GUI (que reutiliza este mismo lexer).

## Archivos

| Archivo | Rol |
|---------|-----|
| `mod.rs` | Definición `logos` de los patrones y el loop de escaneo con recuperación |
| `token.rs` | `TokenKind` (64 variantes) y `Token` con posiciones |
| `tests/` | Tests unitarios por categoría de token |

**Salida:** `Vec<Token>` (terminado en `EOF`) + errores léxicos → exit code `1`.
