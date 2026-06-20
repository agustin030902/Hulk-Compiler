use crate::parser::expression::{BlockExpr, Expr, IfExpr, LetInExpr, Span, Statement};

use super::super::helper::{SemanticType, TypeId};
use super::TypeChecker;

pub(in crate::semantic) struct TypeConstraintEngine;

impl TypeConstraintEngine {
    pub(in crate::semantic) fn merge_types(
        current: SemanticType,
        inferred: SemanticType,
    ) -> Result<SemanticType, (SemanticType, SemanticType)> {
        match (current, inferred) {
            (SemanticType::Unknown, known) => Ok(known),
            (known, SemanticType::Unknown) => Ok(known),
            (left, right) if left == right => Ok(left),
            (left, SemanticType::Null) if left.is_nullable() => Ok(left),
            (SemanticType::Null, right) if right.is_nullable() => Ok(right),
            (left, right) => Err((left, right)),
        }
    }

    pub(in crate::semantic) fn constrain_expr_type(
        checker: &mut TypeChecker<'_>,
        expr: &Expr,
        expected: SemanticType,
        source: &str,
    ) -> SemanticType {
        match expr {
            Expr::Variable { name, span } => {
                Self::constrain_variable_type(checker, name, expected, *span, source)
            }
            Expr::FunctionCall(call) => Self::constrain_function_return_type(
                checker,
                &call.name,
                expected,
                call.name_span,
                source,
            ),
            Expr::MethodCall(call) => {
                let receiver_type = checker
                    .check_expr(&call.receiver, source)
                    .unwrap_or(SemanticType::Unknown);
                if let SemanticType::Struct(type_id) = receiver_type
                    && let Some(key) =
                        checker.resolve_method_symbol_key(TypeId(type_id), &call.method_name)
                {
                    return Self::constrain_function_return_type(
                        checker,
                        &key,
                        expected,
                        call.method_name_span,
                        source,
                    );
                }
                expected
            }
            Expr::Block(block) => {
                Self::constrain_block_result_type(checker, block, expected, source)
            }
            Expr::LetIn(let_in) => {
                Self::constrain_let_in_result_type(checker, let_in, expected, source)
            }
            Expr::If(if_expr) => Self::constrain_if_result_type(checker, if_expr, expected, source),
            _ => expected,
        }
    }

    pub(in crate::semantic) fn constrain_variable_type(
        checker: &mut TypeChecker<'_>,
        name: &str,
        expected: SemanticType,
        span: Span,
        source: &str,
    ) -> SemanticType {
        let Some((scope_index, current_type)) = checker.analyzer.lookup_with_scope_index(name)
        else {
            return SemanticType::Unknown;
        };

        match Self::merge_types(current_type.clone(), expected) {
            Ok(merged) => {
                if merged != current_type {
                    checker
                        .analyzer
                        .assign_in_scope(scope_index, name.to_string(), merged.clone());
                }
                merged
            }
            Err((left, right)) => {
                checker.analyzer.push_type_error(
                    span,
                    source,
                    format!(
                        "Type inference conflict for variable '{}': {} vs {}.",
                        name,
                        left.display_name_with_table(&checker.analyzer.type_table),
                        right.display_name_with_table(&checker.analyzer.type_table)
                    ),
                );
                current_type
            }
        }
    }

    pub(in crate::semantic) fn constrain_function_param_type(
        checker: &mut TypeChecker<'_>,
        function_name: &str,
        param_index: usize,
        inferred: SemanticType,
        span: Span,
        source: &str,
    ) -> SemanticType {
        let Some(current_type) = checker
            .analyzer
            .functions
            .get(function_name)
            .and_then(|signature| signature.param_types.get(param_index).cloned())
        else {
            return SemanticType::Unknown;
        };

        match Self::merge_types(current_type.clone(), inferred) {
            Ok(merged) => {
                if merged != current_type
                    && let Some(signature) = checker.analyzer.functions.get_mut(function_name)
                {
                    signature.param_types[param_index] = merged.clone();
                }
                merged
            }
            Err((left, right)) => {
                checker.analyzer.push_type_error(
                    span,
                    source,
                    format!(
                        "Function '{}' argument #{} has conflicting types: {} vs {}.",
                        function_name,
                        param_index + 1,
                        left.display_name_with_table(&checker.analyzer.type_table),
                        right.display_name_with_table(&checker.analyzer.type_table)
                    ),
                );
                current_type
            }
        }
    }

    pub(in crate::semantic) fn constrain_function_return_type(
        checker: &mut TypeChecker<'_>,
        function_name: &str,
        inferred: SemanticType,
        span: Span,
        source: &str,
    ) -> SemanticType {
        let Some(current_type) = checker
            .analyzer
            .functions
            .get(function_name)
            .map(|signature| signature.return_type.clone())
        else {
            return SemanticType::Unknown;
        };

        match Self::merge_types(current_type.clone(), inferred) {
            Ok(merged) => {
                if merged != current_type
                    && let Some(signature) = checker.analyzer.functions.get_mut(function_name)
                {
                    signature.return_type = merged.clone();
                }
                merged
            }
            Err((left, right)) => {
                checker.analyzer.push_type_error(
                    span,
                    source,
                    format!(
                        "Function '{}' return type conflict: {} vs {}.",
                        function_name,
                        left.display_name_with_table(&checker.analyzer.type_table),
                        right.display_name_with_table(&checker.analyzer.type_table)
                    ),
                );
                current_type
            }
        }
    }

    fn constrain_statement_result_type(
        checker: &mut TypeChecker<'_>,
        statement: &Statement,
        expected: SemanticType,
        source: &str,
    ) -> SemanticType {
        match statement {
            Statement::Expr { value, .. }
            | Statement::Let { value, .. }
            | Statement::Assign { value, .. } => {
                Self::constrain_expr_type(checker, value, expected, source)
            }
            Statement::Print { .. } => SemanticType::Unit,
        }
    }

    fn constrain_block_result_type(
        checker: &mut TypeChecker<'_>,
        block: &BlockExpr,
        expected: SemanticType,
        source: &str,
    ) -> SemanticType {
        let Some(last_statement) = block.statements.last() else {
            return SemanticType::Unit;
        };
        Self::constrain_statement_result_type(checker, last_statement, expected, source)
    }

    fn constrain_let_in_result_type(
        checker: &mut TypeChecker<'_>,
        let_in: &LetInExpr,
        expected: SemanticType,
        source: &str,
    ) -> SemanticType {
        Self::constrain_expr_type(checker, &let_in.body, expected, source)
    }

    fn constrain_if_result_type(
        checker: &mut TypeChecker<'_>,
        if_expr: &IfExpr,
        expected: SemanticType,
        source: &str,
    ) -> SemanticType {
        let _ =
            Self::constrain_expr_type(checker, &if_expr.condition, SemanticType::Boolean, source);
        let _ = Self::constrain_expr_type(checker, &if_expr.then_branch, expected.clone(), source);
        for branch in &if_expr.elif_branches {
            let _ = Self::constrain_expr_type(
                checker,
                &branch.condition,
                SemanticType::Boolean,
                source,
            );
            let _ = Self::constrain_expr_type(checker, &branch.body, expected.clone(), source);
        }
        let _ = Self::constrain_expr_type(checker, &if_expr.else_branch, expected.clone(), source);
        expected
    }
}
