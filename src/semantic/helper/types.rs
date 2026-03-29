#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticType {
    Number,
    Boolean,
    String,
    Unit,
    Unknown,
}

impl SemanticType {
    pub(in crate::semantic) fn display_name(self) -> &'static str {
        match self {
            SemanticType::Number => "Number",
            SemanticType::Boolean => "Boolean",
            SemanticType::String => "String",
            SemanticType::Unit => "Unit",
            SemanticType::Unknown => "Unknown",
        }
    }
}
