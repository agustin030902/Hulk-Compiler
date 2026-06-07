use std::collections::HashMap;

use crate::semantic::SemanticAnalyzer;

use super::LlvmBackend;
use crate::codegen::llvm::helper::state::ValueType;

#[derive(Debug, Clone)]
pub(in crate::codegen::llvm) struct FieldLayout {
    pub(in crate::codegen::llvm) offset: usize,
    pub(in crate::codegen::llvm) value_type: ValueType,
}

#[derive(Debug, Clone)]
pub(in crate::codegen::llvm) struct StructLayout {
    pub(in crate::codegen::llvm) size_bytes: usize,
    pub(in crate::codegen::llvm) fields: HashMap<String, FieldLayout>,
}

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn load_struct_layouts(
        &mut self,
        analyzer: &SemanticAnalyzer,
    ) -> bool {
        let entries = analyzer
            .type_symbols()
            .iter()
            .map(|(name, type_id)| (name.clone(), *type_id))
            .collect::<Vec<_>>();

        for (type_name, type_id) in entries {
            if !self.ensure_struct_layout(analyzer, &type_name, type_id) {
                return false;
            }
        }

        true
    }

    fn ensure_struct_layout(
        &mut self,
        analyzer: &SemanticAnalyzer,
        type_name: &str,
        type_id: crate::semantic::TypeId,
    ) -> bool {
        if self.struct_layouts.contains_key(&type_id.0) {
            return true;
        }

        let Some(type_info) = analyzer.type_table().get_struct(type_id) else {
            self.semantic_error(format!(
                "Type '{}' is registered but has no struct entry.",
                type_name
            ));
            return false;
        };

        if type_info.is_protocol {
            return true;
        }

        let mut offset = 0usize;
        let mut max_align = 1usize;
        let mut fields = HashMap::new();

        if let Some(parent_id) = type_info.parent {
            let parent_name = analyzer
                .type_symbols()
                .iter()
                .find_map(|(name, id)| (*id == parent_id).then_some(name.clone()))
                .unwrap_or_else(|| "Object".to_string());
            if !self.ensure_struct_layout(analyzer, &parent_name, parent_id) {
                return false;
            }

            if let Some(parent_layout) = self.struct_layouts.get(&parent_id.0).cloned() {
                offset = parent_layout.size_bytes;
                fields.extend(parent_layout.fields);
            }
        }

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

        true
    }

    pub(in crate::codegen::llvm) fn struct_layout(&self, type_id: u32) -> Option<&StructLayout> {
        self.struct_layouts.get(&type_id)
    }

    pub(in crate::codegen::llvm) fn field_layout(
        &self,
        type_id: u32,
        field_name: &str,
    ) -> Option<&FieldLayout> {
        self.struct_layout(type_id)
            .and_then(|layout| layout.fields.get(field_name))
    }

    pub(in crate::codegen::llvm) fn emit_field_ptr(
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
}
