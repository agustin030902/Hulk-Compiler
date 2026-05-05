use crate::parser::expression::FunctionCallExpr;

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_function_call(
        &mut self,
        call: &FunctionCallExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let Some(expected_arity) = self.functions.get(&call.name).copied() else {
            self.push_semantic_error(
                call.name_span,
                source,
                format!("Function '{}' is called before declaration.", call.name),
            );
            return None;
        };

        if expected_arity != call.args.len() {
            self.push_semantic_error(
                call.span,
                source,
                format!(
                    "Function '{}' expects {} argument(s), but got {}.",
                    call.name,
                    expected_arity,
                    call.args.len()
                ),
            );
            return None;
        }

        for arg in &call.args {
            let _ = self.check_expr(arg, source);
        }

        Some(SemanticType::Unknown)
    }
}
