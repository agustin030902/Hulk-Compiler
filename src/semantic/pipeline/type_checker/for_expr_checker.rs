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

    fn find_method(
        &self,
        type_id: TypeId,
        method_name: &str,
    ) -> Option<FunctionSignature> {
        let key = SymbolCollector::method_symbol_key(type_id, method_name);
        if let Some(sig) = self.analyzer.functions.get(&key) {
            return Some(sig.clone());
        }
        SymbolCollector::find_method_in_parent(&self.analyzer, type_id, method_name)
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

        if let Some(current_sig) = self.find_method(type_id, "current") {
            if !current_sig.param_types.is_empty() {
                self.analyzer.push_type_error(
                    for_expr.span,
                    source,
                    "Iterable method 'current()' must take no parameters.".to_string(),
                );
                return None;
            }

            if let Some(next_sig) = self.find_method(type_id, "next") {
                if !next_sig.param_types.is_empty() {
                    self.analyzer.push_type_error(
                        for_expr.span,
                        source,
                        "Iterable method 'next()' must take no parameters.".to_string(),
                    );
                    return None;
                }
                if next_sig.return_type != SemanticType::Boolean {
                    self.analyzer.push_type_error(
                        for_expr.span,
                        source,
                        "Iterable method 'next()' must return Boolean.".to_string(),
                    );
                    return None;
                }

                return Some(current_sig.return_type.clone());
            }

            self.analyzer.push_type_error(
                for_expr.span,
                source,
                format!(
                    "Type '{}' has 'current()' but is missing 'next()' required for iteration.",
                    iter_type.display_name_with_table(&self.analyzer.type_table)
                ),
            );
            return None;
        }

        let iter_key = SymbolCollector::method_symbol_key(type_id, "iter");
        if let Some(iter_sig) = self.analyzer.functions.get(&iter_key) {
            if !iter_sig.param_types.is_empty() {
                self.analyzer.push_type_error(
                    for_expr.span,
                    source,
                    "Enumerable method 'iter()' must take no parameters.".to_string(),
                );
                return None;
            }

            if let SemanticType::Struct(iter_raw) = iter_sig.return_type {
                let iter_return_id = TypeId(iter_raw);

                if let Some(inner_current_sig) =
                    self.find_method(iter_return_id, "current")
                {
                    if !inner_current_sig.param_types.is_empty() {
                        self.analyzer.push_type_error(
                            for_expr.span,
                            source,
                            "Iterator method 'current()' must take no parameters.".to_string(),
                        );
                        return None;
                    }

                    if let Some(inner_next_sig) =
                        self.find_method(iter_return_id, "next")
                    {
                        if !inner_next_sig.param_types.is_empty() {
                            self.analyzer.push_type_error(
                                for_expr.span,
                                source,
                                "Iterator method 'next()' must take no parameters.".to_string(),
                            );
                            return None;
                        }
                        if inner_next_sig.return_type != SemanticType::Boolean {
                            self.analyzer.push_type_error(
                                for_expr.span,
                                source,
                                "Iterator method 'next()' must return Boolean.".to_string(),
                            );
                            return None;
                        }

                        return Some(inner_current_sig.return_type.clone());
                    }

                    self.analyzer.push_type_error(
                        for_expr.span,
                        source,
                        format!(
                            "Iterator type '{}' returned by 'iter()' is missing 'next()' method.",
                            iter_sig
                                .return_type
                                .display_name_with_table(&self.analyzer.type_table)
                        ),
                    );
                    return None;
                }
            }

            self.analyzer.push_type_error(
                for_expr.span,
                source,
                format!(
                    "Enumerable method 'iter()' must return an Iterable type, but got {}.",
                    iter_sig
                        .return_type
                        .display_name_with_table(&self.analyzer.type_table)
                ),
            );
            return None;
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
