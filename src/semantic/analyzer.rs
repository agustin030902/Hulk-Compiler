use std::collections::{HashMap, HashSet};

use crate::{
    error::{CompilerError, ErrorCategory, offset_to_line_column},
    parser::expression::{FunctionDecl, Program, Span},
};

use super::helper::SemanticType;

#[derive(Debug, Default)]
pub struct SemanticAnalyzer {
    pub(super) scopes: Vec<HashMap<String, SemanticType>>,
    pub(super) functions: HashMap<String, usize>,
    pub(super) errors: Vec<CompilerError>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn analyze(&mut self, program: &Program, source: &str) -> Vec<CompilerError> {
        self.scopes.clear();
        self.functions.clear();
        self.errors.clear();
        self.push_scope();

        self.collect_functions(&program.functions, source);
        for function in &program.functions {
            self.check_function_decl(function, source);
        }

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

    fn collect_functions(&mut self, functions: &[FunctionDecl], source: &str) {
        for function in functions {
            if self.functions.contains_key(&function.name) {
                self.push_semantic_error(
                    function.name_span,
                    source,
                    format!("Function '{}' redeclared.", function.name),
                );
                continue;
            }

            self.functions
                .insert(function.name.clone(), function.params.len());
        }
    }

    fn check_function_decl(&mut self, function: &FunctionDecl, source: &str) {
        let mut param_names = HashSet::new();
        self.push_scope();

        for param in &function.params {
            if !param_names.insert(param.name.clone()) {
                self.push_semantic_error(
                    param.span,
                    source,
                    format!(
                        "Parameter '{}' redeclared in function '{}'.",
                        param.name, function.name
                    ),
                );
                continue;
            }

            self.current_scope_mut()
                .insert(param.name.clone(), SemanticType::Unknown);
        }

        let _ = self.check_expr(&function.body, source);
        self.pop_scope();
    }
}
