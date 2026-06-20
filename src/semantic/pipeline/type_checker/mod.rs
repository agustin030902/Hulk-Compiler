use std::collections::HashSet;

use crate::parser::expression::{
    AsExpr, AssignTarget, BinaryExpr, BinaryOp, BlockExpr, BuiltinFunction, DestructiveAssignExpr,
    ElifBranch, Expr, ForExpr, FunctionCallExpr, FunctionDecl, IfExpr, IsExpr, LetInExpr, Literal,
    MethodCallExpr, MethodDecl, NewExpr, Program, InterfaceDecl, Span, Statement, TypeDecl,
    UnaryExpr, UnaryOp, WhileExpr,
};

use super::super::{
    analyzer::SemanticAnalyzer,
    helper::{FunctionSignature, FunctionSymbol, SemanticType, TypeId},
};
use super::{SymbolCollector, TypeConstraintEngine, TypeResolver};

mod array_index_checker;
mod array_literal_checker;
mod binary_expr_checker;
mod block_expr_checker;
mod builtin_call_expr_checker;
mod destructive_assign_expr_checker;
mod for_expr_checker;
mod function_call_expr_checker;
mod if_expr_checker;
mod let_in_expr_checker;
mod literal_expr_checker;
mod member_access_expr_checker;
mod method_call_expr_checker;
mod new_array_checker;
mod new_expr_checker;
    mod interface_checker;
mod unary_expr_checker;
mod variable_expr_checker;
mod while_expr_checker;

pub(in crate::semantic) use interface_checker::InterfaceChecker;

pub(in crate::semantic) struct TypeChecker<'a> {
    pub(in crate::semantic) analyzer: &'a mut SemanticAnalyzer,
}

impl<'a> TypeChecker<'a> {
    pub(in crate::semantic) fn new(analyzer: &'a mut SemanticAnalyzer) -> Self {
        Self { analyzer }
    }

    pub(in crate::semantic) fn check_program(&mut self, program: &Program, source: &str) {
        for interface_decl in &program.interfaces {
            self.check_interface_decl(interface_decl, source);
        }

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

                self.analyzer.bind_current_scope(name.clone(), binding_type.clone());
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
                    .assign_in_scope(scope_index, name.clone(), value_type.clone());
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
            Expr::For(for_expr) => self.check_for_expr(for_expr, source),
            Expr::If(if_expr) => self.check_if_expr(if_expr, source),
            Expr::BuiltinCall(call) => {
                self.check_builtin_call(call.function, &call.args, call.span, source)
            }
            Expr::FunctionCall(call) => self.check_function_call(call, source),
            Expr::MethodCall(call) => self.check_method_call(call, source),
            Expr::MemberAccess(access) => self.check_member_access(access, source),
            Expr::New(new_expr) => self.check_new_expr(new_expr, source),
            Expr::NewArray(new_array) => self.check_new_array(new_array, source),
            Expr::ArrayIndex(array_index) => self.check_array_index(array_index, source),
            Expr::ArrayLiteral(array_literal) => self.check_array_literal(array_literal, source),
            Expr::Binary(binary) => self.check_binary_expr(binary, source),
            Expr::Is(is_expr) => self.check_is_expr(is_expr, source),
            Expr::As(as_expr) => self.check_as_expr(as_expr, source),
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
                TypeConstraintEngine::constrain_expr_type(self, value, annotation_type.clone(), source);
        }

        let SemanticType::Struct(annotation_raw) = annotation_type.clone() else {
            if value_type != SemanticType::Unknown && value_type != annotation_type {
                if self.types_compatible(annotation_type.clone(), value_type.clone()) {
                    return annotation_type;
                }
                self.analyzer.push_type_error(
                    annotation_span,
                    source,
                    format!(
                        "Type annotation for variable '{}' expects {}, but initializer is {}.",
                        variable_name,
                        annotation_type.display_name_with_table(&self.analyzer.type_table),
                        value_type.display_name_with_table(&self.analyzer.type_table)
                    ),
                );
            }
            return annotation_type;
        };

