use super::{TypeId, TypeInfo, TypeTable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticType(pub TypeId);

impl SemanticType {
    pub(in crate::semantic) fn display_name(
        self,
        types: &TypeTable,
    ) -> &'static str {
        match types.get(self.0) {
            TypeInfo::Number => "Number",
            TypeInfo::Boolean => "Boolean",
            TypeInfo::String => "String",
            TypeInfo::Unit => "Unit",
            TypeInfo::Unknown => "Unknown",
            TypeInfo::Function(_) => "Function",
            TypeInfo::Type(_) => "Type",
        }
    }

    pub(in crate::semantic) fn from_annotation_name(
        name: &str,
        types: &TypeTable,
    ) -> Option<Self> {
        match name {
            "Number" => Some(Self(types.number)),
            "Boolean" => Some(Self(types.boolean)),
            "String" => Some(Self(types.string)),
            "Unit" => Some(Self(types.unit)),
            _ => None,
        }
    }

    pub(in crate::semantic) const fn annotation_names() -> &'static str {
        "Number, Boolean, String, Unit"
    }
}