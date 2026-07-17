# 🚨 Error — Diagnóstico Unificado

Todas las fases reportan errores con el mismo tipo,
[`CompilerError`](mod.rs), que conserva **categoría + mensaje + línea/columna**.

![Flujo de errores](docs/error-flow.svg)

## Flujo

1. Cualquier fase construye `CompilerError::new(categoría, mensaje, línea, col)`.
   El parser traduce los offsets de byte con
   [`offset_to_line_column`](mod.rs).
2. La categoría ([`ErrorCategory`](mod.rs)) determina **dos salidas**:
   - El prefijo del mensaje en `stderr` (`Display`):
     `(línea,columna) CATEGORÍA: mensaje`
   - El **exit code** de la CLI ([`exit_code()`](mod.rs))

| Categoría | Prefijo | Exit code |
|-----------|---------|-----------|
| `Lexical` | `LEXICAL` | `1` |
| `Syntax` | `SYNTACTIC` | `2` |
| `Type` | `SEMANTIC` | `3` |
| `Semantic` | `SEMANTIC` | `3` |

`Type` y `Semantic` se distinguen internamente (ayuda a depurar el
compilador) pero de cara al usuario son la misma categoría — es lo que exige
el contrato del corrector automático.

## Filosofía

- **Acumular, no abortar**: el lexer y los checkers recolectan tantos errores
  como pueden por pasada; solo el *cambio de fase* es fail-fast.
- **Posiciones siempre**: ningún error se emite sin línea/columna — por eso
  todo nodo del AST lleva su `Span`.
