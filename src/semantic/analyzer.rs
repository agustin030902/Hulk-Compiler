use std::collections::HashMap;

use crate::{
    error::{CompilerError, ErrorCategory, offset_to_line_column},
    parser::expression::{FunctionDecl, Program, Span},
};

use super::{
    builtins,
    helper::{FunctionSignature, FunctionSymbol, ScopeStack, SemanticType, TypeId, TypeTable},
    pipeline::{SignatureInferencePass, SymbolCollector, TypeChecker},
};

#[derive(Debug, Default)]
pub struct SemanticAnalyzer {
    pub(super) scopes: ScopeStack<SemanticType>,
    pub(super) functions: HashMap<String, FunctionSignature>,
    pub(super) function_symbols: HashMap<String, FunctionSymbol>,
    pub(super) type_symbols: HashMap<String, TypeId>,
    pub(super) type_table: TypeTable,
    pub(super) errors: Vec<CompilerError>,
    pub(super) current_method_receiver: Option<TypeId>,
    pub(super) current_self_scope_index: Option<usize>,
    pub(super) suppress_errors: bool,
    pub(super) protocol_real_types: HashMap<String, TypeId>,
    pub(super) param_real_types: Vec<HashMap<String, TypeId>>,
    pub(super) flat_param_real_types: HashMap<String, TypeId>,
    pub(super) function_decls: HashMap<String, FunctionDecl>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn function_signatures(&self) -> &HashMap<String, FunctionSignature> {
        &self.functions
    }

    pub fn function_symbols(&self) -> &HashMap<String, FunctionSymbol> {
        &self.function_symbols
    }

    pub fn type_table(&self) -> &TypeTable {
        &self.type_table
    }

    pub fn type_symbols(&self) -> &HashMap<String, TypeId> {
        &self.type_symbols
    }

    pub fn protocol_real_types(&self) -> &HashMap<String, TypeId> {
        &self.protocol_real_types
    }

    pub fn flat_param_real_types(&self) -> &HashMap<String, TypeId> {
        &self.flat_param_real_types
    }

    pub fn current_param_real_types(&self) -> Option<&HashMap<String, TypeId>> {
        self.param_real_types.last()
    }

    pub(super) fn push_param_real_types(&mut self) {
        self.param_real_types.push(HashMap::new());
    }

    pub(super) fn pop_param_real_types(&mut self) {
        self.param_real_types.pop();
    }

    pub(super) fn bind_param_real_type(&mut self, name: String, type_id: TypeId) {
        if let Some(ctx) = self.param_real_types.last_mut() {
            ctx.insert(name.clone(), type_id);
        }
        self.flat_param_real_types.insert(name, type_id);
    }

    pub(super) fn lookup_param_real_type(&self, name: &str) -> Option<TypeId> {
        self.param_real_types
            .iter()
            .rev()
            .find_map(|ctx| ctx.get(name).copied())
    }

    pub fn analyze(&mut self, program: &Program, source: &str) -> Vec<CompilerError> {
        let inferred_signatures =
            SignatureInferencePass::infer_function_signatures(self, program, source);

        self.reset_analysis_state();
        SymbolCollector::inject_splat_protocols(self, program);
        SymbolCollector::collect_types(self, &program.types, source);
        SymbolCollector::collect_protocols(self, &program.protocols, source);
        SymbolCollector::collect_functions(self, &program.functions, source);
        SymbolCollector::collect_methods(self, &program.types, source);
        SymbolCollector::collect_protocol_methods(self, &program.protocols, source);
        SignatureInferencePass::apply_inferred_signatures(self, &inferred_signatures);

        self.start_scope_pass();
        {
            let mut checker = TypeChecker::new(self);
            checker.check_program(program, source);
        }

        SignatureInferencePass::push_unresolved_function_type_errors(self, program, source);
        SignatureInferencePass::sync_function_type_entries(self);

        self.errors.clone()
    }

    pub(super) fn reset_analysis_state(&mut self) {
        self.scopes.clear();
        self.functions.clear();
        self.function_symbols.clear();
        self.type_symbols.clear();
        self.type_table = TypeTable::new();
        self.type_symbols
            .insert("Object".to_string(), self.type_table.object);
        builtins::register_builtin_iterable(self);
        builtins::register_builtin_range(self);
        self.errors.clear();
        self.current_method_receiver = None;
        self.current_self_scope_index = None;
        self.suppress_errors = false;
        self.protocol_real_types.clear();
        self.param_real_types.clear();
        self.flat_param_real_types.clear();
        self.function_decls.clear();
    }

    pub(super) fn start_scope_pass(&mut self) {
        self.scopes.clear();
        self.push_scope();
    }

    pub(super) fn push_type_error(&mut self, span: Span, source: &str, message: String) {
        if self.suppress_errors {
            return;
        }
        let (line, column) = offset_to_line_column(source, span.start);
        self.errors.push(CompilerError::new(
            ErrorCategory::Type,
            message,
            line,
            column,
        ));
    }

    pub(super) fn push_semantic_error(&mut self, span: Span, source: &str, message: String) {
        if self.suppress_errors {
            return;
        }
        let (line, column) = offset_to_line_column(source, span.start);
        self.errors.push(CompilerError::new(
            ErrorCategory::Semantic,
            message,
            line,
            column,
        ));
    }

    pub(super) fn push_scope(&mut self) {
        self.scopes.push();
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn bind_current_scope(&mut self, name: String, value_type: SemanticType) {
        self.scopes.insert_current(name, value_type);
    }

    pub(super) fn assign_in_scope(
        &mut self,
        scope_index: usize,
        name: String,
        value_type: SemanticType,
    ) {
        self.scopes.assign_at(scope_index, name, value_type);
    }

    pub(super) fn lookup(&self, name: &str) -> Option<SemanticType> {
        self.scopes.lookup(name)
    }

    pub(super) fn find_scope_index(&self, name: &str) -> Option<usize> {
        self.scopes.find_scope_index(name)
    }

    pub(super) fn lookup_with_scope_index(&self, name: &str) -> Option<(usize, SemanticType)> {
        self.scopes.lookup_with_index(name)
    }

    pub(super) fn is_declared_in_current_scope(&self, name: &str) -> bool {
        self.scopes.contains_in_current(name)
    }
}
