use std::collections::{HashMap, HashSet};

use crate::{
    error::{CompilerError, ErrorCategory, offset_to_line_column},
    parser::expression::{
        BlockExpr, Expr, FunctionDecl, IfExpr, LetInExpr, MemberAccessExpr, MethodCallExpr,
        Program, Span, Statement, TypeAnnotation, TypeDecl,
    },
};

use super::helper::{FunctionSignature, ScopeStack, SemanticType};

const MAX_INFERENCE_PASSES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypeParamSignature {
    pub(crate) name: String,
    pub(crate) value_type: SemanticType,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttributeSignature {
    pub(crate) value_type: SemanticType,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypeSignature {
    pub(crate) type_id: u32,
    pub(crate) params: Vec<TypeParamSignature>,
    pub(crate) attributes: HashMap<String, AttributeSignature>,
    pub(crate) methods: HashMap<String, FunctionSignature>,
}

#[derive(Debug, Default)]
pub struct SemanticAnalyzer {
    pub(super) scopes: ScopeStack<SemanticType>,
    pub(super) functions: HashMap<String, FunctionSignature>,
    pub(super) types: HashMap<String, TypeSignature>,
    pub(super) errors: Vec<CompilerError>,
    next_function_type_id: u32,
    next_struct_type_id: u32,
    suppress_errors: bool,
    pub(super) current_type_context: Option<String>,
    pub(super) implicit_self_scope_index: Option<usize>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn function_signatures(&self) -> &HashMap<String, FunctionSignature> {
        &self.functions
    }

    pub fn type_signatures(&self) -> &HashMap<String, TypeSignature> {
        &self.types
    }

    pub fn analyze(&mut self, program: &Program, source: &str) -> Vec<CompilerError> {
        let (inferred_function_signatures, inferred_type_signatures) =
            self.infer_signatures(program, source);

        self.reset_analysis_state();
        self.collect_types(&program.type_decls, source);
        self.collect_functions(&program.functions, source);
        self.apply_inferred_signatures(&inferred_function_signatures, &inferred_type_signatures);
        self.start_scope_pass();

        for type_decl in &program.type_decls {
            self.check_type_decl(type_decl, source);
        }

        for function in &program.functions {
            self.check_function_decl(function, source);
        }

        for statement in &program.statements {
            let _ = self.check_statement(statement, source);
        }

        self.push_unresolved_type_member_errors(program, source);
        self.push_unresolved_function_type_errors(program, source);

        self.errors.clone()
    }

    fn infer_signatures(
        &mut self,
        program: &Program,
        source: &str,
    ) -> (
        HashMap<String, FunctionSignature>,
        HashMap<String, TypeSignature>,
    ) {
        self.reset_analysis_state();
        self.suppress_errors = true;
        self.collect_types(&program.type_decls, source);
        self.collect_functions(&program.functions, source);

        for _ in 0..MAX_INFERENCE_PASSES {
            let before_functions = self.functions.clone();
            let before_types = self.types.clone();
            self.start_scope_pass();

            for type_decl in &program.type_decls {
                self.check_type_decl(type_decl, source);
            }

            for function in &program.functions {
                self.check_function_decl(function, source);
            }

            for statement in &program.statements {
                let _ = self.check_statement(statement, source);
            }

            if self.functions == before_functions && self.types == before_types {
                break;
            }
        }

        self.suppress_errors = false;
        (self.functions.clone(), self.types.clone())
    }

    fn reset_analysis_state(&mut self) {
        self.scopes.clear();
        self.functions.clear();
        self.types.clear();
        self.errors.clear();
        self.next_function_type_id = 0;
        self.next_struct_type_id = 0;
        self.suppress_errors = false;
        self.current_type_context = None;
        self.implicit_self_scope_index = None;
    }

    fn start_scope_pass(&mut self) {
        self.scopes.clear();
        self.push_scope();
    }

    fn apply_inferred_signatures(
        &mut self,
        inferred_functions: &HashMap<String, FunctionSignature>,
        inferred_types: &HashMap<String, TypeSignature>,
    ) {
        for (name, signature) in inferred_functions {
            if let Some(entry) = self.functions.get_mut(name) {
                entry.param_types = signature.param_types.clone();
                entry.return_type = signature.return_type;
            }
        }

        for (type_name, inferred_signature) in inferred_types {
            let Some(target_signature) = self.types.get_mut(type_name) else {
                continue;
            };

            for (index, inferred_param) in inferred_signature.params.iter().enumerate() {
                if let Some(target_param) = target_signature.params.get_mut(index) {
                    target_param.value_type = inferred_param.value_type;
                }
            }

            for (attr_name, inferred_attr) in &inferred_signature.attributes {
                if let Some(target_attr) = target_signature.attributes.get_mut(attr_name) {
                    target_attr.value_type = inferred_attr.value_type;
                }
            }

            for (method_name, inferred_method) in &inferred_signature.methods {
                if let Some(target_method) = target_signature.methods.get_mut(method_name) {
                    target_method.param_types = inferred_method.param_types.clone();
                    target_method.return_type = inferred_method.return_type;
                }
            }
        }
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

    pub(super) fn current_scope_index(&self) -> Option<usize> {
        self.scopes.current_index()
    }

    pub(super) fn current_type_id(&self) -> Option<u32> {
        let type_name = self.current_type_context.as_ref()?;
        self.types.get(type_name).map(|entry| entry.type_id)
    }

    pub(super) fn lookup_type_by_id(&self, type_id: u32) -> Option<(&String, &TypeSignature)> {
        self.types
            .iter()
            .find(|(_, signature)| signature.type_id == type_id)
    }

    pub(super) fn lookup_attribute_type(
        &self,
        owner_type_id: u32,
        attribute_name: &str,
    ) -> Option<SemanticType> {
        self.lookup_type_by_id(owner_type_id)
            .and_then(|(_, signature)| signature.attributes.get(attribute_name))
            .map(|entry| entry.value_type)
    }

    pub(super) fn lookup_method_signature(
        &self,
        owner_type_id: u32,
        method_name: &str,
    ) -> Option<FunctionSignature> {
        self.lookup_type_by_id(owner_type_id)
            .and_then(|(_, signature)| signature.methods.get(method_name))
            .cloned()
    }

    pub(super) fn resolve_annotation_type(
        &mut self,
        annotation: &TypeAnnotation,
        source: &str,
    ) -> Option<SemanticType> {
        if let Some(annotation_type) = self.resolve_type_name(&annotation.name) {
            return Some(annotation_type);
        }

        self.push_semantic_error(
            annotation.span,
            source,
            format!(
                "Unknown type annotation '{}'. Expected one of: {}.",
                annotation.name,
                self.available_annotation_names()
            ),
        );
        None
    }

    pub(super) fn resolve_type_name(&self, name: &str) -> Option<SemanticType> {
        match name {
            "Number" => Some(SemanticType::Number),
            "Boolean" => Some(SemanticType::Boolean),
            "String" => Some(SemanticType::String),
            "Unit" => Some(SemanticType::Unit),
            _ => self
                .types
                .get(name)
                .map(|signature| SemanticType::Struct(signature.type_id)),
        }
    }

    fn available_annotation_names(&self) -> String {
        let mut names = vec![
            "Number".to_string(),
            "Boolean".to_string(),
            "String".to_string(),
            "Unit".to_string(),
        ];

        let mut user_types = self.types.keys().cloned().collect::<Vec<_>>();
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
            Expr::MethodCall(call) => self.constrain_method_return_type(call, expected, source),
            Expr::MemberAccess(access) => {
                self.constrain_member_access_type(access, expected, source)
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

    fn constrain_method_return_type(
        &mut self,
        call: &MethodCallExpr,
        inferred: SemanticType,
        source: &str,
    ) -> SemanticType {
        let instance_type = self
            .check_expr(&call.instance, source)
            .unwrap_or(SemanticType::Unknown);
        let SemanticType::Struct(owner_type_id) = instance_type else {
            return SemanticType::Unknown;
        };

        let owner_name = self
            .lookup_type_by_id(owner_type_id)
            .map(|(name, _)| name.clone());
        let Some(owner_name) = owner_name else {
            return SemanticType::Unknown;
        };

        let current_type = self
            .types
            .get(&owner_name)
            .and_then(|entry| entry.methods.get(&call.method))
            .map(|signature| signature.return_type)
            .unwrap_or(SemanticType::Unknown);

        match merge_types(current_type, inferred) {
            Ok(merged) => {
                if merged != current_type
                    && let Some(entry) = self.types.get_mut(&owner_name)
                    && let Some(signature) = entry.methods.get_mut(&call.method)
                {
                    signature.return_type = merged;
                }
                merged
            }
            Err((left, right)) => {
                self.push_type_error(
                    call.span,
                    source,
                    format!(
                        "Method '{}.{}' return type conflict: {} vs {}.",
                        owner_name,
                        call.method,
                        left.display_name(),
                        right.display_name()
                    ),
                );
                current_type
            }
        }
    }

    fn constrain_member_access_type(
        &mut self,
        access: &MemberAccessExpr,
        inferred: SemanticType,
        source: &str,
    ) -> SemanticType {
        let instance_type = self
            .check_expr(&access.instance, source)
            .unwrap_or(SemanticType::Unknown);
        let SemanticType::Struct(owner_type_id) = instance_type else {
            return SemanticType::Unknown;
        };

        let owner_name = self
            .lookup_type_by_id(owner_type_id)
            .map(|(name, _)| name.clone());
        let Some(owner_name) = owner_name else {
            return SemanticType::Unknown;
        };

        let current_type = self
            .types
            .get(&owner_name)
            .and_then(|entry| entry.attributes.get(&access.member))
            .map(|attribute| attribute.value_type)
            .unwrap_or(SemanticType::Unknown);

        match merge_types(current_type, inferred) {
            Ok(merged) => {
                if merged != current_type
                    && let Some(entry) = self.types.get_mut(&owner_name)
                    && let Some(attribute) = entry.attributes.get_mut(&access.member)
                {
                    attribute.value_type = merged;
                }
                merged
            }
            Err((left, right)) => {
                self.push_type_error(
                    access.span,
                    source,
                    format!(
                        "Attribute '{}.{}' has conflicting types: {} vs {}.",
                        owner_name,
                        access.member,
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
            if matches!(
                type_decl.name.as_str(),
                "Number" | "Boolean" | "String" | "Unit" | "Object"
            ) {
                self.push_semantic_error(
                    type_decl.name_span,
                    source,
                    format!(
                        "Type '{}' is reserved and cannot be redeclared.",
                        type_decl.name
                    ),
                );
                continue;
            }

            if self.types.contains_key(&type_decl.name) {
                self.push_semantic_error(
                    type_decl.name_span,
                    source,
                    format!("Type '{}' redeclared.", type_decl.name),
                );
                continue;
            }

            self.types.insert(
                type_decl.name.clone(),
                TypeSignature {
                    type_id: self.next_struct_type_id,
                    params: Vec::new(),
                    attributes: HashMap::new(),
                    methods: HashMap::new(),
                },
            );
            self.next_struct_type_id = self.next_struct_type_id.saturating_add(1);
        }

        for type_decl in type_decls {
            if !self.types.contains_key(&type_decl.name) {
                continue;
            }

            let mut params = Vec::with_capacity(type_decl.params.len());
            let mut param_names = HashSet::new();
            for param in &type_decl.params {
                if !param_names.insert(param.name.clone()) {
                    self.push_semantic_error(
                        param.span,
                        source,
                        format!(
                            "Type parameter '{}' redeclared in type '{}'.",
                            param.name, type_decl.name
                        ),
                    );
                    continue;
                }

                let value_type = param
                    .type_annotation
                    .as_ref()
                    .and_then(|annotation| self.resolve_annotation_type(annotation, source))
                    .unwrap_or(SemanticType::Unknown);
                params.push(TypeParamSignature {
                    name: param.name.clone(),
                    value_type,
                    span: param.span,
                });
            }

            let mut attributes = HashMap::new();
            for attribute in &type_decl.attributes {
                if attributes.contains_key(&attribute.name) {
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

                let value_type = attribute
                    .type_annotation
                    .as_ref()
                    .and_then(|annotation| self.resolve_annotation_type(annotation, source))
                    .unwrap_or(SemanticType::Unknown);
                attributes.insert(
                    attribute.name.clone(),
                    AttributeSignature {
                        value_type,
                        span: attribute.span,
                    },
                );
            }

            let mut methods = HashMap::new();
            for method in &type_decl.methods {
                if methods.contains_key(&method.name) {
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

                let mut param_types = Vec::with_capacity(method.params.len());
                let mut method_param_names = HashSet::new();
                for param in &method.params {
                    if !method_param_names.insert(param.name.clone()) {
                        self.push_semantic_error(
                            param.span,
                            source,
                            format!(
                                "Parameter '{}' redeclared in method '{}.{}'.",
                                param.name, type_decl.name, method.name
                            ),
                        );
                        continue;
                    }

                    let value_type = param
                        .type_annotation
                        .as_ref()
                        .and_then(|annotation| self.resolve_annotation_type(annotation, source))
                        .unwrap_or(SemanticType::Unknown);
                    param_types.push(value_type);
                }

                let return_type = method
                    .return_type_annotation
                    .as_ref()
                    .and_then(|annotation| self.resolve_annotation_type(annotation, source))
                    .unwrap_or(SemanticType::Unknown);

                let signature = FunctionSignature {
                    type_id: self.next_function_type_id,
                    param_types,
                    return_type,
                };
                self.next_function_type_id = self.next_function_type_id.saturating_add(1);
                methods.insert(method.name.clone(), signature);
            }

            if let Some(type_signature) = self.types.get_mut(&type_decl.name) {
                type_signature.params = params;
                type_signature.attributes = attributes;
                type_signature.methods = methods;
            }
        }
    }

    fn check_type_decl(&mut self, type_decl: &TypeDecl, source: &str) {
        if !self.types.contains_key(&type_decl.name) {
            return;
        }

        self.current_type_context = Some(type_decl.name.clone());

        for attribute in &type_decl.attributes {
            self.check_type_attribute_decl(
                &type_decl.name,
                attribute.name.as_str(),
                &attribute.value,
                attribute.span,
                source,
            );
        }

        for method in &type_decl.methods {
            self.check_method_decl(&type_decl.name, method, source);
        }

        self.current_type_context = None;
    }

    fn check_type_attribute_decl(
        &mut self,
        type_name: &str,
        attribute_name: &str,
        value: &Expr,
        span: Span,
        source: &str,
    ) {
        let Some(type_signature) = self.types.get(type_name).cloned() else {
            return;
        };

        self.push_scope();
        for param in &type_signature.params {
            self.bind_current_scope(param.name.clone(), param.value_type);
        }

        let mut inferred_attr_type = self
            .check_expr(value, source)
            .unwrap_or(SemanticType::Unknown);

        let mut expected_attr_type = self
            .types
            .get(type_name)
            .and_then(|entry| entry.attributes.get(attribute_name))
            .map(|entry| entry.value_type)
            .unwrap_or(SemanticType::Unknown);

        if inferred_attr_type == SemanticType::Unknown
            && expected_attr_type != SemanticType::Unknown
        {
            inferred_attr_type = self.constrain_expr_type(value, expected_attr_type, source);
        }

        if expected_attr_type == SemanticType::Unknown
            && inferred_attr_type != SemanticType::Unknown
        {
            if let Some(entry) = self.types.get_mut(type_name)
                && let Some(attribute) = entry.attributes.get_mut(attribute_name)
            {
                attribute.value_type = inferred_attr_type;
                expected_attr_type = inferred_attr_type;
            }
        }

        if expected_attr_type != SemanticType::Unknown
            && inferred_attr_type != SemanticType::Unknown
            && expected_attr_type != inferred_attr_type
        {
            self.push_type_error(
                span,
                source,
                format!(
                    "Attribute '{}.{}' expects {}, but initializer is {}.",
                    type_name,
                    attribute_name,
                    expected_attr_type.display_name(),
                    inferred_attr_type.display_name()
                ),
            );
        }

        let inferred_params = type_signature
            .params
            .iter()
            .map(|param| {
                (
                    param.name.clone(),
                    param.span,
                    self.lookup(&param.name).unwrap_or(SemanticType::Unknown),
                )
            })
            .collect::<Vec<_>>();
        self.pop_scope();

        for (index, (param_name, param_span, inferred_type)) in inferred_params.iter().enumerate() {
            let current_type = self
                .types
                .get(type_name)
                .and_then(|entry| entry.params.get(index))
                .map(|entry| entry.value_type)
                .unwrap_or(SemanticType::Unknown);

            match merge_types(current_type, *inferred_type) {
                Ok(merged) => {
                    if merged != current_type
                        && let Some(entry) = self.types.get_mut(type_name)
                        && let Some(param) = entry.params.get_mut(index)
                    {
                        param.value_type = merged;
                    }
                }
                Err((left, right)) => {
                    self.push_type_error(
                        *param_span,
                        source,
                        format!(
                            "Type parameter '{}.{}' has conflicting types: {} vs {}.",
                            type_name,
                            param_name,
                            left.display_name(),
                            right.display_name()
                        ),
                    );
                }
            }
        }
    }

    fn check_method_decl(
        &mut self,
        type_name: &str,
        method: &crate::parser::expression::MethodDecl,
        source: &str,
    ) {
        let Some(type_signature) = self.types.get(type_name).cloned() else {
            return;
        };
        let Some(method_signature) = type_signature.methods.get(&method.name).cloned() else {
            return;
        };

        self.push_scope();
        self.bind_current_scope(
            "self".to_string(),
            SemanticType::Struct(type_signature.type_id),
        );
        self.implicit_self_scope_index = self.current_scope_index();

        let mut param_names = HashSet::new();
        for (index, param) in method.params.iter().enumerate() {
            if !param_names.insert(param.name.clone()) {
                self.push_semantic_error(
                    param.span,
                    source,
                    format!(
                        "Parameter '{}' redeclared in method '{}.{}'.",
                        param.name, type_name, method.name
                    ),
                );
                continue;
            }

            let param_type = method_signature
                .param_types
                .get(index)
                .copied()
                .unwrap_or(SemanticType::Unknown);
            self.bind_current_scope(param.name.clone(), param_type);
        }

        let expected_return_type = method_signature.return_type;
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
        self.implicit_self_scope_index = None;

        for (index, (param, inferred_type)) in method
            .params
            .iter()
            .zip(inferred_param_types.iter().copied())
            .enumerate()
        {
            let current_type = self
                .types
                .get(type_name)
                .and_then(|entry| entry.methods.get(&method.name))
                .and_then(|signature| signature.param_types.get(index).copied())
                .unwrap_or(SemanticType::Unknown);

            match merge_types(current_type, inferred_type) {
                Ok(merged) => {
                    if merged != current_type
                        && let Some(entry) = self.types.get_mut(type_name)
                        && let Some(signature) = entry.methods.get_mut(&method.name)
                    {
                        signature.param_types[index] = merged;
                    }
                }
                Err((left, right)) => {
                    self.push_type_error(
                        param.span,
                        source,
                        format!(
                            "Method '{}.{}' argument #{} has conflicting types: {} vs {}.",
                            type_name,
                            method.name,
                            index + 1,
                            left.display_name(),
                            right.display_name()
                        ),
                    );
                }
            }
        }

        let current_return_type = self
            .types
            .get(type_name)
            .and_then(|entry| entry.methods.get(&method.name))
            .map(|signature| signature.return_type)
            .unwrap_or(SemanticType::Unknown);

        match merge_types(current_return_type, body_type) {
            Ok(merged) => {
                if merged != current_return_type
                    && let Some(entry) = self.types.get_mut(type_name)
                    && let Some(signature) = entry.methods.get_mut(&method.name)
                {
                    signature.return_type = merged;
                }
            }
            Err((left, right)) => {
                self.push_type_error(
                    method.span,
                    source,
                    format!(
                        "Method '{}.{}' return type conflict: {} vs {}.",
                        type_name,
                        method.name,
                        left.display_name(),
                        right.display_name()
                    ),
                );
            }
        }
    }

    fn collect_functions(&mut self, functions: &[FunctionDecl], source: &str) {
        for function in functions {
            if self.types.contains_key(&function.name) {
                self.push_semantic_error(
                    function.name_span,
                    source,
                    format!(
                        "Function '{}' conflicts with an existing type declaration.",
                        function.name
                    ),
                );
                continue;
            }

            if self.functions.contains_key(&function.name) {
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

            let signature = FunctionSignature {
                type_id: self.next_function_type_id,
                param_types,
                return_type,
            };
            self.next_function_type_id = self.next_function_type_id.saturating_add(1);
            self.functions.insert(function.name.clone(), signature);
        }
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

    fn push_unresolved_type_member_errors(&mut self, program: &Program, source: &str) {
        for type_decl in &program.type_decls {
            let Some(type_signature) = self.types.get(&type_decl.name).cloned() else {
                continue;
            };

            for (index, param_signature) in type_signature.params.iter().enumerate() {
                if param_signature.value_type == SemanticType::Unknown
                    && let Some(param_decl) = type_decl.params.get(index)
                {
                    self.push_type_error(
                        param_decl.span,
                        source,
                        format!(
                            "Could not infer type for type parameter '{}.{}'.",
                            type_decl.name, param_decl.name
                        ),
                    );
                }
            }

            for attribute_decl in &type_decl.attributes {
                let attr_type = type_signature
                    .attributes
                    .get(&attribute_decl.name)
                    .map(|entry| entry.value_type)
                    .unwrap_or(SemanticType::Unknown);

                if attr_type == SemanticType::Unknown {
                    self.push_type_error(
                        attribute_decl.span,
                        source,
                        format!(
                            "Could not infer type for attribute '{}.{}'.",
                            type_decl.name, attribute_decl.name
                        ),
                    );
                }
            }

            for method_decl in &type_decl.methods {
                let Some(method_signature) = type_signature.methods.get(&method_decl.name) else {
                    continue;
                };

                for (index, param_type) in method_signature.param_types.iter().copied().enumerate()
                {
                    if param_type == SemanticType::Unknown
                        && let Some(param_decl) = method_decl.params.get(index)
                    {
                        self.push_type_error(
                            param_decl.span,
                            source,
                            format!(
                                "Could not infer type for parameter '{}' in method '{}.{}'.",
                                param_decl.name, type_decl.name, method_decl.name
                            ),
                        );
                    }
                }

                if method_signature.return_type == SemanticType::Unknown {
                    self.push_type_error(
                        method_decl.span,
                        source,
                        format!(
                            "Could not infer return type for method '{}.{}'.",
                            type_decl.name, method_decl.name
                        ),
                    );
                }
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
