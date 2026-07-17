//! Soporte de runtime emitido al inicio de cada módulo: la tabla de padres
//! `@hulk_type_parents` y la función `@hulk_is_subtype`, que implementan el
//! subtipado en tiempo de ejecución que consume el operador `is`.

use super::LlvmBackend;

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_type_hierarchy_globals(&mut self) {
        let max_type_id = self.type_ids.values().copied().max().unwrap_or(8) + 1;
        let mut parent_entries = vec!["-1".to_string(); max_type_id as usize];

        if let Some(object_id) = self.type_ids.get("Object").copied() {
            parent_entries[object_id as usize] = "-1".to_string();
        }
        if let Some(iterable_id) = self.type_ids.get("Iterable").copied() {
            parent_entries[iterable_id as usize] = "-1".to_string();
        }
        if let Some(enumerable_id) = self.type_ids.get("Enumerable").copied() {
            parent_entries[enumerable_id as usize] = "-1".to_string();
        }
        if let Some(range_id) = self.type_ids.get("Range").copied() {
            if let Some(object_id) = self.type_ids.get("Object").copied() {
                parent_entries[range_id as usize] = object_id.to_string();
            }
        }

        for (name, type_id) in &self.type_ids {
            if let Some(decl) = self.type_decls.get(name) {
                if let Some(parent_name) = &decl.parent_name {
                    if let Some(parent_id) = self.type_ids.get(parent_name).copied() {
                        parent_entries[*type_id as usize] = parent_id.to_string();
                    }
                }
            }
        }

        // Completar con la jerarquía semántica (interfaces declaradas, builtin
        // y splat sintetizadas) para que `is` funcione también sobre ellas.
        for (child, parent) in &self.type_parents {
            if (*child as usize) < parent_entries.len() && parent_entries[*child as usize] == "-1" {
                parent_entries[*child as usize] = parent.to_string();
            }
        }

        let entries_str = parent_entries
            .iter()
            .map(|v| format!("i64 {v}"))
            .collect::<Vec<_>>()
            .join(", ");
        self.emit_global(format!(
            "@hulk_type_parents = internal global [{max_type_id} x i64] [{entries_str}]"
        ));

        self.emit_function_line(format!(
            "define i1 @hulk_is_subtype(i64 %child, i64 %parent) {{"
        ));
        self.emit_function_line("entry:".to_string());
        self.emit_function_line("  %cmp0 = icmp eq i64 %child, %parent".to_string());
        self.emit_function_line("  br i1 %cmp0, label %ret_true, label %walk".to_string());
        self.emit_function_line("walk:".to_string());
        self.emit_function_line(
            "  %current = phi i64 [ %child, %entry ], [ %parent_id, %check ]".to_string(),
        );
        self.emit_function_line(format!(
            "  %idx = getelementptr [{max_type_id} x i64], [{max_type_id} x i64]* @hulk_type_parents, i64 0, i64 %current"
        ));
        self.emit_function_line("  %parent_id = load i64, i64* %idx".to_string());
        self.emit_function_line("  %is_neg1 = icmp eq i64 %parent_id, -1".to_string());
        self.emit_function_line("  br i1 %is_neg1, label %ret_false, label %check".to_string());
        self.emit_function_line("check:".to_string());
        self.emit_function_line("  %cmp_eq = icmp eq i64 %parent_id, %parent".to_string());
        self.emit_function_line("  br i1 %cmp_eq, label %ret_true, label %walk".to_string());
        self.emit_function_line("ret_true:".to_string());
        self.emit_function_line("  ret i1 true".to_string());
        self.emit_function_line("ret_false:".to_string());
        self.emit_function_line("  ret i1 false".to_string());
        self.emit_function_line("}".to_string());
    }
}