        let annotation_id = TypeId(annotation_raw);
        if !SymbolCollector::is_interface(self.analyzer, annotation_id) {
            if value_type != SemanticType::Unknown && value_type != annotation_type {
                if self.types_compatible(annotation_type.clone(), value_type.clone()) {
                    return annotation_type;
                }
                self.analyzer.push_type_error(
                    annotation_span,
                    source,
                    format!(
                        "Type annotation for variable '{}' expects {}, but initializer is {}.",
                        variable_name,
                        annotation_type.display_name_with_table(&self.analyzer.type_table),
                        value_type.display_name_with_table(&self.analyzer.type_table)
                    ),
                );
            }
            return annotation_type;
        }

        if value_type == SemanticType::Unknown {
            return annotation_type;
        }

        if value_type == annotation_type {
            return annotation_type;
        }

        if self
            .validate_interface_conformance(value_type.clone(), annotation_id, source)
            .is_some()
        {
            if let SemanticType::Struct(real_raw) = value_type {
                self.analyzer
                    .interface_real_types
                    .insert(variable_name.to_string(), TypeId(real_raw));
            }
            return annotation_type;
        }

        self.analyzer.push_type_error(
            annotation_span,
            source,
            format!(
                "Type annotation for variable '{}' uses interface '{}' but initializer of type {} does not conform to it.",
                variable_name,
                self.analyzer
                    .type_table
                    .get_struct(annotation_id)
                    .map(|info| info.name.clone())
                    .unwrap_or_default(),
                value_type.display_name_with_table(&self.analyzer.type_table)
            ),
        );
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

