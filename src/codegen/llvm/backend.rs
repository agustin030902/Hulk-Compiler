use std::collections::HashMap;

use crate::{
    codegen::CodegenBackend,
    error::{CompilerError, ErrorCategory},
    parser::expression::{Program, TypeDecl},
    semantic::{SemanticAnalyzer, SemanticType, TypeId, TypeInfo},
};

use super::helper::state::{ValueRef, ValueType, VariableInfo};

#[derive(Debug, Clone)]
pub(super) struct FunctionInfo {
    pub(super) llvm_name: String,
    pub(super) receiver_type_id: Option<u32>,
    pub(super) param_types: Vec<ValueType>,
    pub(super) return_type: ValueType,
}

#[derive(Debug, Clone)]
pub(super) struct FieldLayout {
    pub(super) offset: usize,
    pub(super) value_type: ValueType,
}

#[derive(Debug, Clone)]
pub(super) struct StructLayout {
    pub(super) size_bytes: usize,
    pub(super) fields: HashMap<String, FieldLayout>,
}

#[derive(Debug, Default)]
pub struct LlvmBackend {
    pub(super) body_lines: Vec<String>,
    pub(super) function_lines: Vec<String>,
    pub(super) global_lines: Vec<String>,
    pub(super) errors: Vec<CompilerError>,
    pub(super) scopes: Vec<HashMap<String, VariableInfo>>,
    pub(super) functions: HashMap<String, FunctionInfo>,
    pub(super) type_ids: HashMap<String, u32>,
    pub(super) type_decls: HashMap<String, TypeDecl>,
    pub(super) struct_layouts: HashMap<u32, StructLayout>,
    pub(super) method_dispatch: HashMap<(u32, String), String>,
    pub(super) temp_counter: usize,
    pub(super) label_counter: usize,
    pub(super) string_counter: usize,
}

