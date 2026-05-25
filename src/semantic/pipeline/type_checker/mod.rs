use std::collections::HashSet;

use crate::parser::expression::{
    AssignTarget, BinaryExpr, BinaryOp, BlockExpr, BuiltinFunction, DestructiveAssignExpr,
    ElifBranch, Expr, FunctionCallExpr, FunctionDecl, IfExpr, LetInExpr, Literal, MethodCallExpr,
    MethodDecl, NewExpr, Program, Span, Statement, TypeDecl, UnaryExpr, UnaryOp, WhileExpr,
};

use super::super::{
    analyzer::SemanticAnalyzer,
    helper::{SemanticType, TypeId},
};
use super::{SymbolCollector, TypeConstraintEngine, TypeResolver};

mod binary_expr_checker;
mod block_expr_checker;
mod builtin_call_expr_checker;
mod destructive_assign_expr_checker;
mod function_call_expr_checker;
mod if_expr_checker;
mod let_in_expr_checker;
mod literal_expr_checker;
mod member_access_expr_checker;
mod method_call_expr_checker;
mod new_expr_checker;
mod unary_expr_checker;
mod variable_expr_checker;
mod while_expr_checker;

pub(in crate::semantic) struct TypeChecker<'a> {
    pub(in crate::semantic) analyzer: &'a mut SemanticAnalyzer,
}

impl<'a> TypeChecker<'a> {
    pub(in crate::semantic) fn new(analyzer: &'a mut SemanticAnalyzer) -> Self {
        Self { analyzer }
    }

    pub(in crate::semantic) fn check_program(&mut self, program: &Program, source: &str) {
        for type_decl in &program.types {
            self.check_type_decl(type_decl, source);
        }

        for function in &program.functions {
            self.check_function_decl(function, source);
        }

        for statement in &program.statements {
            let _ = self.check_statement(statement, source);
        }
    }

    pub(in crate::semantic) fn check_statement(
        &mut self,
        statement: &Statement,
        source: &str,
    ) -> Option<SemanticType> {
        match statement {
            Statement::Let {
                name,
                name_span,
                type_annotation,
                value,
                ..
            } => {
                if self.analyzer.is_declared_in_current_scope(name) {
                    self.analyzer.push_semantic_error(
                        *name_span,
                        source,
                        format!(
                            "Variable '{}' redeclared. A variable can only be declared once.",
                            name
                        ),
                    );
                    return None;
                }

                let binding_type = if let Some(annotation) = type_annotation {
                    if let Some(annotation_type) =
                        TypeResolver::resolve_annotation_type(self.analyzer, annotation, source)
                    {
                        self.check_annotated_initializer(
                            name,
                            value,
                            annotation_type,
                            annotation.span,
                            source,
                        )
                    } else {
                        self.check_expr(value, source)
                            .unwrap_or(SemanticType::Unknown)
                    }
                } else {
                    self.check_expr(value, source)
                        .unwrap_or(SemanticType::Unknown)
                };

                self.analyzer.bind_current_scope(name.clone(), binding_type);
                Some(binding_type)
            }
            Statement::Print { value, span } => self.check_print_argument(value, *span, source),
            Statement::Expr { value, .. } => self.check_expr(value, source),
            Statement::Assign {
                name,
                name_span,
                value,
                ..
            } => {
                let Some(scope_index) = self.analyzer.find_scope_index(name) else {
                    self.analyzer.push_semantic_error(
                        *name_span,
                        source,
                        format!(
                            "Variable '{}' is assigned before declaration. Declare it with 'let' first.",
                            name
                        ),
                    );
                    return None;
                };

                let value_type = self.check_expr(value, source)?;
                self.analyzer
                    .assign_in_scope(scope_index, name.clone(), value_type);
                Some(value_type)
            }
        }
    }

