use super::TypeTable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticType {
    Number,
    Boolean,
    String,
    Unit,
    Null,
    Function(u32),
    Struct(u32),
    Array(Box<SemanticType>),
    Unknown,
}

impl SemanticType {
    pub(in crate::semantic) fn display_name(&self) -> &'static str {
        match self {
            SemanticType::Number => "Number",
            SemanticType::Boolean => "Boolean",
            SemanticType::String => "String",
            SemanticType::Unit => "Unit",
            SemanticType::Null => "Null",
            SemanticType::Function(_) => "Function",
            SemanticType::Struct(_) => "Struct",
            SemanticType::Array(_) => "Array",
            SemanticType::Unknown => "Unknown",
        }
    }

    pub(in crate::semantic) fn display_name_with_table(&self, table: &TypeTable) -> String {
        match self {
            SemanticType::Number => "Number".to_string(),
            SemanticType::Boolean => "Boolean".to_string(),
            SemanticType::String => "String".to_string(),
            SemanticType::Unit => "Unit".to_string(),
            SemanticType::Null => "Null".to_string(),
            SemanticType::Unknown => "Unknown".to_string(),
            SemanticType::Array(element) => {
                format!("{}[]", element.display_name_with_table(table))
            }
            SemanticType::Struct(id) => {
                let type_id = super::TypeId(*id);
                table
                    .get_struct(type_id)
                    .map(|info| info.name.clone())
                    .unwrap_or_else(|| "Struct".to_string())
            }
            SemanticType::Function(id) => {
                let type_id = super::TypeId(*id);
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

    pub(in crate::semantic) const fn annotation_names() -> &'static str {
        "Number, Boolean, String, Unit, Null"
    }

    pub(in crate::semantic) fn is_nullable(&self) -> bool {
        matches!(
            self,
            SemanticType::Null
                | SemanticType::String
                | SemanticType::Function(_)
                | SemanticType::Struct(_)
                | SemanticType::Array(_)
        )
    }

    pub(in crate::semantic) fn element_type(&self) -> Option<&SemanticType> {
        match self {
            SemanticType::Array(element) => Some(element),
            _ => None,
        }
    }
}
