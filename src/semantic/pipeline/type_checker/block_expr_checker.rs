use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_block_expr(
        &mut self,
        block: &BlockExpr,
        source: &str,
    ) -> Option<SemanticType> {
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
}
