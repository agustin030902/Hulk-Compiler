use super::{FunctionTypeInfo, StructTypeInfo, TypeId, TypeInfo};

#[derive(Debug)]
pub struct TypeTable {
    types: Vec<TypeInfo>,

    pub number: TypeId,
    pub boolean: TypeId,
    pub string: TypeId,
    pub unit: TypeId,
    pub null: TypeId,
    pub unknown: TypeId,
    pub object: TypeId,
    pub iterable: TypeId,
    pub range: TypeId,
    pub enumerable: TypeId,
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
        let null = push(TypeInfo::Null);
        let unknown = push(TypeInfo::Unknown);
        let object = push(TypeInfo::Type(StructTypeInfo {
            name: "Object".to_string(),
            constructor_params: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            parent: None,
            is_interface: false,
        }));

        let iterable = push(TypeInfo::Type(StructTypeInfo {
            name: "Iterable".to_string(),
            constructor_params: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            parent: None,
            is_interface: true,
        }));

        let range = push(TypeInfo::Type(StructTypeInfo {
            name: "Range".to_string(),
            constructor_params: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            parent: Some(object),
            is_interface: false,
        }));

        let enumerable = push(TypeInfo::Type(StructTypeInfo {
            name: "Enumerable".to_string(),
            constructor_params: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            parent: None,
            is_interface: true,
        }));

        Self {
            types,
            number,
            boolean,
            string,
            unit,
            null,
            unknown,
            object,
            iterable,
            range,
            enumerable,
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

    /// Interna un tipo función estructural `(params) -> ret` (sin receptor).
    /// Dos anotaciones `(Number)->Number` comparten el mismo TypeId.
    pub fn function_type_of(&mut self, params: Vec<TypeId>, return_type: TypeId) -> TypeId {
        for (index, info) in self.types.iter().enumerate() {
            if let TypeInfo::Function(existing) = info {
                if existing.receiver.is_none()
                    && existing.params == params
                    && existing.return_type == return_type
                {
                    return TypeId(index as u32);
                }
            }
        }
        self.register_plain_function(params, return_type)
    }

    /// Todas las entradas de tipo función sin receptor: (id, params, retorno).
    pub fn plain_function_entries(&self) -> Vec<(TypeId, Vec<TypeId>, TypeId)> {
        self.types
            .iter()
            .enumerate()
            .filter_map(|(index, info)| match info {
                TypeInfo::Function(f) if f.receiver.is_none() => {
                    Some((TypeId(index as u32), f.params.clone(), f.return_type))
                }
                _ => None,
            })
            .collect()
    }

    /// Interna el tipo de arreglo `elem[]`: si ya existe una entrada con el
    /// mismo tipo de elemento la reutiliza, de modo que dos `Number[]` sean
    /// el mismo TypeId (la igualdad de tipos de arreglo es estructural).
    pub fn array_of(&mut self, elem: TypeId) -> TypeId {
        for (index, info) in self.types.iter().enumerate() {
            if let TypeInfo::Array { elem: existing } = info {
                if *existing == elem {
                    return TypeId(index as u32);
                }
            }
        }
        let id = TypeId(self.types.len() as u32);
        self.types.push(TypeInfo::Array { elem });
        id
    }

    pub fn get_array_elem(&self, id: TypeId) -> Option<TypeId> {
        match self.get(id) {
            TypeInfo::Array { elem } => Some(*elem),
            _ => None,
        }
    }

    /// Todas las entradas de arreglo registradas: (id del arreglo, id del elemento).
    pub fn array_entries(&self) -> Vec<(TypeId, TypeId)> {
        self.types
            .iter()
            .enumerate()
            .filter_map(|(index, info)| match info {
                TypeInfo::Array { elem } => Some((TypeId(index as u32), *elem)),
                _ => None,
            })
            .collect()
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

    pub fn get_mut(&mut self, id: TypeId) -> &mut TypeInfo {
        &mut self.types[id.0 as usize]
    }

    pub fn get_function(&self, id: TypeId) -> Option<&FunctionTypeInfo> {
        match self.get(id) {
            TypeInfo::Function(info) => Some(info),
            _ => None,
        }
    }

    pub fn get_function_mut(&mut self, id: TypeId) -> Option<&mut FunctionTypeInfo> {
        match self.get_mut(id) {
            TypeInfo::Function(info) => Some(info),
            _ => None,
        }
    }

    pub fn get_struct(&self, id: TypeId) -> Option<&StructTypeInfo> {
        match self.get(id) {
            TypeInfo::Type(info) => Some(info),
            _ => None,
        }
    }

    pub fn get_struct_mut(&mut self, id: TypeId) -> Option<&mut StructTypeInfo> {
        match self.get_mut(id) {
            TypeInfo::Type(info) => Some(info),
            _ => None,
        }
    }

    // Wrappers usados por la suite de tests.
    #[allow(dead_code)]
    pub fn is_method(&self, id: TypeId) -> bool {
        self.get_function(id)
            .map(FunctionTypeInfo::is_method)
            .unwrap_or(false)
    }

    #[allow(dead_code)]
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
