use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_for_expr(
        &mut self,
        for_expr: &ForExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let iter_type = self.check_expr(&for_expr.iter, source)?;

        let element_type = self.resolve_iterable_element_type(&iter_type, for_expr, source)?;

        self.analyzer.push_scope();
        self.analyzer
            .bind_current_scope(for_expr.id.clone(), element_type);
        self.check_block_expr(&for_expr.body, source);
        self.analyzer.pop_scope();

        Some(SemanticType::Unit)
    }

    fn resolve_iterable_element_type(
        &mut self,
        iter_type: &SemanticType,
        for_expr: &ForExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let SemanticType::Struct(raw_id) = iter_type else {
            self.analyzer.push_type_error(
                for_expr.span,
                source,
                "for loop requires an iterable or enumerable type.".to_string(),
            );
            return None;
        };
        let type_id = TypeId(*raw_id);

        let current_key = SymbolCollector::method_symbol_key(type_id, "current");
        if let Some(sig) = self.analyzer.functions.get(&current_key) {
            return Some(sig.return_type.clone());
        }

        let iter_key = SymbolCollector::method_symbol_key(type_id, "iter");
        if let Some(iter_sig) = self.analyzer.functions.get(&iter_key) {
            if let SemanticType::Struct(iter_raw) = iter_sig.return_type {
                let iter_return_id = TypeId(iter_raw);
                let current_key_inner =
                    SymbolCollector::method_symbol_key(iter_return_id, "current");
                if let Some(current_sig) = self.analyzer.functions.get(&current_key_inner) {
                    return Some(current_sig.return_type.clone());
                }
            }
        }

        self.analyzer.push_type_error(
            for_expr.span,
            source,
            format!(
                "Type '{}' is not iterable or enumerable.",
                iter_type.display_name_with_table(&self.analyzer.type_table)
            ),
        );
        None
    }
}
