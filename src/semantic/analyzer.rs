use std::collections::HashMap;

use crate::{
    error::{CompilerError, ErrorCategory, offset_to_line_column},
    parser::expression::{Program, Span},
};

use super::helper::SemanticType;

#[derive(Debug, Default)]
pub struct SemanticAnalyzer {
    pub(super) scopes: Vec<HashMap<String, SemanticType>>,
    pub(super) errors: Vec<CompilerError>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn analyze(&mut self, program: &Program, source: &str) -> Vec<CompilerError> {
        self.scopes.clear();
        self.errors.clear();
        self.push_scope();

        for statement in &program.statements {
            let _ = self.check_statement(statement, source);
        }

        self.errors.clone()
    }

    pub(super) fn push_type_error(&mut self, span: Span, source: &str, message: String) {
        let (line, column) = offset_to_line_column(source, span.start);
        self.errors.push(CompilerError::new(
            ErrorCategory::Type,
            message,
            line,
            column,
        ));
    }

    pub(super) fn push_semantic_error(&mut self, span: Span, source: &str, message: String) {
        let (line, column) = offset_to_line_column(source, span.start);
        self.errors.push(CompilerError::new(
            ErrorCategory::Semantic,
            message,
            line,
            column,
        ));
    }

    pub(super) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn current_scope_mut(&mut self) -> &mut HashMap<String, SemanticType> {
        self.scopes
            .last_mut()
            .expect("at least one scope should be present")
    }

    pub(super) fn assign_in_scope(
        &mut self,
        scope_index: usize,
        name: String,
        value_type: SemanticType,
    ) {
        self.scopes[scope_index].insert(name, value_type);
    }

    pub(super) fn lookup(&self, name: &str) -> Option<SemanticType> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    pub(super) fn find_scope_index(&self, name: &str) -> Option<usize> {
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(idx, scope)| scope.contains_key(name).then_some(idx))
    }

    pub(super) fn is_declared_in_current_scope(&self, name: &str) -> bool {
        self.scopes
            .last()
            .map(|scope| scope.contains_key(name))
            .unwrap_or(false)
    }
}
