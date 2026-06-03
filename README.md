# Uso del proyecto Hulk Compiler

## Introducción

El proyecto **Hulk Compiler** es un compilador implementado en Rust que transforma programas escritos en el lenguaje Hulk hacia LLVM IR, y posteriormente en un ejecutable nativo mediante `clang`.

El sistema está diseñado como un pipeline por fases:

```text
lexer → parser (LR1) → semantic → codegen LLVM → clang → executable