        match (expected, actual.clone()) {
            (SemanticType::Struct(parent), SemanticType::Struct(child)) => {
                let parent_id = TypeId(parent);
                if SymbolCollector::is_interface(self.analyzer, parent_id) {
                    return self
                        .validate_interface_conformance(actual, parent_id, "")
                        .is_some();
                }
                self.is_subtype_of(TypeId(child), parent_id)
            }
            _ => false,
        }
    }

    pub(in crate::semantic) fn check_interface_decl(
        &mut self,
        interface_decl: &InterfaceDecl,
        source: &str,
    ) {
        let Some(interface_id) = self
            .analyzer
            .type_symbols
            .get(&interface_decl.name)
            .copied()
        else {
            return;
        };

        for method in &interface_decl.methods {
            for param in &method.params {
                if let Some(annotation) = &param.type_annotation {
                    let _ = TypeResolver::resolve_annotation_type(
                        self.analyzer,
                        annotation,
                        source,
                    );
                }
            }
            let _ = TypeResolver::resolve_annotation_type(
                self.analyzer,
                &method.return_type_annotation,
                source,
            );
        }

        if let Some(parent_name) = &interface_decl.parent_name {
            let Some(parent_id) = self.analyzer.type_symbols.get(parent_name).copied() else {
                return;
            };
            for parent_method in
                InterfaceChecker::collect_inherited_interface_methods(self.analyzer, interface_id, parent_id)
            {
                let key = SymbolCollector::method_symbol_key(interface_id, &parent_method.name);
                if !self.analyzer.functions.contains_key(&key) {
                    self.analyzer.function_symbols.insert(
                        key.clone(),
                        FunctionSymbol::new_method(
                            parent_method.name.clone(),
                            parent_method.type_id,
                            interface_id,
                        ),
                    );
                    self.analyzer.functions.insert(
                        key,
                        FunctionSignature {
                            type_id: parent_method.type_id.0,
                            param_names: vec![],
                            param_types: parent_method.param_types.clone(),
                            return_type: parent_method.return_type,
                        },
                    );
                }
            }
        }
    }

    pub(super) fn validate_interface_conformance(
        &self,
        impl_type: SemanticType,
        interface_id: TypeId,
        source: &str,
    ) -> Option<()> {
        InterfaceChecker::validate_interface_conformance(self.analyzer, impl_type, interface_id, source)
    }

    pub(super) fn validate_interface_method_call(
        &mut self,
        impl_type: SemanticType,
        interface_id: TypeId,
        method_name: &str,
        call_span: Span,
        source: &str,
    ) -> Option<SemanticType> {
        InterfaceChecker::validate_interface_method_call(self.analyzer, impl_type, interface_id, method_name, call_span, source)
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
                        && !self.types_compatible(expected_type.clone(), arg_type.clone())
                    {
                        self.analyzer.push_type_error(
                            arg.span(),
                            source,
                            format!(
                                "Parent type '{}' constructor argument #{} expects {}, but got {}.",
                                parent_name,
                                index + 1,
                                expected_type.display_name_with_table(&self.analyzer.type_table),
                                arg_type.display_name_with_table(&self.analyzer.type_table)
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

            let annotated_type = attribute
                .type_annotation
                .as_ref()
                .and_then(|annotation| {
                    TypeResolver::resolve_annotation_type(self.analyzer, annotation, source)
                });

            let field_type = match annotated_type {
                Some(annotation_type) => {
                    if value_type != SemanticType::Unknown
                        && value_type != annotation_type
                    {
                        self.analyzer.push_type_error(
                            attribute.type_annotation.as_ref().unwrap().span,
                            source,
                            format!(
                                "Type annotation for attribute '{}' in type '{}' expects {}, but initializer is {}.",
                                attribute.name,
                                type_decl.name,
                                annotation_type.display_name_with_table(&self.analyzer.type_table),
                                value_type.display_name_with_table(&self.analyzer.type_table)
                            ),
                        );
                    }
                    annotation_type
                }
                None => value_type,
            };

            fields.push((
                attribute.name.clone(),
                TypeResolver::semantic_type_to_type_id(self.analyzer, field_type),
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
                .cloned()
                .unwrap_or(SemanticType::Unknown);
            self.analyzer
                .bind_current_scope(param.name.clone(), param_type);
        }

        let expected_return_type = self
            .analyzer
            .functions
            .get(&key)
            .map(|signature| signature.return_type.clone())
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
            .zip(inferred_param_types.iter().cloned())
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
        self.analyzer
            .function_decls
            .insert(function.name.clone(), function.clone());

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
                .cloned()
                .unwrap_or(SemanticType::Unknown);
            self.analyzer
                .bind_current_scope(param.name.clone(), param_type);
        }

        let expected_return_type = self
            .analyzer
            .functions
            .get(&function.name)
            .map(|signature| signature.return_type.clone())
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
            .zip(inferred_param_types.iter().cloned())
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
            if self
                .analyzer
                .type_table
                .get_struct(current)
                .is_some_and(|info| info.is_interface)
            {
                return false;
            }
            cursor = self
                .analyzer
                .type_table
                .get_struct(current)
                .and_then(|info| info.parent);
        }
        false
    }

    fn check_is_expr(&mut self, is_expr: &IsExpr, source: &str) -> Option<SemanticType> {
        let _expr_type = self.check_expr(&is_expr.expr, source)?;
        if TypeResolver::resolve_named_type(self.analyzer, &is_expr.target_type).is_none() {
            self.analyzer.push_semantic_error(
                is_expr.target_type_span,
                source,
                format!(
                    "Unknown type '{}' in 'is' expression. Expected one of: {}.",
                    is_expr.target_type,
                    TypeResolver::known_annotation_names(self.analyzer)
                ),
            );
        }
        Some(SemanticType::Boolean)
    }

    fn check_as_expr(&mut self, as_expr: &AsExpr, source: &str) -> Option<SemanticType> {
        let _expr_type = self.check_expr(&as_expr.expr, source)?;
        if let Some(semantic_type) =
            TypeResolver::resolve_named_type(self.analyzer, &as_expr.target_type)
        {
            Some(semantic_type)
        } else {
            self.analyzer.push_semantic_error(
                as_expr.target_type_span,
                source,
                format!(
                    "Unknown type '{}' in 'as' expression. Expected one of: {}.",
                    as_expr.target_type,
                    TypeResolver::known_annotation_names(self.analyzer)
                ),
            );
            Some(SemanticType::Unknown)
        }
    }
}
