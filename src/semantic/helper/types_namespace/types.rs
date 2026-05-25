#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticType {
    Number,
    Boolean,
    String,
    Null,
    Unit,
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
            SemanticType::Null => "Null",
            SemanticType::Unit => "Unit",
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
            "Null" => Some(SemanticType::Null),
            "Unit" => Some(SemanticType::Unit),
            _ => None,
        }
    }

    pub(in crate::semantic) const fn annotation_names() -> &'static str {
        "Number, Boolean, String, Null, Unit"
    }
}
