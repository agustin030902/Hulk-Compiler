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

        self.protocol_real_types = analyzer
            .protocol_real_types()
            .iter()
            .map(|(name, type_id)| (name.clone(), type_id.0))
            .collect();

        self.param_real_types = analyzer
            .flat_param_real_types()
            .iter()
            .map(|(name, type_id)| (name.clone(), type_id.0))
            .collect();

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

            current = self
                .type_ids
                .iter()
                .find_map(|(name, id)| (*id == type_id).then_some(name.as_str()))
                .and_then(|name| self.type_decls.get(name))
                .and_then(|decl| decl.parent_name.as_ref())
                .and_then(|parent_name| self.type_ids.get(parent_name).copied());
        }

        None
    }
}
