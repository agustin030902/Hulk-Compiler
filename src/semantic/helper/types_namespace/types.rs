#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticType {
    Number,
    Boolean,
    String,
    Unit,
    Null,
    Function(u32),
    Struct(u32),
    Unknown,
}

impl SemanticType {
    pub(in crate::semantic) fn display_name(self) -> &'static str {
        match self {
            SemanticType::Number => "Number",
            SemanticType::Boolean => "Boolean",
            SemanticType::String => "String",
            SemanticType::Unit => "Unit",
            SemanticType::Null => "Null",
            SemanticType::Function(_) => "Function",
            SemanticType::Struct(_) => "Struct",
            SemanticType::Unknown => "Unknown",
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

    pub(in crate::semantic) const fn is_nullable(self) -> bool {
        matches!(
            self,
            SemanticType::Null
                | SemanticType::String
                | SemanticType::Function(_)
                | SemanticType::Struct(_)
        )
    }
}
