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
            Expr::Literal { value, .. } => Some(self.check_literal(value)),
            Expr::DestructiveAssign(assign) => self.check_destructive_assign(assign, source),
            Expr::Variable { name, span } => self.check_variable(name, *span, source),
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

    fn check_literal(&self, literal: &Literal) -> SemanticType {
        match literal {
            Literal::Integer(_) | Literal::Float(_) => SemanticType::Number,
            Literal::Boolean(_) => SemanticType::Boolean,
            Literal::String(_) => SemanticType::String,
        }
    }

    fn check_variable(&mut self, name: &str, span: Span, source: &str) -> Option<SemanticType> {
        if let Some(var_type) = self.analyzer.lookup(name) {
            Some(var_type)
        } else {
            self.analyzer.push_semantic_error(
                span,
                source,
                format!(
                    "Variable '{}' is used before declaration. Declare it with 'let' first.",
                    name
                ),
            );
            None
        }
    }

    fn check_block_expr(&mut self, block: &BlockExpr, source: &str) -> Option<SemanticType> {
        self.analyzer.push_scope();

        let mut last_type: Option<SemanticType> = None;
        for statement in &block.statements {
            let stmt_type = self.check_statement(statement, source);
            if stmt_type.is_some() {
                last_type = stmt_type;
            }
        }

        self.analyzer.pop_scope();

        if block.statements.is_empty() {
            Some(SemanticType::Unit)
        } else {
            last_type
        }
    }

    fn check_let_in_expr(&mut self, let_in: &LetInExpr, source: &str) -> Option<SemanticType> {
        self.analyzer.push_scope();

        for binding in &let_in.bindings {
            if self.analyzer.is_declared_in_current_scope(&binding.name) {
                self.analyzer.push_semantic_error(
                    binding.span,
                    source,
                    format!("Variable '{}' redeclared in let-in binding.", binding.name),
                );
                continue;
            }

            let binding_type = if let Some(annotation) = &binding.type_annotation {
                if let Some(annotation_type) =
                    TypeResolver::resolve_annotation_type(self.analyzer, annotation, source)
                {
                    self.check_annotated_initializer(
                        &binding.name,
                        &binding.value,
                        annotation_type,
                        annotation.span,
                        source,
                    )
                } else {
                    self.check_expr(&binding.value, source)
                        .unwrap_or(SemanticType::Unknown)
                }
            } else {
                self.check_expr(&binding.value, source)
                    .unwrap_or(SemanticType::Unknown)
            };
            self.analyzer
                .bind_current_scope(binding.name.clone(), binding_type);
        }

        let body_type = self.check_expr(&let_in.body, source);

        self.analyzer.pop_scope();

        body_type
    }

    fn check_while_expr(&mut self, while_expr: &WhileExpr, source: &str) -> Option<SemanticType> {
        let mut condition_type = self.check_expr(&while_expr.condition, source)?;
        if condition_type == SemanticType::Unknown {
            condition_type = TypeConstraintEngine::constrain_expr_type(
                self,
                &while_expr.condition,
                SemanticType::Boolean,
                source,
            );
        }

        if condition_type == SemanticType::Unknown {
            return Some(SemanticType::Unknown);
        }

        if condition_type != SemanticType::Boolean {
            self.analyzer.push_type_error(
                while_expr.condition.span(),
                source,
                format!(
                    "While condition expects Boolean, but got {}.",
                    condition_type.display_name()
                ),
            );
            return None;
        }

        if self.check_block_expr(&while_expr.body, source).is_none() {
            return Some(SemanticType::Unknown);
        }

        Some(SemanticType::Unit)
    }

    fn check_if_expr(&mut self, if_expr: &IfExpr, source: &str) -> Option<SemanticType> {
        let condition_ok = self.check_condition(&if_expr.condition, source);

        let then_branch = if_expr.then_branch.as_ref();
        let mut branch_types = Vec::with_capacity(if_expr.elif_branches.len() + 2);
        if let Some(value_type) = self.check_expr(then_branch, source) {
            branch_types.push((then_branch, value_type));
        }

        let mut elif_conditions_ok = true;
        for branch in &if_expr.elif_branches {
            elif_conditions_ok &= self.check_elif_branch_condition(branch, source);
            if let Some(value_type) = self.check_expr(&branch.body, source) {
                branch_types.push((&branch.body, value_type));
            }
        }

        let else_branch = if_expr.else_branch.as_ref();
        if let Some(value_type) = self.check_expr(else_branch, source) {
            branch_types.push((else_branch, value_type));
        }

        if !condition_ok || !elif_conditions_ok {
            return Some(SemanticType::Unknown);
        }

        let Some(expected_type) = branch_types.iter().find_map(|(_, value_type)| {
            (*value_type != SemanticType::Unknown).then_some(*value_type)
        }) else {
            return Some(SemanticType::Unknown);
        };

        for (branch_expr, value_type) in &mut branch_types {
            if *value_type == SemanticType::Unknown {
                *value_type = TypeConstraintEngine::constrain_expr_type(
                    self,
                    branch_expr,
                    expected_type,
                    source,
                );
            }
        }

        if branch_types
            .iter()
            .any(|(_, value_type)| *value_type == SemanticType::Unknown)
        {
            return Some(SemanticType::Unknown);
        }

        for (branch_expr, actual_type) in branch_types.iter().copied() {
            if actual_type != expected_type {
                self.analyzer.push_type_error(
                    branch_expr.span(),
                    source,
                    format!(
                        "If branches must return the same type, but got {} and {}.",
                        expected_type.display_name(),
                        actual_type.display_name()
                    ),
                );
                return None;
            }
        }

        Some(expected_type)
    }

    fn check_elif_branch_condition(&mut self, branch: &ElifBranch, source: &str) -> bool {
        self.check_condition(&branch.condition, source)
    }

    fn check_condition(&mut self, condition: &Expr, source: &str) -> bool {
        match self.check_expr(condition, source) {
            Some(SemanticType::Boolean) => true,
            Some(SemanticType::Unknown) => {
                TypeConstraintEngine::constrain_expr_type(
                    self,
                    condition,
                    SemanticType::Boolean,
                    source,
                ) == SemanticType::Boolean
            }
            Some(condition_type) => {
                self.analyzer.push_type_error(
                    condition.span(),
                    source,
                    format!(
                        "If condition expects Boolean, but got {}.",
                        condition_type.display_name()
                    ),
                );
                false
            }
            None => false,
        }
    }

    fn check_unary_expr(&mut self, unary: &UnaryExpr, source: &str) -> Option<SemanticType> {
        let mut expr_type = self.check_expr(&unary.expr, source)?;

        match unary.op {
            UnaryOp::Neg => {
                if expr_type == SemanticType::Unknown {
                    expr_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &unary.expr,
                        SemanticType::Number,
                        source,
                    );
                }

                if expr_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if expr_type == SemanticType::Number {
                    Some(SemanticType::Number)
                } else {
                    self.analyzer.push_type_error(
                        unary.span,
                        source,
                        format!(
                            "Unary '-' expects Number, but got {}.",
                            expr_type.display_name()
                        ),
                    );
                    None
                }
            }
            UnaryOp::Not => {
                if expr_type == SemanticType::Unknown {
                    expr_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &unary.expr,
                        SemanticType::Boolean,
                        source,
                    );
                }

                if expr_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if expr_type == SemanticType::Boolean {
                    Some(SemanticType::Boolean)
                } else {
                    self.analyzer.push_type_error(
                        unary.span,
                        source,
                        format!(
                            "Unary '!' expects Boolean, but got {}.",
                            expr_type.display_name()
                        ),
                    );
                    None
                }
            }
        }
    }

    fn check_binary_expr(&mut self, binary: &BinaryExpr, source: &str) -> Option<SemanticType> {
        let left_type = self.check_expr(&binary.left, source);
        let right_type = self.check_expr(&binary.right, source);

        let (Some(mut left_type), Some(mut right_type)) = (left_type, right_type) else {
            return None;
        };

        match binary.op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Pow => {
                if left_type == SemanticType::Unknown {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.left,
                        SemanticType::Number,
                        source,
                    );
                }
                if right_type == SemanticType::Unknown {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.right,
                        SemanticType::Number,
                        source,
                    );
                }

                if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if left_type == SemanticType::Number && right_type == SemanticType::Number {
                    Some(SemanticType::Number)
                } else {
                    let op_name = op_symbol(binary.op.clone());
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        format!(
                            "Operator '{}' expects Number and Number, but got {} and {}.",
                            op_name,
                            left_type.display_name(),
                            right_type.display_name()
                        ),
                    );
                    None
                }
            }
            BinaryOp::Concat => {
                if left_type == SemanticType::Unknown
                    && (right_type == SemanticType::Number || right_type == SemanticType::String)
                {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.left,
                        SemanticType::String,
                        source,
                    );
                }
                if right_type == SemanticType::Unknown
                    && (left_type == SemanticType::Number || left_type == SemanticType::String)
                {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.right,
                        SemanticType::String,
                        source,
                    );
                }

                if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if is_valid_concat_pair(left_type, right_type) {
                    Some(SemanticType::String)
                } else {
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        format!(
                            "Operator '@' expects (String, String), (String, Number), or (Number, String), but got {} and {}.",
                            left_type.display_name(),
                            right_type.display_name()
                        ),
                    );
                    None
                }
            }
            BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEqual | BinaryOp::GreaterEqual => {
                if left_type == SemanticType::Unknown {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.left,
                        SemanticType::Number,
                        source,
                    );
                }
                if right_type == SemanticType::Unknown {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.right,
                        SemanticType::Number,
                        source,
                    );
                }

                if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if left_type == SemanticType::Number && right_type == SemanticType::Number {
                    Some(SemanticType::Boolean)
                } else {
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        format!(
                            "Comparison operator '{}' expects Number and Number, but got {} and {}.",
                            op_symbol(binary.op.clone()),
                            left_type.display_name(),
                            right_type.display_name()
                        ),
                    );
                    None
                }
            }
            BinaryOp::Equal | BinaryOp::NotEqual => {
                if left_type == SemanticType::Unknown && right_type != SemanticType::Unknown {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.left,
                        right_type,
                        source,
                    );
                } else if right_type == SemanticType::Unknown && left_type != SemanticType::Unknown
                {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.right,
                        left_type,
                        source,
                    );
                }

                if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if left_type != right_type {
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        format!(
                            "Operator '{}' expects operands of the same type, but got {} and {}.",
                            op_symbol(binary.op.clone()),
                            left_type.display_name(),
                            right_type.display_name()
                        ),
                    );
                    return None;
                }

                if is_equality_type(left_type) {
                    Some(SemanticType::Boolean)
                } else {
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        format!(
                            "Operator '{}' only supports Number, Boolean, or String operands.",
                            op_symbol(binary.op.clone())
                        ),
                    );
                    None
                }
            }
            BinaryOp::And | BinaryOp::Or => {
                if left_type == SemanticType::Unknown {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.left,
                        SemanticType::Boolean,
                        source,
                    );
                }
                if right_type == SemanticType::Unknown {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &binary.right,
                        SemanticType::Boolean,
                        source,
                    );
                }

                if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if left_type == SemanticType::Boolean && right_type == SemanticType::Boolean {
                    Some(SemanticType::Boolean)
                } else {
                    self.analyzer.push_type_error(
                        binary.span,
                        source,
                        "logical operator requires Boolean operands".to_string(),
                    );
                    None
                }
            }
        }
    }

    fn check_print_argument(
        &mut self,
        arg: &Expr,
        span: Span,
        source: &str,
    ) -> Option<SemanticType> {
        let arg_type = self.check_expr(arg, source)?;

        if arg_type == SemanticType::Unknown {
            return Some(SemanticType::Unknown);
        }

        if arg_type == SemanticType::Unit {
            self.analyzer.push_type_error(
                span,
                source,
                "Function 'print' expects a non-Unit argument, but got Unit.".to_string(),
            );
            return None;
        }

        if matches!(
            arg_type,
            SemanticType::Function(_) | SemanticType::Struct(_)
        ) {
            self.analyzer.push_type_error(
                span,
                source,
                format!(
                    "Function 'print' cannot print values of type {}.",
                    arg_type.display_name()
                ),
            );
            return None;
        }

        Some(SemanticType::Unit)
    }

    fn check_builtin_call(
        &mut self,
        function: BuiltinFunction,
        args: &[Expr],
        span: Span,
        source: &str,
    ) -> Option<SemanticType> {
        match function {
            BuiltinFunction::Print => {
                if args.len() != 1 {
                    self.analyzer.push_semantic_error(
                        span,
                        source,
                        "Function 'print' expects 1 argument.".to_string(),
                    );
                    return None;
                }
                let arg = &args[0];
                self.check_print_argument(arg, span, source)
            }
            BuiltinFunction::Sin
            | BuiltinFunction::Cos
            | BuiltinFunction::Sqrt
            | BuiltinFunction::Exp => {
                if args.len() != 1 {
                    self.analyzer.push_semantic_error(
                        span,
                        source,
                        format!("Function '{}' expects 1 argument.", function.name()),
                    );
                    return None;
                }
                let arg = &args[0];

                let mut arg_type = self.check_expr(arg, source)?;
                if arg_type == SemanticType::Unknown {
                    arg_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        arg,
                        SemanticType::Number,
                        source,
                    );
                }
                if arg_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if arg_type == SemanticType::Number {
                    Some(SemanticType::Number)
                } else {
                    self.analyzer.push_type_error(
                        span,
                        source,
                        format!(
                            "Function '{}' expects Number, but got {}.",
                            function.name(),
                            arg_type.display_name()
                        ),
                    );
                    None
                }
            }
            BuiltinFunction::Log => {
                if args.len() != 2 {
                    self.analyzer.push_semantic_error(
                        span,
                        source,
                        "Function 'log' expects 2 arguments.".to_string(),
                    );
                    return None;
                }

                let mut left_type = self.check_expr(&args[0], source)?;
                let mut right_type = self.check_expr(&args[1], source)?;
                if left_type == SemanticType::Unknown {
                    left_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &args[0],
                        SemanticType::Number,
                        source,
                    );
                }
                if right_type == SemanticType::Unknown {
                    right_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &args[1],
                        SemanticType::Number,
                        source,
                    );
                }
                if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown {
                    return Some(SemanticType::Unknown);
                }

                if left_type == SemanticType::Number && right_type == SemanticType::Number {
                    Some(SemanticType::Number)
                } else {
                    self.analyzer.push_type_error(
                        span,
                        source,
                        format!(
                            "Function 'log' expects (Number, Number), but got {} and {}.",
                            left_type.display_name(),
                            right_type.display_name()
                        ),
                    );
                    None
                }
            }
            BuiltinFunction::Rand => {
                if !args.is_empty() {
                    self.analyzer.push_semantic_error(
                        span,
                        source,
                        "Function 'rand' expects 0 arguments.".to_string(),
                    );
                    return None;
                }

                Some(SemanticType::Number)
            }
        }
    }

    fn check_function_call(
        &mut self,
        call: &FunctionCallExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let Some(symbol) = self.analyzer.function_symbols.get(&call.name).cloned() else {
            self.analyzer.push_semantic_error(
                call.name_span,
                source,
                format!("Function '{}' is called before declaration.", call.name),
            );
            return None;
        };

        if symbol.is_method() {
            self.analyzer.push_semantic_error(
                call.name_span,
                source,
                format!(
                    "Method '{}' requires a receiver and cannot be called as a global function.",
                    call.name
                ),
            );
            return None;
        }

        let Some(signature) = self.analyzer.functions.get(&call.name).cloned() else {
            self.analyzer.push_semantic_error(
                call.name_span,
                source,
                format!("Function '{}' is called before declaration.", call.name),
            );
            return None;
        };

        if signature.arity() != call.args.len() {
            self.analyzer.push_semantic_error(
                call.span,
                source,
                format!(
                    "Function '{}' expects {} argument(s), but got {}.",
                    call.name,
                    signature.arity(),
                    call.args.len()
                ),
            );
            return None;
        }

        let mut valid_call = true;

        for (index, arg) in call.args.iter().enumerate() {
            let arg_type = self
                .check_expr(arg, source)
                .unwrap_or(SemanticType::Unknown);
            let expected_type = self
                .analyzer
                .functions
                .get(&call.name)
                .and_then(|entry| entry.param_types.get(index).copied())
                .unwrap_or(SemanticType::Unknown);

            if arg_type == SemanticType::Unknown && expected_type != SemanticType::Unknown {
                let _ = TypeConstraintEngine::constrain_expr_type(self, arg, expected_type, source);
                continue;
            }

            if expected_type == SemanticType::Unknown && arg_type != SemanticType::Unknown {
                let _ = TypeConstraintEngine::constrain_function_param_type(
                    self,
                    &call.name,
                    index,
                    arg_type,
                    arg.span(),
                    source,
                );
                continue;
            }

            if expected_type != SemanticType::Unknown
                && arg_type != SemanticType::Unknown
                && expected_type != arg_type
            {
                self.analyzer.push_type_error(
                    arg.span(),
                    source,
                    format!(
                        "Function '{}' argument #{} expects {}, but got {}.",
                        call.name,
                        index + 1,
                        expected_type.display_name(),
                        arg_type.display_name()
                    ),
                );
                valid_call = false;
            }
        }

        if !valid_call {
            return None;
        }

        self.analyzer
            .functions
            .get(&call.name)
            .map(|entry| entry.return_type)
            .or(Some(SemanticType::Unknown))
    }

    fn check_method_call(&mut self, call: &MethodCallExpr, source: &str) -> Option<SemanticType> {
        let receiver_type = self.check_expr(&call.receiver, source)?;
        let SemanticType::Struct(receiver_raw) = receiver_type else {
            self.analyzer.push_type_error(
                call.span,
                source,
                format!(
                    "Method call expects a struct instance receiver, but got {}.",
                    receiver_type.display_name()
                ),
            );
            return None;
        };

        let receiver_id = TypeId(receiver_raw);
        let Some(method_key) = self.resolve_method_symbol_key(receiver_id, &call.method_name)
        else {
            self.analyzer.push_semantic_error(
                call.method_name_span,
                source,
                format!(
                    "Method '{}' is not declared for this type.",
                    call.method_name
                ),
            );
            return None;
        };

        let Some(signature) = self.analyzer.functions.get(&method_key).cloned() else {
            self.analyzer.push_semantic_error(
                call.method_name_span,
                source,
                format!(
                    "Method '{}' is not declared for this type.",
                    call.method_name
                ),
            );
            return None;
        };

        if signature.arity() != call.args.len() {
            self.analyzer.push_semantic_error(
                call.span,
                source,
                format!(
                    "Method '{}' expects {} argument(s), but got {}.",
                    call.method_name,
                    signature.arity(),
                    call.args.len()
                ),
            );
            return None;
        }

        let mut valid_call = true;

        for (index, arg) in call.args.iter().enumerate() {
            let arg_type = self
                .check_expr(arg, source)
                .unwrap_or(SemanticType::Unknown);
            let expected_type = self
                .analyzer
                .functions
                .get(&method_key)
                .and_then(|entry| entry.param_types.get(index).copied())
                .unwrap_or(SemanticType::Unknown);

            if arg_type == SemanticType::Unknown && expected_type != SemanticType::Unknown {
                let _ = TypeConstraintEngine::constrain_expr_type(self, arg, expected_type, source);
                continue;
            }

            if expected_type == SemanticType::Unknown && arg_type != SemanticType::Unknown {
                let _ = TypeConstraintEngine::constrain_function_param_type(
                    self,
                    &method_key,
                    index,
                    arg_type,
                    arg.span(),
                    source,
                );
                continue;
            }

            if expected_type != SemanticType::Unknown
                && arg_type != SemanticType::Unknown
                && expected_type != arg_type
            {
                self.analyzer.push_type_error(
                    arg.span(),
                    source,
                    format!(
                        "Method '{}' argument #{} expects {}, but got {}.",
                        call.method_name,
                        index + 1,
                        expected_type.display_name(),
                        arg_type.display_name()
                    ),
                );
                valid_call = false;
            }
        }

        if !valid_call {
            return None;
        }

        self.analyzer
            .functions
            .get(&method_key)
            .map(|entry| entry.return_type)
            .or(Some(SemanticType::Unknown))
    }

    fn check_member_access(
        &mut self,
        access: &crate::parser::expression::MemberAccessExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let object_type = self.check_expr(&access.object, source)?;
        let SemanticType::Struct(type_raw_id) = object_type else {
            self.analyzer.push_type_error(
                access.span,
                source,
                format!(
                    "Member access expects a struct instance, but got {}.",
                    object_type.display_name()
                ),
            );
            return None;
        };

        let receiver = TypeId(type_raw_id);
        let Some(field_type_id) = self.lookup_field_type_id(receiver, &access.member) else {
            self.analyzer.push_semantic_error(
                access.member_span,
                source,
                format!(
                    "Attribute '{}' is not declared in this type.",
                    access.member
                ),
            );
            return None;
        };

        if !self.can_access_private_field(receiver) {
            self.analyzer.push_semantic_error(
                access.member_span,
                source,
                format!(
                    "Attribute '{}' is private and cannot be accessed from this context.",
                    access.member
                ),
            );
            return None;
        }

        Some(TypeResolver::type_id_to_semantic_type(
            self.analyzer,
            field_type_id,
        ))
    }

    fn check_new_expr(&mut self, new_expr: &NewExpr, source: &str) -> Option<SemanticType> {
        let Some(type_id) = self.analyzer.type_symbols.get(&new_expr.type_name).copied() else {
            self.analyzer.push_semantic_error(
                new_expr.type_name_span,
                source,
                format!("Type '{}' is not declared.", new_expr.type_name),
            );
            return None;
        };

        let constructor_params = self
            .analyzer
            .type_table
            .get_struct(type_id)
            .map(|info| info.constructor_params.clone())
            .unwrap_or_default();

        if constructor_params.len() != new_expr.args.len() {
            self.analyzer.push_semantic_error(
                new_expr.span,
                source,
                format!(
                    "Type '{}' constructor expects {} argument(s), but got {}.",
                    new_expr.type_name,
                    constructor_params.len(),
                    new_expr.args.len()
                ),
            );
            return None;
        }

        for (index, arg) in new_expr.args.iter().enumerate() {
            let arg_type = self
                .check_expr(arg, source)
                .unwrap_or(SemanticType::Unknown);
            let expected_type = constructor_params
                .get(index)
                .map(|(_, param_type_id)| {
                    TypeResolver::type_id_to_semantic_type(self.analyzer, *param_type_id)
                })
                .unwrap_or(SemanticType::Unknown);

            if arg_type == SemanticType::Unknown && expected_type != SemanticType::Unknown {
                let _ = TypeConstraintEngine::constrain_expr_type(self, arg, expected_type, source);
                continue;
            }

            if expected_type == SemanticType::Unknown && arg_type != SemanticType::Unknown {
                let inferred_type_id =
                    TypeResolver::semantic_type_to_type_id(self.analyzer, arg_type);
                if let Some(info) = self.analyzer.type_table.get_struct_mut(type_id)
                    && let Some((_, entry_type_id)) = info.constructor_params.get_mut(index)
                {
                    *entry_type_id = inferred_type_id;
                }
                continue;
            }

            if expected_type != SemanticType::Unknown
                && arg_type != SemanticType::Unknown
                && expected_type != arg_type
            {
                self.analyzer.push_type_error(
                    arg.span(),
                    source,
                    format!(
                        "Type '{}' constructor argument #{} expects {}, but got {}.",
                        new_expr.type_name,
                        index + 1,
                        expected_type.display_name(),
                        arg_type.display_name()
                    ),
                );
                return None;
            }
        }

        Some(SemanticType::Struct(type_id.0))
    }

    fn check_destructive_assign(
        &mut self,
        assign: &DestructiveAssignExpr,
        source: &str,
    ) -> Option<SemanticType> {
        match &assign.target {
            AssignTarget::Variable { name, name_span } => {
                let Some((scope_index, existing)) = self.analyzer.lookup_with_scope_index(name)
                else {
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

                if self.is_self_binding(name, scope_index) {
                    self.analyzer.push_semantic_error(
                        *name_span,
                        source,
                        "`self` is not a valid assignment target.".to_string(),
                    );
                    return None;
                }

                let value_type = self.check_expr(&assign.value, source)?;

                if existing != SemanticType::Unknown && existing != value_type {
                    self.analyzer.push_type_error(
                        assign.span,
                        source,
                        format!(
                            "Destructive assignment ':=' requires the same type. Variable '{}' is {:?} but expression is {:?}.",
                            name, existing, value_type
                        ),
                    );
                    return None;
                }

                self.analyzer
                    .assign_in_scope(scope_index, name.clone(), value_type);
                Some(value_type)
            }
            AssignTarget::Member {
                object,
                member,
                member_span,
                ..
            } => {
                let receiver_type = self.check_expr(object, source)?;
                let SemanticType::Struct(receiver_raw) = receiver_type else {
                    self.analyzer.push_type_error(
                        assign.span,
                        source,
                        format!(
                            "Member assignment expects a struct instance, but got {}.",
                            receiver_type.display_name()
                        ),
                    );
                    return None;
                };

                let receiver_id = TypeId(receiver_raw);
                if !self.can_access_private_field(receiver_id) {
                    self.analyzer.push_semantic_error(
                        *member_span,
                        source,
                        format!(
                            "Attribute '{}' is private and cannot be assigned from this context.",
                            member
                        ),
                    );
                    return None;
                }

                let Some(field_type_id) = self.lookup_field_type_id(receiver_id, member) else {
                    self.analyzer.push_semantic_error(
                        *member_span,
                        source,
                        format!("Attribute '{}' is not declared in this type.", member),
                    );
                    return None;
                };

                let expected_type =
                    TypeResolver::type_id_to_semantic_type(self.analyzer, field_type_id);
                let mut value_type = self.check_expr(&assign.value, source)?;

                if value_type == SemanticType::Unknown && expected_type != SemanticType::Unknown {
                    value_type = TypeConstraintEngine::constrain_expr_type(
                        self,
                        &assign.value,
                        expected_type,
                        source,
                    );
                }

                if expected_type != SemanticType::Unknown
                    && value_type != SemanticType::Unknown
                    && expected_type != value_type
                {
                    self.analyzer.push_type_error(
                        assign.span,
                        source,
                        format!(
                            "Destructive assignment ':=' requires type {}, but expression is {}.",
                            expected_type.display_name(),
                            value_type.display_name()
                        ),
                    );
                    return None;
                }

                if expected_type != SemanticType::Unknown {
                    Some(expected_type)
                } else {
                    Some(value_type)
                }
            }
        }
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
        self.analyzer
            .type_table
            .get_struct(receiver)
            .and_then(|info| info.fields.iter().find(|(name, _)| name == field_name))
            .map(|(_, type_id)| *type_id)
    }

    fn can_access_private_field(&self, receiver: TypeId) -> bool {
        self.analyzer.current_method_receiver == Some(receiver)
    }

    fn is_self_binding(&self, name: &str, scope_index: usize) -> bool {
        name == "self" && self.analyzer.current_self_scope_index == Some(scope_index)
    }
}

fn op_symbol(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Pow => "^",
        BinaryOp::Concat => "@",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::Greater => ">",
        BinaryOp::LessEqual => "<=",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

fn is_valid_concat_pair(left: SemanticType, right: SemanticType) -> bool {
    matches!(
        (left, right),
        (SemanticType::String, SemanticType::String)
            | (SemanticType::String, SemanticType::Number)
            | (SemanticType::Number, SemanticType::String)
    )
}

fn is_equality_type(value_type: SemanticType) -> bool {
    matches!(
        value_type,
        SemanticType::Number | SemanticType::Boolean | SemanticType::String
    )
}