    pub(in crate::semantic) fn check_expr(
        &mut self,
        expr: &Expr,
        source: &str,
    ) -> Option<SemanticType> {
        match expr {
            Expr::Literal { value, .. } => self.check_literal_expr(value),
            Expr::DestructiveAssign(assign) => self.check_destructive_assign(assign, source),
            Expr::Variable { name, span } => self.check_variable_expr(name, *span, source),
            Expr::Unary(unary) => self.check_unary_expr(unary, source),
            Expr::Block(block) => self.check_block_expr(block, source),
            Expr::LetIn(let_in) => self.check_let_in_expr(let_in, source),
            Expr::While(while_expr) => self.check_while_expr(while_expr, source),
            Expr::If(if_expr) => self.check_if_expr(if_expr, source),
            Expr::BuiltinCall(call) => {
                self.check_builtin_call(call.function, &call.args, call.span, source)
            }
            Expr::FunctionCall(call) => self.check_function_call(call, source),
            Expr::MethodCall(call) => self.check_method_call(call, source),
            Expr::MemberAccess(access) => self.check_member_access(access, source),
            Expr::New(new_expr) => self.check_new_expr(new_expr, source),
            Expr::Binary(binary) => self.check_binary_expr(binary, source),
        }
    }

    fn check_annotated_initializer(
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
            value_type =
                TypeConstraintEngine::constrain_expr_type(self, value, annotation_type, source);
        }

