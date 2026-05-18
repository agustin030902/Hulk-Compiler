use super::TypeId;
    
#[derive(Debug, Clone)]

pub struct FunctionSymbol {
    pub name: String,
    pub type_id: TypeId,
    /// None = función global
    /// Some(TypeId) = método de ese tipo
    pub receiver: Option<TypeId>,
}