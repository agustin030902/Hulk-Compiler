#[derive(Debug, Clone)]
pub(in crate::codegen::llvm) struct VariableInfo {
    pub(in crate::codegen::llvm) ptr_name: String,
    pub(in crate::codegen::llvm) value_type: ValueType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::codegen::llvm) enum ElementTag {
    Double,
    Bool,
    StringPtr,
    Unit,
    Null,
    Function,
    Struct(u32),
    Array,
}

impl ElementTag {
    pub(in crate::codegen::llvm) fn to_value_type(self) -> ValueType {
        match self {
            ElementTag::Double => ValueType::Double,
            ElementTag::Bool => ValueType::Bool,
            ElementTag::StringPtr => ValueType::StringPtr,
            ElementTag::Unit => ValueType::Unit,
            ElementTag::Null => ValueType::Null,
            ElementTag::Function => ValueType::Function,
            ElementTag::Struct(id) => ValueType::Struct(id),
            ElementTag::Array => ValueType::ArrayPtr,
        }
    }
}

impl From<ValueType> for ElementTag {
    fn from(vt: ValueType) -> Self {
        match vt {
            ValueType::Double => ElementTag::Double,
            ValueType::Bool => ElementTag::Bool,
            ValueType::StringPtr => ElementTag::StringPtr,
            ValueType::Unit => ElementTag::Unit,
            ValueType::Null => ElementTag::Null,
            ValueType::Function => ElementTag::Function,
            ValueType::Struct(id) => ElementTag::Struct(id),
            ValueType::ArrayPtr | ValueType::ArrayPtrOf(_) => ElementTag::Array,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::codegen::llvm) enum ValueType {
    Double,
    Bool,
    StringPtr,
    Unit,
    Null,
    Function,
    Struct(u32),
    ArrayPtr,
    ArrayPtrOf(ElementTag),
}

impl ValueType {
    pub(in crate::codegen::llvm) fn llvm_type(self) -> &'static str {
        match self {
            ValueType::Double => "double",
            ValueType::Bool => "i1",
            ValueType::StringPtr => "i8*",
            ValueType::Unit => "i8",
            ValueType::Null => "i8*",
            ValueType::Function => "i8*",
            ValueType::Struct(_) => "i8*",
            ValueType::ArrayPtr | ValueType::ArrayPtrOf(_) => "i8*",
        }
    }

    pub(in crate::codegen::llvm) fn display_name(self) -> &'static str {
        match self {
            ValueType::Double => "Number",
            ValueType::Bool => "Boolean",
            ValueType::StringPtr => "String",
            ValueType::Unit => "Unit",
            ValueType::Null => "Null",
            ValueType::Function => "Function",
            ValueType::Struct(_) => "Struct",
            ValueType::ArrayPtr | ValueType::ArrayPtrOf(_) => "Array",
        }
    }

    pub(in crate::codegen::llvm) fn is_array(self) -> bool {
        matches!(self, ValueType::ArrayPtr | ValueType::ArrayPtrOf(_))
    }

    pub(in crate::codegen::llvm) fn array_element_type(self) -> Option<ValueType> {
        match self {
            ValueType::ArrayPtrOf(tag) => Some(tag.to_value_type()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::codegen::llvm) struct ValueRef {
    pub(in crate::codegen::llvm) value_type: ValueType,
    pub(in crate::codegen::llvm) repr: String,
}
