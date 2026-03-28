use crate::parser::expression::BlockExpr;

use super::super::{SemanticType, analyzer::SemanticAnalyzer};

impl SemanticAnalyzer {
    pub(super) fn check_block_expr(
        &mut self,
        block: &BlockExpr,
        source: &str,
    ) -> Option<SemanticType> {
        self.push_scope();

        let mut last_type: Option<SemanticType> = None;
        for statement in &block.statements {
            let stmt_type = self.check_statement(statement, source);
            if stmt_type.is_some() {
                last_type = stmt_type;
            }
        }

        self.pop_scope();

        if block.statements.is_empty() {
            Some(SemanticType::Unknown)
        } else {
            last_type
        }
    }
}
