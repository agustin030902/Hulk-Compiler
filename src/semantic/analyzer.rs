use std::collections::{HashMap, HashSet};

use crate::{
    error::{CompilerError, ErrorCategory, offset_to_line_column},
    parser::expression::{
        BlockExpr, Expr, FunctionDecl, IfExpr, LetInExpr, MethodDecl, Program, Span, Statement,
        TypeAnnotation, TypeDecl,
    },
};

use super::helper::{
    FunctionSignature, FunctionSymbol, ScopeStack, SemanticType, TypeId, TypeTable,
};

const MAX_INFERENCE_PASSES: usize = 8;

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
    suppress_errors: bool,
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

    pub fn analyze(&mut self, program: &Program, source: &str) -> Vec<CompilerError> {
        let inferred_signatures = self.infer_function_signatures(program, source);

        self.reset_analysis_state();
        self.collect_types(&program.types, source);
        self.collect_functions(&program.functions, source);
        self.collect_methods(&program.types, source);
        self.apply_inferred_signatures(&inferred_signatures);
        self.start_scope_pass();

        for type_decl in &program.types {
            self.check_type_decl(type_decl, source);
        }

        for function in &program.functions {
            self.check_function_decl(function, source);
        }

        for statement in &program.statements {
            let _ = self.check_statement(statement, source);
        }

        self.push_unresolved_function_type_errors(program, source);
        self.sync_function_type_entries();

        self.errors.clone()
    }

    fn infer_function_signatures(
        &mut self,
        program: &Program,
        source: &str,
    ) -> HashMap<String, FunctionSignature> {
        self.reset_analysis_state();
        self.suppress_errors = true;
        self.collect_types(&program.types, source);
        self.collect_functions(&program.functions, source);
        self.collect_methods(&program.types, source);

        for _ in 0..MAX_INFERENCE_PASSES {
            let before = self.functions.clone();
            self.start_scope_pass();

            for type_decl in &program.types {
                self.check_type_decl(type_decl, source);
            }

            for function in &program.functions {
                self.check_function_decl(function, source);
            }

            for statement in &program.statements {
                let _ = self.check_statement(statement, source);
            }

            if self.functions == before {
                break;
            }
        }

        self.suppress_errors = false;
        self.functions.clone()
    }

    fn reset_analysis_state(&mut self) {
        self.scopes.clear();
        self.functions.clear();
        self.function_symbols.clear();
        self.type_symbols.clear();
        self.type_table = TypeTable::new();
        self.errors.clear();
        self.current_method_receiver = None;
        self.current_self_scope_index = None;
        self.suppress_errors = false;
    }

    fn start_scope_pass(&mut self) {
        self.scopes.clear();
        self.push_scope();
    }

    fn apply_inferred_signatures(&mut self, inferred: &HashMap<String, FunctionSignature>) {
        for (name, signature) in inferred {
            if let Some(entry) = self.functions.get_mut(name) {
                entry.param_types = signature.param_types.clone();
                entry.return_type = signature.return_type;
            }
        }
        self.sync_function_type_entries();
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

    pub(super) fn resolve_annotation_type(
        &mut self,
        annotation: &TypeAnnotation,
        source: &str,
    ) -> Option<SemanticType> {
        let Some(annotation_type) = self.resolve_named_type(&annotation.name) else {
            self.push_semantic_error(
                annotation.span,
                source,
                format!(
                    "Unknown type annotation '{}'. Expected one of: {}.",
                    annotation.name,
                    self.known_annotation_names()
                ),
            );
            return None;
        };

        Some(annotation_type)
    }

    pub(super) fn resolve_named_type(&self, name: &str) -> Option<SemanticType> {
        if let Some(primitive) = SemanticType::from_annotation_name(name) {
            return Some(primitive);
        }

        self.type_symbols
            .get(name)
            .copied()
            .map(|type_id| SemanticType::Struct(type_id.0))
    }

    fn known_annotation_names(&self) -> String {
        let mut names = vec!["Number", "Boolean", "String", "Unit"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();

        let mut user_types = self.type_symbols.keys().cloned().collect::<Vec<_>>();
        user_types.sort();
        names.extend(user_types);

        names.join(", ")
    }

    pub(super) fn check_annotated_initializer(
        &mut self,
        variable_name: &str,
        value: &Expr,
        annotation_type: SemanticType,
        annotation_span: Span,
        source: &str,
    ) -> SemanticType {
        let mut value_type = self
            .check_expr(value, source)
            .unwrap_or(SemanticType::Unknown);

        if value_type == SemanticType::Unknown {
            value_type = self.constrain_expr_type(value, annotation_type, source);
        }

        if value_type != SemanticType::Unknown && value_type != annotation_type {
            self.push_type_error(
                annotation_span,
                source,
                format!(
                    "Type annotation for variable '{}' expects {}, but initializer is {}.",
                    variable_name,
                    annotation_type.display_name(),
                    value_type.display_name()
                ),
            );
        }

        annotation_type
    }

    pub(super) fn constrain_expr_type(
        &mut self,
        expr: &Expr,
        expected: SemanticType,
        source: &str,
    ) -> SemanticType {
        match expr {
            Expr::Variable { name, span } => {
                self.constrain_variable_type(name, expected, *span, source)
            }
            Expr::FunctionCall(call) => {
                self.constrain_function_return_type(&call.name, expected, call.name_span, source)
            }
            Expr::MethodCall(call) => {
                let receiver_type = self
                    .check_expr(&call.receiver, source)
                    .unwrap_or(SemanticType::Unknown);
                if let SemanticType::Struct(type_id) = receiver_type
                    && let Some(key) =
                        self.resolve_method_symbol_key(TypeId(type_id), &call.method_name)
                {
                    return self.constrain_function_return_type(
                        &key,
                        expected,
                        call.method_name_span,
                        source,
                    );
                }
                expected
            }
            Expr::Block(block) => self.constrain_block_result_type(block, expected, source),
            Expr::LetIn(let_in) => self.constrain_let_in_result_type(let_in, expected, source),
            Expr::If(if_expr) => self.constrain_if_result_type(if_expr, expected, source),
            _ => expected,
        }
    }

    fn constrain_statement_result_type(
        &mut self,
        statement: &Statement,
        expected: SemanticType,
        source: &str,
    ) -> SemanticType {
        match statement {
            Statement::Expr { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. } => self.constrain_expr_type(value, expected, source),
            Statement::Print { .. } => SemanticType::Unit,
        }
    }

    fn constrain_block_result_type(
        &mut self,
        block: &BlockExpr,
        expected: SemanticType,
        source: &str,
    ) -> SemanticType {
        let Some(last_statement) = block.statements.last() else {
            return SemanticType::Unit;
        };
        self.constrain_statement_result_type(last_statement, expected, source)
    }

    fn constrain_let_in_result_type(
        &mut self,
        let_in: &LetInExpr,
        expected: SemanticType,
        source: &str,
    ) -> SemanticType {
        self.constrain_expr_type(&let_in.body, expected, source)
    }

    fn constrain_if_result_type(
        &mut self,
        if_expr: &IfExpr,
        expected: SemanticType,
        source: &str,
    ) -> SemanticType {
        let _ = self.constrain_expr_type(&if_expr.condition, SemanticType::Boolean, source);
        let _ = self.constrain_expr_type(&if_expr.then_branch, expected, source);
        for branch in &if_expr.elif_branches {
            let _ = self.constrain_expr_type(&branch.condition, SemanticType::Boolean, source);
            let _ = self.constrain_expr_type(&branch.body, expected, source);
        }
        let _ = self.constrain_expr_type(&if_expr.else_branch, expected, source);
        expected
    }

    pub(super) fn constrain_variable_type(
        &mut self,
        name: &str,
        expected: SemanticType,
        span: Span,
        source: &str,
    ) -> SemanticType {
        let Some((scope_index, current_type)) = self.lookup_with_scope_index(name) else {
            return SemanticType::Unknown;
        };

        match merge_types(current_type, expected) {
            Ok(merged) => {
                if merged != current_type {
                    self.assign_in_scope(scope_index, name.to_string(), merged);
                }
                merged
            }
            Err((left, right)) => {
                self.push_type_error(
                    span,
                    source,
                    format!(
                        "Type inference conflict for variable '{}': {} vs {}.",
                        name,
                        left.display_name(),
                        right.display_name()
                    ),
                );
                current_type
            }
        }
    }

    pub(super) fn constrain_function_param_type(
        &mut self,
        function_name: &str,
        param_index: usize,
        inferred: SemanticType,
        span: Span,
        source: &str,
    ) -> SemanticType {
        let Some(current_type) = self
            .functions
            .get(function_name)
            .and_then(|signature| signature.param_types.get(param_index).copied())
        else {
            return SemanticType::Unknown;
        };

        match merge_types(current_type, inferred) {
            Ok(merged) => {
                if merged != current_type
                    && let Some(signature) = self.functions.get_mut(function_name)
                {
                    signature.param_types[param_index] = merged;
                }
                merged
            }
            Err((left, right)) => {
                self.push_type_error(
                    span,
                    source,
                    format!(
                        "Function '{}' argument #{} has conflicting types: {} vs {}.",
                        function_name,
                        param_index + 1,
                        left.display_name(),
                        right.display_name()
                    ),
                );
                current_type
            }
        }
    }

    pub(super) fn constrain_function_return_type(
        &mut self,
        function_name: &str,
        inferred: SemanticType,
        span: Span,
        source: &str,
    ) -> SemanticType {
        let Some(current_type) = self
            .functions
            .get(function_name)
            .map(|signature| signature.return_type)
        else {
            return SemanticType::Unknown;
        };

        match merge_types(current_type, inferred) {
            Ok(merged) => {
                if merged != current_type
                    && let Some(signature) = self.functions.get_mut(function_name)
                {
                    signature.return_type = merged;
                }
                merged
            }
            Err((left, right)) => {
                self.push_type_error(
                    span,
                    source,
                    format!(
                        "Function '{}' return type conflict: {} vs {}.",
                        function_name,
                        left.display_name(),
                        right.display_name()
                    ),
                );
                current_type
            }
        }
    }

    fn collect_types(&mut self, type_decls: &[TypeDecl], source: &str) {
        for type_decl in type_decls {
            if SemanticType::from_annotation_name(&type_decl.name).is_some() {
                self.push_semantic_error(
                    type_decl.name_span,
                    source,
                    format!(
                        "Type '{}' cannot be declared because the name is reserved.",
                        type_decl.name
                    ),
                );
                continue;
            }

            if self.type_symbols.contains_key(&type_decl.name) {
                self.push_semantic_error(
                    type_decl.name_span,
                    source,
                    format!("Type '{}' redeclared.", type_decl.name),
                );
                continue;
            }

            let type_id = self
                .type_table
                .register_type(super::helper::StructTypeInfo {
                    name: type_decl.name.clone(),
                    constructor_params: Vec::new(),
                    fields: Vec::new(),
                    methods: Vec::new(),
                    parent: None,
                });
            self.type_symbols.insert(type_decl.name.clone(), type_id);
        }

        for type_decl in type_decls {
            let Some(type_id) = self.type_symbols.get(&type_decl.name).copied() else {
                continue;
            };

            let mut constructor_params = Vec::with_capacity(type_decl.params.len());
            for param in &type_decl.params {
                let param_type = param
                    .type_annotation
                    .as_ref()
                    .and_then(|annotation| self.resolve_annotation_type(annotation, source))
                    .unwrap_or(SemanticType::Unknown);
                constructor_params.push((
                    param.name.clone(),
                    self.semantic_type_to_type_id(param_type),
                ));
            }

            if let Some(struct_info) = self.type_table.get_struct_mut(type_id) {
                struct_info.constructor_params = constructor_params;
            }
        }
    }

    fn collect_methods(&mut self, type_decls: &[TypeDecl], source: &str) {
        for type_decl in type_decls {
            let Some(receiver_type_id) = self.type_symbols.get(&type_decl.name).copied() else {
                continue;
            };

            for method in &type_decl.methods {
                let key = Self::method_symbol_key(receiver_type_id, &method.name);
                if self.function_symbols.contains_key(&key) {
                    self.push_semantic_error(
                        method.name_span,
                        source,
                        format!(
                            "Method '{}' redeclared in type '{}'.",
                            method.name, type_decl.name
                        ),
                    );
                    continue;
                }

                let param_types = method
                    .params
                    .iter()
                    .map(|param| {
                        param
                            .type_annotation
                            .as_ref()
                            .and_then(|annotation| self.resolve_annotation_type(annotation, source))
                            .unwrap_or(SemanticType::Unknown)
                    })
                    .collect::<Vec<_>>();

                let return_type = method
                    .return_type_annotation
                    .as_ref()
                    .and_then(|annotation| self.resolve_annotation_type(annotation, source))
                    .unwrap_or(SemanticType::Unknown);

                let param_type_ids = param_types
                    .iter()
                    .copied()
                    .map(|semantic_type| self.semantic_type_to_type_id(semantic_type))
                    .collect::<Vec<_>>();
                let return_type_id = self.semantic_type_to_type_id(return_type);
                let method_type_id = self.type_table.register_method(
                    receiver_type_id,
                    param_type_ids,
                    return_type_id,
                );

                self.function_symbols.insert(
                    key.clone(),
                    FunctionSymbol::new_method(
                        method.name.clone(),
                        method_type_id,
                        receiver_type_id,
                    ),
                );
                self.functions.insert(
                    key.clone(),
                    FunctionSignature {
                        type_id: method_type_id.0,
                        param_types,
                        return_type,
                    },
                );

                if let Some(info) = self.type_table.get_struct_mut(receiver_type_id) {
                    info.methods.push((method.name.clone(), method_type_id));
                }
            }
        }
    }

    fn collect_functions(&mut self, functions: &[FunctionDecl], source: &str) {
        for function in functions {
            if self.function_symbols.contains_key(&function.name) {
                self.push_semantic_error(
                    function.name_span,
                    source,
                    format!("Function '{}' redeclared.", function.name),
                );
                continue;
            }

            let param_types = function
                .params
                .iter()
                .map(|param| {
                    param
                        .type_annotation
                        .as_ref()
                        .and_then(|annotation| self.resolve_annotation_type(annotation, source))
                        .unwrap_or(SemanticType::Unknown)
                })
                .collect::<Vec<_>>();

            let return_type = function
                .return_type_annotation
                .as_ref()
                .and_then(|annotation| self.resolve_annotation_type(annotation, source))
                .unwrap_or(SemanticType::Unknown);

            let param_type_ids = param_types
                .iter()
                .copied()
                .map(|semantic_type| self.semantic_type_to_type_id(semantic_type))
                .collect::<Vec<_>>();
            let return_type_id = self.semantic_type_to_type_id(return_type);
            let function_type_id = self
                .type_table
                .register_plain_function(param_type_ids, return_type_id);

            let signature = FunctionSignature {
                type_id: function_type_id.0,
                param_types,
                return_type,
            };
            self.function_symbols.insert(
                function.name.clone(),
                FunctionSymbol::new_function(function.name.clone(), function_type_id),
            );
            self.functions.insert(function.name.clone(), signature);
        }
    }

    fn check_type_decl(&mut self, type_decl: &TypeDecl, source: &str) {
        let Some(type_id) = self.type_symbols.get(&type_decl.name).copied() else {
            return;
        };

        self.push_scope();
        let mut ctor_param_names = HashSet::new();

        let constructor_types = self
            .type_table
            .get_struct(type_id)
            .map(|info| info.constructor_params.clone())
            .unwrap_or_default();

        for (index, param) in type_decl.params.iter().enumerate() {
            if !ctor_param_names.insert(param.name.clone()) {
                self.push_semantic_error(
                    param.span,
                    source,
                    format!(
                        "Constructor parameter '{}' redeclared in type '{}'.",
                        param.name, type_decl.name
                    ),
                );
                continue;
            }

            let semantic_type = constructor_types
                .get(index)
                .map(|(_, type_id)| self.type_id_to_semantic_type(*type_id))
                .unwrap_or(SemanticType::Unknown);
            self.bind_current_scope(param.name.clone(), semantic_type);
        }

        let mut fields = Vec::with_capacity(type_decl.attributes.len());
        let mut field_names = HashSet::new();

        for attribute in &type_decl.attributes {
            if !field_names.insert(attribute.name.clone()) {
                self.push_semantic_error(
                    attribute.name_span,
                    source,
                    format!(
                        "Attribute '{}' redeclared in type '{}'.",
                        attribute.name, type_decl.name
                    ),
                );
                continue;
            }

            let value_type = self
                .check_expr(&attribute.value, source)
                .unwrap_or(SemanticType::Unknown);
            fields.push((
                attribute.name.clone(),
                self.semantic_type_to_type_id(value_type),
            ));
        }

        self.pop_scope();

        if let Some(info) = self.type_table.get_struct_mut(type_id) {
            info.fields = fields;
        }

        for method in &type_decl.methods {
            self.check_method_decl(type_id, &type_decl.name, method, source);
        }
    }

    fn check_method_decl(
        &mut self,
        receiver_type_id: TypeId,
        receiver_type_name: &str,
        method: &MethodDecl,
        source: &str,
    ) {
        let key = Self::method_symbol_key(receiver_type_id, &method.name);
        let mut param_names = HashSet::new();

        let param_types = self
            .functions
            .get(&key)
            .map(|signature| signature.param_types.clone())
            .unwrap_or_else(|| vec![SemanticType::Unknown; method.params.len()]);

        let previous_receiver = self.current_method_receiver;
        let previous_self_scope = self.current_self_scope_index;

        self.push_scope();
        self.bind_current_scope("self".to_string(), SemanticType::Struct(receiver_type_id.0));
        self.current_method_receiver = Some(receiver_type_id);
        self.current_self_scope_index = self.find_scope_index("self");

        self.push_scope();

        for (index, param) in method.params.iter().enumerate() {
            if !param_names.insert(param.name.clone()) {
                self.push_semantic_error(
                    param.span,
                    source,
                    format!(
                        "Parameter '{}' redeclared in method '{}.{}'.",
                        param.name, receiver_type_name, method.name
                    ),
                );
                continue;
            }

            let param_type = param_types
                .get(index)
                .copied()
                .unwrap_or(SemanticType::Unknown);
            self.bind_current_scope(param.name.clone(), param_type);
        }

        let expected_return_type = self
            .functions
            .get(&key)
            .map(|signature| signature.return_type)
            .unwrap_or(SemanticType::Unknown);

        let mut body_type = self
            .check_expr(&method.body, source)
            .unwrap_or(SemanticType::Unknown);
        if body_type == SemanticType::Unknown && expected_return_type != SemanticType::Unknown {
            body_type = self.constrain_expr_type(&method.body, expected_return_type, source);
        }

        let inferred_param_types = method
            .params
            .iter()
            .map(|param| self.lookup(&param.name).unwrap_or(SemanticType::Unknown))
            .collect::<Vec<_>>();

        self.pop_scope();
        self.pop_scope();
        self.current_method_receiver = previous_receiver;
        self.current_self_scope_index = previous_self_scope;

        for (index, (param, inferred_type)) in method
            .params
            .iter()
            .zip(inferred_param_types.iter().copied())
            .enumerate()
        {
            let _ =
                self.constrain_function_param_type(&key, index, inferred_type, param.span, source);
        }

        let _ = self.constrain_function_return_type(&key, body_type, method.span, source);
    }

    fn check_function_decl(&mut self, function: &FunctionDecl, source: &str) {
        let mut param_names = HashSet::new();
        let param_types = self
            .functions
            .get(&function.name)
            .map(|signature| signature.param_types.clone())
            .unwrap_or_else(|| vec![SemanticType::Unknown; function.params.len()]);

        self.push_scope();

        for (index, param) in function.params.iter().enumerate() {
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

            let param_type = param_types
                .get(index)
                .copied()
                .unwrap_or(SemanticType::Unknown);
            self.bind_current_scope(param.name.clone(), param_type);
        }

        let expected_return_type = self
            .functions
            .get(&function.name)
            .map(|signature| signature.return_type)
            .unwrap_or(SemanticType::Unknown);

        let mut body_type = self
            .check_expr(&function.body, source)
            .unwrap_or(SemanticType::Unknown);
        if body_type == SemanticType::Unknown && expected_return_type != SemanticType::Unknown {
            body_type = self.constrain_expr_type(&function.body, expected_return_type, source);
        }

        let inferred_param_types = function
            .params
            .iter()
            .map(|param| self.lookup(&param.name).unwrap_or(SemanticType::Unknown))
            .collect::<Vec<_>>();

        self.pop_scope();

        for (index, (param, inferred_type)) in function
            .params
            .iter()
            .zip(inferred_param_types.iter().copied())
            .enumerate()
        {
            let _ = self.constrain_function_param_type(
                &function.name,
                index,
                inferred_type,
                param.span,
                source,
            );
        }

        let _ =
            self.constrain_function_return_type(&function.name, body_type, function.span, source);
    }

    pub(super) fn method_symbol_key(receiver: TypeId, method_name: &str) -> String {
        format!("type#{}::{}", receiver.0, method_name)
    }

    pub(super) fn resolve_method_symbol_key(
        &self,
        receiver: TypeId,
        method_name: &str,
    ) -> Option<String> {
        let mut cursor = Some(receiver);
        while let Some(current) = cursor {
            let key = Self::method_symbol_key(current, method_name);
            if self.function_symbols.contains_key(&key) {
                return Some(key);
            }
            cursor = self
                .type_table
                .get_struct(current)
                .and_then(|info| info.parent);
        }

        None
    }

    pub(super) fn lookup_field_type_id(
        &self,
        receiver: TypeId,
        field_name: &str,
    ) -> Option<TypeId> {
        self.type_table
            .get_struct(receiver)
            .and_then(|info| info.fields.iter().find(|(name, _)| name == field_name))
            .map(|(_, type_id)| *type_id)
    }

    pub(super) fn can_access_private_field(&self, receiver: TypeId) -> bool {
        self.current_method_receiver == Some(receiver)
    }

    pub(super) fn is_self_binding(&self, name: &str, scope_index: usize) -> bool {
        name == "self" && self.current_self_scope_index == Some(scope_index)
    }

    pub(super) fn semantic_type_to_type_id(&self, semantic_type: SemanticType) -> TypeId {
        match semantic_type {
            SemanticType::Number => self.type_table.number,
            SemanticType::Boolean => self.type_table.boolean,
            SemanticType::String => self.type_table.string,
            SemanticType::Unit => self.type_table.unit,
            SemanticType::Unknown => self.type_table.unknown,
            SemanticType::Function(type_id) | SemanticType::Struct(type_id) => TypeId(type_id),
        }
    }

    pub(super) fn type_id_to_semantic_type(&self, type_id: TypeId) -> SemanticType {
        if type_id == self.type_table.number {
            return SemanticType::Number;
        }
        if type_id == self.type_table.boolean {
            return SemanticType::Boolean;
        }
        if type_id == self.type_table.string {
            return SemanticType::String;
        }
        if type_id == self.type_table.unit {
            return SemanticType::Unit;
        }
        if type_id == self.type_table.unknown {
            return SemanticType::Unknown;
        }

        match self.type_table.get(type_id) {
            super::helper::TypeInfo::Function(_) => SemanticType::Function(type_id.0),
            super::helper::TypeInfo::Type(_) => SemanticType::Struct(type_id.0),
            _ => SemanticType::Unknown,
        }
    }

    fn sync_function_type_entries(&mut self) {
        let function_names = self
            .function_symbols
            .keys()
            .cloned()
            .collect::<Vec<String>>();

        for name in function_names {
            let Some(symbol) = self.function_symbols.get(&name) else {
                continue;
            };
            let Some(signature) = self.functions.get(&name) else {
                continue;
            };

            let param_type_ids = signature
                .param_types
                .iter()
                .copied()
                .map(|semantic_type| self.semantic_type_to_type_id(semantic_type))
                .collect::<Vec<_>>();
            let return_type_id = self.semantic_type_to_type_id(signature.return_type);

            if let Some(function_info) = self.type_table.get_function_mut(symbol.type_id) {
                function_info.params = param_type_ids;
                function_info.return_type = return_type_id;
            }
        }
    }

    fn push_unresolved_function_type_errors(&mut self, program: &Program, source: &str) {
        for function in &program.functions {
            let Some(signature) = self.functions.get(&function.name).cloned() else {
                continue;
            };

            for (index, param_type) in signature.param_types.iter().copied().enumerate() {
                if param_type == SemanticType::Unknown {
                    self.push_type_error(
                        function.params[index].span,
                        source,
                        format!(
                            "Could not infer type for parameter '{}' in function '{}'.",
                            function.params[index].name, function.name
                        ),
                    );
                }
            }

            if signature.return_type == SemanticType::Unknown {
                self.push_type_error(
                    function.span,
                    source,
                    format!(
                        "Could not infer return type for function '{}'.",
                        function.name
                    ),
                );
            }
        }

        for type_decl in &program.types {
            let Some(receiver_type_id) = self.type_symbols.get(&type_decl.name).copied() else {
                continue;
            };

            for method in &type_decl.methods {
                let key = Self::method_symbol_key(receiver_type_id, &method.name);
                let Some(signature) = self.functions.get(&key).cloned() else {
                    continue;
                };

                for (index, param_type) in signature.param_types.iter().copied().enumerate() {
                    if param_type == SemanticType::Unknown {
                        self.push_type_error(
                            method.params[index].span,
                            source,
                            format!(
                                "Could not infer type for parameter '{}' in method '{}.{}'.",
                                method.params[index].name, type_decl.name, method.name
                            ),
                        );
                    }
                }

                if signature.return_type == SemanticType::Unknown {
                    self.push_type_error(
                        method.span,
                        source,
                        format!(
                            "Could not infer return type for method '{}.{}'.",
                            type_decl.name, method.name
                        ),
                    );
                }
            }
        }
    }
}

fn merge_types(
    current: SemanticType,
    inferred: SemanticType,
) -> Result<SemanticType, (SemanticType, SemanticType)> {
    match (current, inferred) {
        (SemanticType::Unknown, known) => Ok(known),
        (known, SemanticType::Unknown) => Ok(known),
        (left, right) if left == right => Ok(left),
        (left, right) => Err((left, right)),
    }
}
