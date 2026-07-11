use crate::{
    parser::expression::Program,
    semantic::{builtins, SemanticAnalyzer},
};

use super::LlvmBackend;

#[derive(Debug, Clone)]
pub(in crate::codegen::llvm) struct FunctionInfo {
    pub(in crate::codegen::llvm) llvm_name: String,
    pub(in crate::codegen::llvm) receiver_type_id: Option<u32>,
    pub(in crate::codegen::llvm) param_types: Vec<crate::codegen::llvm::helper::state::ValueType>,
    pub(in crate::codegen::llvm) return_type: crate::codegen::llvm::helper::state::ValueType,
}

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn load_function_signatures(&mut self, program: &Program) -> bool {
        let mut analyzer = SemanticAnalyzer::new();
        let semantic_errors = analyzer.analyze(program, "");

        if !semantic_errors.is_empty() {
            self.errors.extend(semantic_errors);
            return false;
        }

        let mut type_decls_map: std::collections::HashMap<String, crate::parser::expression::TypeDecl> =
            program
                .types
                .iter()
                .cloned()
                .map(|type_decl| (type_decl.name.clone(), type_decl))
                .collect();
        type_decls_map
            .entry("Object".to_string())
            .or_insert_with(builtins::build_object_type_decl);
        type_decls_map
            .entry(builtins::RANGE_NAME.to_string())
            .or_insert_with(builtins::build_range_type_decl);

        self.type_decls = type_decls_map;

        self.type_ids = analyzer
            .type_symbols()
            .iter()
            .map(|(name, type_id)| (name.clone(), type_id.0))
            .collect();

        self.interface_real_types = analyzer
            .interface_real_types()
            .iter()
            .map(|(name, type_id)| (name.clone(), type_id.0))
            .collect();

        self.param_real_types = analyzer
            .flat_param_real_types()
            .iter()
            .map(|(name, type_id)| (name.clone(), type_id.0))
            .collect();

        self.type_parents = analyzer
            .type_symbols()
            .values()
            .filter_map(|type_id| {
                analyzer
                    .type_table()
                    .get_struct(*type_id)
                    .and_then(|info| info.parent)
                    .map(|parent| (type_id.0, parent.0))
            })
            .collect();

        for (function_id, param_ids, return_id) in
            analyzer.type_table().plain_function_entries()
        {
            let params = param_ids
                .iter()
                .map(|param_id| {
                    Self::lower_semantic_type_quiet(
                        self.semantic_type_from_type_id(&analyzer, *param_id),
                    )
                })
                .collect::<Option<Vec<_>>>();
            let return_type = Self::lower_semantic_type_quiet(
                self.semantic_type_from_type_id(&analyzer, return_id),
            );
            if let (Some(params), Some(return_type)) = (params, return_type) {
                self.function_types
                    .insert(function_id.0, (params, return_type));
            }
        }

        for (array_id, elem_id) in analyzer.type_table().array_entries() {
            let elem_semantic = self.semantic_type_from_type_id(&analyzer, elem_id);
            let Some(elem_value_type) =
                self.lower_semantic_type(elem_semantic, "array element type")
            else {
                return false;
            };
            self.array_elems.insert(array_id.0, elem_value_type);
        }

        if !self.load_struct_layouts(&analyzer) {
            return false;
        }

        for (key, signature) in analyzer.function_signatures() {
            let Some(symbol) = analyzer.function_symbols().get(key) else {
                self.semantic_error(format!(
                    "Function signature '{}' has no symbol metadata.",
                    key
                ));
                return false;
            };

            let mut param_types = Vec::with_capacity(signature.param_types.len());
            for (index, semantic_type) in signature.param_types.iter().copied().enumerate() {
                let Some(value_type) = self.lower_semantic_type(
                    semantic_type,
                    &format!("parameter #{} in function '{}'", index + 1, symbol.name),
                ) else {
                    return false;
                };
                param_types.push(value_type);
            }

            let Some(return_type) = self.lower_semantic_type(
                signature.return_type,
                &format!("return type in function '{}'", symbol.name),
            ) else {
                return false;
            };

            let llvm_name = if let Some(receiver_type_id) = symbol.receiver {
                format!("hulk_type{}_{}", receiver_type_id.0, symbol.name)
            } else {
                format!("hulk_{}", symbol.name)
            };

            self.functions.insert(
                key.clone(),
                FunctionInfo {
                    llvm_name,
                    receiver_type_id: symbol.receiver.map(|type_id| type_id.0),
                    param_types,
                    return_type,
                },
            );

            if let Some(receiver_type_id) = symbol.receiver {
                self.method_dispatch
                    .insert((receiver_type_id.0, symbol.name.clone()), key.clone());
            }
        }

        true
    }

    pub(in crate::codegen::llvm) fn lookup_method_key(
        &self,
        receiver_type_id: u32,
        method_name: &str,
    ) -> Option<&String> {
        let mut current = Some(receiver_type_id);
        while let Some(type_id) = current {
            if let Some(key) = self.method_dispatch.get(&(type_id, method_name.to_string())) {
                return Some(key);
            }

            // type_parents cubre también interfaces (incluidas las splat
            // sintetizadas), que no aparecen en type_decls.
            current = self.type_parents.get(&type_id).copied().or_else(|| {
                self.type_ids
                    .iter()
                    .find_map(|(name, id)| (*id == type_id).then_some(name.as_str()))
                    .and_then(|name| self.type_decls.get(name))
                    .and_then(|decl| decl.parent_name.as_ref())
                    .and_then(|parent_name| self.type_ids.get(parent_name).copied())
            });
        }

        None
    }
}
