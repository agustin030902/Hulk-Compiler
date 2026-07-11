use super::TypeTable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticType {
    Number,
    Boolean,
    String,
    Unit,
    Null,
    Function(u32),
    Struct(u32),
    /// Arreglo `T[]`; el u32 es el TypeId de la entrada `TypeInfo::Array`.
    Array(u32),
    Unknown,
}

impl SemanticType {
    pub(in crate::semantic) fn display_name_with_table(self, table: &TypeTable) -> String {
        match self {
            SemanticType::Number => "Number".to_string(),
            SemanticType::Boolean => "Boolean".to_string(),
            SemanticType::String => "String".to_string(),
            SemanticType::Unit => "Unit".to_string(),
            SemanticType::Null => "Null".to_string(),
            SemanticType::Unknown => "Unknown".to_string(),
            SemanticType::Struct(id) => {
                let type_id = super::TypeId(id);
                table
                    .get_struct(type_id)
                    .map(|info| info.name.clone())
                    .unwrap_or_else(|| "Struct".to_string())
            }
            SemanticType::Function(id) => {
                let type_id = super::TypeId(id);
                table
                    .get_function(type_id)
                    .map(|info| {
                        if info.is_method() {
                            "Method".to_string()
                        } else {
                            "Function".to_string()
                        }
                    })
                    .unwrap_or_else(|| "Function".to_string())
            }
            SemanticType::Array(id) => {
                let elem = table.get_array_elem(super::TypeId(id));
                match elem {
                    Some(elem_id) => {
                        let elem_type = Self::from_type_id_shallow(elem_id, table);
                        format!("{}[]", elem_type.display_name_with_table(table))
                    }
                    None => "Array".to_string(),
                }
            }
        }
    }

    /// Conversión superficial TypeId → SemanticType para nombres de arreglos.
    fn from_type_id_shallow(type_id: super::TypeId, table: &TypeTable) -> SemanticType {
        use super::TypeInfo;
        match table.get(type_id) {
            TypeInfo::Number => SemanticType::Number,
            TypeInfo::Boolean => SemanticType::Boolean,
            TypeInfo::String => SemanticType::String,
            TypeInfo::Unit => SemanticType::Unit,
            TypeInfo::Null => SemanticType::Null,
            TypeInfo::Unknown => SemanticType::Unknown,
            TypeInfo::Type(_) => SemanticType::Struct(type_id.0),
            TypeInfo::Function(_) => SemanticType::Function(type_id.0),
            TypeInfo::Array { .. } => SemanticType::Array(type_id.0),
        }
    }

    pub(in crate::semantic) fn from_annotation_name(name: &str) -> Option<Self> {
        match name {
            "Number" => Some(SemanticType::Number),
            "Boolean" => Some(SemanticType::Boolean),
            "String" => Some(SemanticType::String),
            "Unit" => Some(SemanticType::Unit),
            "Null" => Some(SemanticType::Null),
            _ => None,
        }
    }

    pub(in crate::semantic) const fn is_nullable(self) -> bool {
        matches!(
            self,
            SemanticType::Null
                | SemanticType::String
                | SemanticType::Function(_)
                | SemanticType::Struct(_)
                | SemanticType::Array(_)
        )
    }
}
