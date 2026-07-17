//! Primitivos de emisión de texto: los tres buffers de líneas del módulo
//! (cuerpo de `main`, funciones y globals) y los generadores de nombres
//! frescos (temporales, etiquetas, constantes de string).

use super::LlvmBackend;

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_body(&mut self, line: impl Into<String>) {
        let line = line.into();

        // Rastrear el bloque actual permite a los `phi` conocer la etiqueta
        // desde la que realmente se saltó (p. ej. en el dispatch de interfaces).
        if line.ends_with(':') {
            self.current_block = line.trim_end_matches(':').to_string();
        }

        self.body_lines.push(line);
    }

    pub(in crate::codegen::llvm) fn emit_function_line(&mut self, line: impl Into<String>) {
        self.function_lines.push(line.into());
    }

    pub(in crate::codegen::llvm) fn emit_global(&mut self, line: impl Into<String>) {
        self.global_lines.push(line.into());
    }

    pub(in crate::codegen::llvm) fn next_temp(&mut self) -> String {
        let current = self.temp_counter;
        self.temp_counter += 1;
        format!("%t{}", current)
    }

    pub(in crate::codegen::llvm) fn next_label(&mut self, prefix: &str) -> String {
        let current = self.label_counter;
        self.label_counter += 1;
        format!("{prefix}.{current}")
    }

    pub(in crate::codegen::llvm) fn next_string_name(&mut self) -> String {
        let current = self.string_counter;
        self.string_counter += 1;
        format!("@.str.{}", current)
    }
}
