use crate::{error::CompilerError, parser::expression::Program};

pub mod llvm;

pub trait CodegenBackend {
    fn generate(&mut self, program: &Program) -> Result<String, Vec<CompilerError>>;
}
