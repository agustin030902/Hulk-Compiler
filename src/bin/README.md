# Hulk GUI (bin/gui.rs)

Guía rápida de la interfaz para probar el compilador Hulk con eframe/egui.

## Barra superior
- **Ejemplos** (ComboBox): lista todos los `.hulk`/`.hk` en `examples/`. Al seleccionar, se llena la ruta en el campo adyacente.
- **Ruta** (input de texto): puedes editar manualmente cualquier archivo fuente.
- **Refrescar ejemplos**: vuelve a escanear `examples/` por si agregaste/quitas archivos.
- **Cargar**: lee el archivo indicado en la ruta y lo copia al editor de la izquierda.
- **Demo rápida**: coloca un snippet de prueba mínimo.
- **Compilar**: ejecuta el pipeline completo (lexer → parser → semantic → LLVM IR). Si genera IR, automáticamente lo corre con `lli` usando la ruta configurada.
- **lli** (input de texto): ruta/comando a `lli` (por defecto `lli`). Edita aquí si está fuera de tu PATH.

## Panel izquierdo
- **Editor Hulk**: zona de texto con scroll donde escribes o pegas el código a compilar.

## Panel central (colapsables)
- **Errores**: muestra errores lex/sintácticos/semánticos; se vacía si no hay errores.
- **Tokens**: lista todos los tokens detectados por el lexer.
- **AST**: AST formateado (`{:#?}`); solo aparece si no hubo errores previos.
- **LLVM IR**: IR generado; indica también la ruta del archivo `.ll`.
- **Salida lli**: stdout/stderr y exit code al ejecutar el IR con `lli`. Incluye un botón **Re-ejecutar lli** para correr de nuevo sin recompilar.

## Flujo típico
1) Elige un ejemplo en el ComboBox o escribe la ruta; pulsa **Cargar**.
2) Opcional: ajusta la ruta de `lli`.
3) Pulsa **Compilar**.
4) Revisa errores/tokens/AST/IR y la **Salida lli**. Si cambiaste solo `lli`, usa **Re-ejecutar lli**.

## Nota
La GUI interpreta IR con `lli`; para generar/ejecutar binarios nativos usa la CLI `cargo run -- run ...` con `clang` (ver README principal).
