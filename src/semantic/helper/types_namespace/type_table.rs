use super::{TypeId, TypeInfo};

pub struct TypeTable {
    types: Vec<TypeInfo>,

    pub number: TypeId,
    pub boolean: TypeId,
    pub string: TypeId,
    pub unit: TypeId,
    pub unknown: TypeId,
}

impl TypeTable {
    pub fn new() -> Self {
        let mut types = Vec::new();

        let mut push = |info| {
            let id = TypeId(types.len() as u32);
            types.push(info);
            id
        };

        let number = push(TypeInfo::Number);
        let boolean = push(TypeInfo::Boolean);
        let string = push(TypeInfo::String);
        let unit = push(TypeInfo::Unit);
        let unknown = push(TypeInfo::Unknown);

        Self { types, number, boolean, string, unit, unknown }
    }

    pub fn register_type(&mut self, info: super::StructTypeInfo) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(TypeInfo::Type(info));
        id
    }

    pub fn register_function(&mut self, info: super::FunctionTypeInfo) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(TypeInfo::Function(info));
        id
    }

    pub fn get(&self, id: TypeId) -> &TypeInfo {
        &self.types[id.0 as usize]
    }
}