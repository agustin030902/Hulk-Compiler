#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticType {
    Number,
    Boolean,
    String,
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
            SemanticType::Unit => "Unit",
            SemanticType::Function(_) => "Function",
            SemanticType::Struct(_) => "Struct",
            SemanticType::Unknown => "Unknown",
        }
    }
}