        if value_type != SemanticType::Unknown && value_type != annotation_type {
            if self.types_compatible(annotation_type, value_type) {
                return annotation_type;
            }
            self.analyzer.push_type_error(
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

    pub(super) fn types_compatible(
        &self,
        expected: SemanticType,
        actual: SemanticType,
    ) -> bool {
        if expected == actual {
            return true;
        }

        if actual == SemanticType::Null && expected.is_nullable() {
            return true;
        }

        if expected == SemanticType::Null && actual.is_nullable() {
            return true;
        }

        match (expected, actual) {
            (SemanticType::Struct(parent), SemanticType::Struct(child)) => {
                self.is_subtype_of(TypeId(child), TypeId(parent))
            }
            _ => false,
        }
    }

    pub(in crate::semantic) fn check_type_decl(&mut self, type_decl: &TypeDecl, source: &str) {
        let Some(type_id) = self.analyzer.type_symbols.get(&type_decl.name).copied() else {
            return;
        };

        self.analyzer.push_scope();
        let mut ctor_param_names = HashSet::new();

        let constructor_types = self
            .analyzer
            .type_table
            .get_struct(type_id)
            .map(|info| info.constructor_params.clone())
            .unwrap_or_default();

        for (index, param) in type_decl.params.iter().enumerate() {
            if !ctor_param_names.insert(param.name.clone()) {
                self.analyzer.push_semantic_error(
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
                .map(|(_, type_id)| TypeResolver::type_id_to_semantic_type(self.analyzer, *type_id))
                .unwrap_or(SemanticType::Unknown);
            self.analyzer
                .bind_current_scope(param.name.clone(), semantic_type);
        }

        if let Some(parent_name) = &type_decl.parent_name {
            let Some(parent_id) = self.analyzer.type_symbols.get(parent_name).copied() else {
                self.analyzer.pop_scope();
                return;
            };

            let parent_params = self
                .analyzer
                .type_table
                .get_struct(parent_id)
                .map(|info| info.constructor_params.clone())
                .unwrap_or_default();

            if parent_params.len() != type_decl.parent_init_exprs.len() {
                if let Some(parent_span) = type_decl.parent_span {
                    self.analyzer.push_semantic_error(
                        parent_span,
                        source,
                        format!(
                            "Parent type '{}' constructor expects {} argument(s), but got {}.",
                            parent_name,
                            parent_params.len(),
                            type_decl.parent_init_exprs.len()
                        ),
                    );
                }
            }

            for (index, arg) in type_decl.parent_init_exprs.iter().enumerate() {
                let arg_type = self.check_expr(arg, source).unwrap_or(SemanticType::Unknown);
                let expected_type = parent_params
                    .get(index)
                    .map(|(_, type_id)| {
                        TypeResolver::type_id_to_semantic_type(self.analyzer, *type_id)
                    })
                    .unwrap_or(SemanticType::Unknown);

                if expected_type != SemanticType::Unknown
                    && arg_type != SemanticType::Unknown
                    && !self.types_compatible(expected_type, arg_type)
                {
                    self.analyzer.push_type_error(
                        arg.span(),
                        source,
                        format!(
                            "Parent type '{}' constructor argument #{} expects {}, but got {}.",
                            parent_name,
                            index + 1,
                            expected_type.display_name(),
                            arg_type.display_name()
                        ),
                    );
                }
            }
        }

        let mut fields = Vec::with_capacity(type_decl.attributes.len());
        let mut field_names = HashSet::new();

        for attribute in &type_decl.attributes {
            if !field_names.insert(attribute.name.clone()) {
                self.analyzer.push_semantic_error(
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
                TypeResolver::semantic_type_to_type_id(self.analyzer, value_type),
            ));
        }

        self.analyzer.pop_scope();

        if let Some(info) = self.analyzer.type_table.get_struct_mut(type_id) {
            info.fields = fields;
        }

        for method in &type_decl.methods {
            self.check_method_decl(type_id, &type_decl.name, method, source);
        }
    }

    pub(in crate::semantic) fn check_method_decl(
        &mut self,
        receiver_type_id: TypeId,
        receiver_type_name: &str,
        method: &MethodDecl,
        source: &str,
    ) {
        let key = SymbolCollector::method_symbol_key(receiver_type_id, &method.name);
        let mut param_names = HashSet::new();

        let param_types = self
            .analyzer
            .functions
            .get(&key)
            .map(|signature| signature.param_types.clone())
            .unwrap_or_else(|| vec![SemanticType::Unknown; method.params.len()]);

        let previous_receiver = self.analyzer.current_method_receiver;
        let previous_self_scope = self.analyzer.current_self_scope_index;

        self.analyzer.push_scope();
        self.analyzer
            .bind_current_scope("self".to_string(), SemanticType::Struct(receiver_type_id.0));
        self.analyzer.current_method_receiver = Some(receiver_type_id);
        self.analyzer.current_self_scope_index = self.analyzer.find_scope_index("self");

        self.analyzer.push_scope();

        for (index, param) in method.params.iter().enumerate() {
            if !param_names.insert(param.name.clone()) {
                self.analyzer.push_semantic_error(
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
            self.analyzer
                .bind_current_scope(param.name.clone(), param_type);
        }

        let expected_return_type = self
            .analyzer
            .functions
            .get(&key)
            .map(|signature| signature.return_type)
            .unwrap_or(SemanticType::Unknown);

        let mut body_type = self
            .check_expr(&method.body, source)
            .unwrap_or(SemanticType::Unknown);
        if body_type == SemanticType::Unknown && expected_return_type != SemanticType::Unknown {
            body_type = TypeConstraintEngine::constrain_expr_type(
                self,
                &method.body,
                expected_return_type,
                source,
            );
        }

        let inferred_param_types = method
            .params
            .iter()
            .map(|param| {
                self.analyzer
                    .lookup(&param.name)
                    .unwrap_or(SemanticType::Unknown)
            })
            .collect::<Vec<_>>();

        self.analyzer.pop_scope();
        self.analyzer.pop_scope();
        self.analyzer.current_method_receiver = previous_receiver;
        self.analyzer.current_self_scope_index = previous_self_scope;

        for (index, (param, inferred_type)) in method
            .params
            .iter()
            .zip(inferred_param_types.iter().copied())
            .enumerate()
        {
            let _ = TypeConstraintEngine::constrain_function_param_type(
                self,
                &key,
                index,
                inferred_type,
                param.span,
                source,
            );
        }

        let _ = TypeConstraintEngine::constrain_function_return_type(
            self,
            &key,
            body_type,
            method.span,
            source,
        );
    }

    pub(in crate::semantic) fn check_function_decl(
        &mut self,
        function: &FunctionDecl,
        source: &str,
    ) {
        let mut param_names = HashSet::new();
        let param_types = self
            .analyzer
            .functions
            .get(&function.name)
            .map(|signature| signature.param_types.clone())
            .unwrap_or_else(|| vec![SemanticType::Unknown; function.params.len()]);

        self.analyzer.push_scope();

        for (index, param) in function.params.iter().enumerate() {
            if !param_names.insert(param.name.clone()) {
                self.analyzer.push_semantic_error(
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
            self.analyzer
                .bind_current_scope(param.name.clone(), param_type);
        }

        let expected_return_type = self
            .analyzer
            .functions
            .get(&function.name)
            .map(|signature| signature.return_type)
            .unwrap_or(SemanticType::Unknown);

        let mut body_type = self
            .check_expr(&function.body, source)
            .unwrap_or(SemanticType::Unknown);
        if body_type == SemanticType::Unknown && expected_return_type != SemanticType::Unknown {
            body_type = TypeConstraintEngine::constrain_expr_type(
                self,
                &function.body,
                expected_return_type,
                source,
            );
        }

        let inferred_param_types = function
            .params
            .iter()
            .map(|param| {
                self.analyzer
                    .lookup(&param.name)
                    .unwrap_or(SemanticType::Unknown)
            })
            .collect::<Vec<_>>();

        self.analyzer.pop_scope();

        for (index, (param, inferred_type)) in function
            .params
            .iter()
            .zip(inferred_param_types.iter().copied())
            .enumerate()
        {
            let _ = TypeConstraintEngine::constrain_function_param_type(
                self,
                &function.name,
                index,
                inferred_type,
                param.span,
                source,
            );
        }

        let _ = TypeConstraintEngine::constrain_function_return_type(
            self,
            &function.name,
            body_type,
            function.span,
            source,
        );
    }

    pub(in crate::semantic) fn resolve_method_symbol_key(
        &self,
        receiver: TypeId,
        method_name: &str,
    ) -> Option<String> {
        let mut cursor = Some(receiver);
        while let Some(current) = cursor {
            let key = SymbolCollector::method_symbol_key(current, method_name);
            if self.analyzer.function_symbols.contains_key(&key) {
                return Some(key);
            }
            cursor = self
                .analyzer
                .type_table
                .get_struct(current)
                .and_then(|info| info.parent);
        }

        None
    }

    fn lookup_field_type_id(&self, receiver: TypeId, field_name: &str) -> Option<TypeId> {
        let mut cursor = Some(receiver);
        while let Some(current) = cursor {
            let Some(info) = self.analyzer.type_table.get_struct(current) else {
                return None;
            };
            if let Some((_, type_id)) = info.fields.iter().find(|(name, _)| name == field_name) {
                return Some(*type_id);
            }
            cursor = info.parent;
        }
        None
    }

    fn can_access_private_field(&self, receiver: TypeId) -> bool {
        self.analyzer.current_method_receiver == Some(receiver)
    }

    fn is_self_binding(&self, name: &str, scope_index: usize) -> bool {
        name == "self" && self.analyzer.current_self_scope_index == Some(scope_index)
    }

    fn is_subtype_of(&self, child: TypeId, parent: TypeId) -> bool {
        if child == parent {
            return true;
        }

        let mut cursor = self
            .analyzer
            .type_table
            .get_struct(child)
            .and_then(|info| info.parent);
        while let Some(current) = cursor {
            if current == parent {
                return true;
            }
            cursor = self
                .analyzer
                .type_table
                .get_struct(current)
                .and_then(|info| info.parent);
        }
        false
    }
}
