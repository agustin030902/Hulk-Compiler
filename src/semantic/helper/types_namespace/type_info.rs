#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeInfo {
    Number,
    Boolean,
    String,
    Unit,
    Null,
    Unknown,
    Type(StructTypeInfo),
    Function(FunctionTypeInfo),
    /// Arreglo homogéneo `T[]`; `elem` es el tipo de los elementos.
    Array { elem: TypeId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructTypeInfo {
    pub name: String,
    pub constructor_params: Vec<(String, TypeId)>,
    pub fields: Vec<(String, TypeId)>,
    pub methods: Vec<(String, TypeId)>,
    pub parent: Option<TypeId>,
    pub is_interface: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionTypeInfo {
    pub receiver: Option<TypeId>,
    pub params: Vec<TypeId>,
    pub return_type: TypeId,
}

impl FunctionTypeInfo {
    pub fn new_function(params: Vec<TypeId>, return_type: TypeId) -> Self {
        Self {
            receiver: None,
            params,
            return_type,
        }
    }

    pub fn new_method(receiver: TypeId, params: Vec<TypeId>, return_type: TypeId) -> Self {
        Self {
            receiver: Some(receiver),
            params,
            return_type,
        }
    }

    pub const fn is_method(&self) -> bool {
        self.receiver.is_some()
    }

    #[allow(dead_code)]
    pub const fn is_function(&self) -> bool {
        self.receiver.is_none()
    }
}
