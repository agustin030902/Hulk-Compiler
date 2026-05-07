#[derive(Debug, Clone)]
pub(in crate::codegen::llvm) struct VariableInfo {
    pub(in crate::codegen::llvm) ptr_name: String,
    pub(in crate::codegen::llvm) value_type: ValueType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::codegen::llvm) enum ValueType {
    Double,
    Bool,
    StringPtr,
    Unit,
    Function,
    Struct(u32),
}

impl ValueType {
    pub(in crate::codegen::llvm) fn llvm_type(self) -> &'static str {
        match self {
            ValueType::Double => "double",
            ValueType::Bool => "i1",
            ValueType::StringPtr => "i8*",
            ValueType::Unit => "i8",
            ValueType::Function => "i8*",
            ValueType::Struct(_) => "i8*",
        }
    }

    pub(in crate::codegen::llvm) fn display_name(self) -> &'static str {
        match self {
            ValueType::Double => "Number",
            ValueType::Bool => "Boolean",
            ValueType::StringPtr => "String",
            ValueType::Unit => "Unit",
            ValueType::Function => "Function",
            ValueType::Struct(_) => "Struct",
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::codegen::llvm) struct ValueRef {
    pub(in crate::codegen::llvm) value_type: ValueType,
    pub(in crate::codegen::llvm) repr: String,
}