impl LlvmBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub(super) fn reset(&mut self) {
        self.body_lines.clear();
        self.function_lines.clear();
        self.global_lines.clear();
        self.errors.clear();
        self.scopes.clear();
        self.functions.clear();
        self.type_ids.clear();
        self.type_decls.clear();
        self.struct_layouts.clear();
        self.method_dispatch.clear();
        self.temp_counter = 0;
        self.label_counter = 0;
        self.string_counter = 0;
        self.push_scope();
    }

    pub(super) fn emit_body(&mut self, line: impl Into<String>) {
        self.body_lines.push(line.into());
    }

    pub(super) fn emit_function_line(&mut self, line: impl Into<String>) {
        self.function_lines.push(line.into());
    }

    pub(super) fn emit_global(&mut self, line: impl Into<String>) {
        self.global_lines.push(line.into());
    }

    pub(super) fn next_temp(&mut self) -> String {
        let current = self.temp_counter;
        self.temp_counter += 1;
        format!("%t{}", current)
    }

    pub(super) fn next_label(&mut self, prefix: &str) -> String {
        let current = self.label_counter;
        self.label_counter += 1;
        format!("{prefix}.{current}")
    }

    pub(super) fn next_string_name(&mut self) -> String {
        let current = self.string_counter;
        self.string_counter += 1;
        format!("@.str.{}", current)
    }

    pub(super) fn semantic_error(&mut self, message: impl Into<String>) {
        self.errors
            .push(CompilerError::new(ErrorCategory::Semantic, message, 1, 1));
    }

    pub(super) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn is_declared_in_current_scope(&self, name: &str) -> bool {
        self.scopes
            .last()
            .map(|scope| scope.contains_key(name))
            .unwrap_or(false)
    }

    pub(super) fn lookup_var(&self, name: &str) -> Option<VariableInfo> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    pub(super) fn lookup_var_with_index(&self, name: &str) -> Option<(usize, VariableInfo)> {
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(idx, scope)| scope.get(name).cloned().map(|info| (idx, info)))
    }

    pub(super) fn allocate_storage(&mut self, value_ref: &ValueRef) -> VariableInfo {
        let ptr_name = self.next_temp();
        let llvm_ty = value_ref.value_type.llvm_type();
        self.emit_body(format!("{ptr_name} = alloca {llvm_ty}"));
        self.emit_body(format!(
            "store {llvm_ty} {}, {llvm_ty}* {ptr_name}",
            value_ref.repr
        ));

        VariableInfo {
            ptr_name,
            value_type: value_ref.value_type,
        }
    }

    pub(super) fn store_value_at(&mut self, ptr_name: &str, value_ref: &ValueRef) {
        let llvm_ty = value_ref.value_type.llvm_type();
        self.emit_body(format!(
            "store {llvm_ty} {}, {llvm_ty}* {ptr_name}",
            value_ref.repr
        ));
    }

    pub(super) fn bind_current_scope(&mut self, name: String, info: VariableInfo) {
        self.scopes
            .last_mut()
            .expect("a scope should always be present")
            .insert(name, info);
    }

    pub(super) fn bind_scope(&mut self, scope_index: usize, name: String, info: VariableInfo) {
        self.scopes[scope_index].insert(name, info);
    }

    pub(super) fn load_function_signatures(&mut self, program: &Program) -> bool {
        let mut analyzer = SemanticAnalyzer::new();
        let semantic_errors = analyzer.analyze(program, "");

        if !semantic_errors.is_empty() {
            self.errors.extend(semantic_errors);
            return false;
        }

        self.type_decls = program
            .types
            .iter()
            .cloned()
            .map(|type_decl| (type_decl.name.clone(), type_decl))
            .collect::<HashMap<_, _>>();

        self.type_ids = analyzer
            .type_symbols()
            .iter()
            .map(|(name, type_id)| (name.clone(), type_id.0))
            .collect::<HashMap<_, _>>();

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

    fn load_struct_layouts(&mut self, analyzer: &SemanticAnalyzer) -> bool {
        for (type_name, type_id) in analyzer.type_symbols() {
            let Some(type_info) = analyzer.type_table().get_struct(*type_id) else {
                self.semantic_error(format!(
                    "Type '{}' is registered but has no struct entry.",
                    type_name
                ));
                return false;
            };

            let mut offset = 0usize;
            let mut max_align = 1usize;
            let mut fields = HashMap::new();

            for (field_name, field_type_id) in &type_info.fields {
                let semantic_type = self.semantic_type_from_type_id(analyzer, *field_type_id);
                let Some(value_type) = self.lower_semantic_type(
                    semantic_type,
                    &format!("field '{}' in type '{}'", field_name, type_name),
                ) else {
                    return false;
                };

                let (size, align) = Self::value_layout(value_type);
                offset = Self::align_to(offset, align);
                max_align = max_align.max(align);

                fields.insert(field_name.clone(), FieldLayout { offset, value_type });

                offset += size;
            }

            let total_size = Self::align_to(offset.max(1), max_align);
            self.struct_layouts.insert(
                type_id.0,
                StructLayout {
                    size_bytes: total_size,
                    fields,
                },
            );
        }

        true
    }

    fn semantic_type_from_type_id(
        &self,
        analyzer: &SemanticAnalyzer,
        type_id: TypeId,
    ) -> SemanticType {
        match analyzer.type_table().get(type_id) {
            TypeInfo::Number => SemanticType::Number,
            TypeInfo::Boolean => SemanticType::Boolean,
            TypeInfo::String => SemanticType::String,
            TypeInfo::Unit => SemanticType::Unit,
            TypeInfo::Unknown => SemanticType::Unknown,
            TypeInfo::Function(_) => SemanticType::Function(type_id.0),
            TypeInfo::Type(_) => SemanticType::Struct(type_id.0),
        }
    }

    fn align_to(value: usize, alignment: usize) -> usize {
        if alignment <= 1 {
            return value;
        }
        let remainder = value % alignment;
        if remainder == 0 {
            value
        } else {
            value + (alignment - remainder)
        }
    }

    fn value_layout(value_type: ValueType) -> (usize, usize) {
        match value_type {
            ValueType::Double => (8, 8),
            ValueType::Bool => (1, 1),
            ValueType::StringPtr | ValueType::Function | ValueType::Struct(_) => (8, 8),
            ValueType::Unit => (1, 1),
        }
    }

    pub(super) fn lookup_method_key(
        &self,
        receiver_type_id: u32,
        method_name: &str,
    ) -> Option<&String> {
        self.method_dispatch
            .get(&(receiver_type_id, method_name.to_string()))
    }

    pub(super) fn struct_layout(&self, type_id: u32) -> Option<&StructLayout> {
        self.struct_layouts.get(&type_id)
    }

    pub(super) fn field_layout(&self, type_id: u32, field_name: &str) -> Option<&FieldLayout> {
        self.struct_layout(type_id)
            .and_then(|layout| layout.fields.get(field_name))
    }

    pub(super) fn emit_field_ptr(
        &mut self,
        object_repr: &str,
        field_offset: usize,
        value_type: ValueType,
    ) -> String {
        let raw_ptr = self.next_temp();
        self.emit_body(format!(
            "{raw_ptr} = getelementptr i8, i8* {object_repr}, i64 {field_offset}"
        ));

        let typed_ptr = self.next_temp();
        self.emit_body(format!(
            "{typed_ptr} = bitcast i8* {raw_ptr} to {}*",
            value_type.llvm_type()
        ));
        typed_ptr
    }

    fn lower_semantic_type(
        &mut self,
        semantic_type: SemanticType,
        context: &str,
    ) -> Option<ValueType> {
        let lowered = match semantic_type {
            SemanticType::Number => ValueType::Double,
            SemanticType::Boolean => ValueType::Bool,
            SemanticType::String => ValueType::StringPtr,
            SemanticType::Unit => ValueType::Unit,
            SemanticType::Function(_) => ValueType::Function,
            SemanticType::Struct(type_id) => ValueType::Struct(type_id),
            SemanticType::Unknown => {
                self.semantic_error(format!(
                    "Could not infer a concrete type for {context} before code generation."
                ));
                return None;
            }
        };

        Some(lowered)
    }
}

impl CodegenBackend for LlvmBackend {
    fn generate(&mut self, program: &Program) -> Result<String, Vec<CompilerError>> {
        self.reset();

        if !self.load_function_signatures(program) {
            return Err(self.errors.clone());
        }

        self.emit_program(program);

        if self.errors.is_empty() {
            Ok(self.compose_module())
        } else {
            Err(self.errors.clone())
        }
    }
}
