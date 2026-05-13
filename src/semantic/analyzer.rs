use std::collections::{HashMap, HashSet};

use crate::{
    error::{CompilerError, ErrorCategory, offset_to_line_column},
    parser::expression::{
        BlockExpr, Expr, FunctionDecl, IfExpr, LetInExpr, Program, Span, Statement,
        TypeAnnotation,
    },
};

use super::helper::{FunctionSignature, ScopeStack, SemanticType};

const MAX_INFERENCE_PASSES: usize = 8;

#[derive(Debug, Default)]
pub struct SemanticAnalyzer {
    pub(super) scopes: ScopeStack<SemanticType>,
    pub(super) functions: HashMap<String, FunctionSignature>,
    pub(super) errors: Vec<CompilerError>,
    next_function_type_id: u32,
    suppress_errors: bool,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn function_signatures(&self) -> &HashMap<String, FunctionSignature> {
        &self.functions
    }

    pub fn analyze(&mut self, program: &Program, source: &str) -> Vec<CompilerError> {
        let inferred_signatures = self.infer_function_signatures(program, source);

        self.reset_analysis_state();
        self.collect_functions(&program.functions, source);
        self.apply_inferred_signatures(&inferred_signatures);
        self.start_scope_pass();

        for function in &program.functions {
            self.check_function_decl(function, source);
        }

        for statement in &program.statements {
            let _ = self.check_statement(statement, source);
        }

        self.push_unresolved_function_type_errors(program, source);

        self.errors.clone()
    }

    fn infer_function_signatures(
        &mut self,
        program: &Program,
        source: &str,
    ) -> HashMap<String, FunctionSignature> {
        self.reset_analysis_state();
        self.suppress_errors = true;
        self.collect_functions(&program.functions, source);

        for _ in 0..MAX_INFERENCE_PASSES {
            let before = self.functions.clone();
            self.start_scope_pass();

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
        self.errors.clear();
        self.next_function_type_id = 0;
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
        let Some(annotation_type) = SemanticType::from_annotation_name(&annotation.name) else {
            self.push_semantic_error(
                annotation.span,
                source,
                format!(
                    "Unknown type annotation '{}'. Expected one of: {}.",
                    annotation.name,
                    SemanticType::annotation_names()
                ),
            );
            return None;
        };

        Some(annotation_type)
    }

    pub(super) fn check_annotated_initializer(
        &mut self,
        variable_name: &str,
        value: &Expr,
        annotation_type: SemanticType,
        annotation_span: Span,
        source: &str,
    ) -> SemanticType {
        let mut value_type = self.check_expr(value, source).unwrap_or(SemanticType::Unknown);

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
