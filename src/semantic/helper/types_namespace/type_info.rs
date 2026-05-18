use super::TypeId;

#[derive(Debug)]
pub enum TypeInfo {
    // Builtins
    Number,
    Boolean,
    String,
    Unit,

    // User types
    Type(StructTypeInfo),
    Function(FunctionTypeInfo),
}

#[derive(Debug)]
pub struct StructTypeInfo {
    pub name: String,
    pub fields: Vec<(String, TypeId)>,
    pub parent: Option<TypeId>,
}

#[derive(Debug)]
pub struct FunctionTypeInfo {
    pub params: Vec<TypeId>,
    pub return_type: TypeId,
}