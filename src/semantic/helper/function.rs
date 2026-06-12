use super::{SemanticType, TypeId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    pub type_id: u32,
    pub param_names: Vec<String>,
    pub param_types: Vec<SemanticType>,
    pub return_type: SemanticType,
}

impl FunctionSignature {
    pub fn arity(&self) -> usize {
        self.param_types.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSymbol {
    pub name: String,
    pub type_id: TypeId,
    pub receiver: Option<TypeId>,
}

impl FunctionSymbol {
    pub fn new_function(name: impl Into<String>, type_id: TypeId) -> Self {
        Self {
            name: name.into(),
            type_id,
            receiver: None,
        }
    }

    pub fn new_method(name: impl Into<String>, type_id: TypeId, receiver: TypeId) -> Self {
        Self {
            name: name.into(),
            type_id,
            receiver: Some(receiver),
        }
    }

    pub const fn is_method(&self) -> bool {
        self.receiver.is_some()
    }

    pub const fn is_function(&self) -> bool {
        self.receiver.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::{FunctionSymbol, TypeId};

    #[test]
    fn detects_global_function_symbol() {
        let symbol = FunctionSymbol::new_function("sum", TypeId(10));
        assert!(symbol.is_function());
        assert!(!symbol.is_method());
        assert_eq!(symbol.receiver, None);
    }

    #[test]
    fn detects_method_symbol() {
        let receiver = TypeId(3);
        let symbol = FunctionSymbol::new_method("push", TypeId(12), receiver);
        assert!(symbol.is_method());
        assert!(!symbol.is_function());
        assert_eq!(symbol.receiver, Some(receiver));
    }
}
