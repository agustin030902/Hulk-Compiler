use super::SemanticType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    pub type_id: u32,
    pub param_types: Vec<SemanticType>,
    pub return_type: SemanticType,
}

impl FunctionSignature {
    pub fn arity(&self) -> usize {
        self.param_types.len()
    }
}
