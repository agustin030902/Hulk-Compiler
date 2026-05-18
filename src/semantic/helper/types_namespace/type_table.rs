use super::{FunctionTypeInfo, StructTypeInfo, TypeId, TypeInfo};

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

        Self {
            types,
            number,
            boolean,
            string,
            unit,
            unknown,
        }
    }

    pub fn register_type(&mut self, info: StructTypeInfo) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(TypeInfo::Type(info));
        id
    }

    pub fn register_function(&mut self, info: FunctionTypeInfo) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(TypeInfo::Function(info));
        id
    }

    pub fn register_plain_function(&mut self, params: Vec<TypeId>, return_type: TypeId) -> TypeId {
        self.register_function(FunctionTypeInfo::new_function(params, return_type))
    }

    pub fn register_method(
        &mut self,
        receiver: TypeId,
        params: Vec<TypeId>,
        return_type: TypeId,
    ) -> TypeId {
        self.register_function(FunctionTypeInfo::new_method(receiver, params, return_type))
    }

    pub fn get(&self, id: TypeId) -> &TypeInfo {
        &self.types[id.0 as usize]
    }

    pub fn get_function(&self, id: TypeId) -> Option<&FunctionTypeInfo> {
        match self.get(id) {
            TypeInfo::Function(info) => Some(info),
            _ => None,
        }
    }

    pub fn is_method(&self, id: TypeId) -> bool {
        self.get_function(id)
            .map(FunctionTypeInfo::is_method)
            .unwrap_or(false)
    }

    pub fn is_function(&self, id: TypeId) -> bool {
        self.get_function(id)
            .map(FunctionTypeInfo::is_function)
            .unwrap_or(false)
    }
}

impl Default for TypeTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{TypeId, TypeTable};

    #[test]
    fn identifies_registered_function_vs_method() {
        let mut table = TypeTable::new();
        let function_id = table.register_plain_function(vec![table.number], table.unit);
        let method_id =
            table.register_method(TypeId(500), vec![table.number, table.string], table.boolean);

        assert!(table.is_function(function_id));
        assert!(!table.is_method(function_id));

        assert!(table.is_method(method_id));
        assert!(!table.is_function(method_id));
    }
}
