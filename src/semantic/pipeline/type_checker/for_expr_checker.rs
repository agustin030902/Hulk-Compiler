use super::*;
use crate::semantic::helper::TypeTable;

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

        if let Some(element_type) = self.try_iterable(type_id, for_expr, source) {
            return Some(element_type);
        }

        if let Some(element_type) = self.try_enumerable(type_id, for_expr, source) {
            return Some(element_type);
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

    fn try_iterable(
        &mut self,
        type_id: TypeId,
        for_expr: &ForExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let current_sig = self.lookup_method_in_hierarchy(type_id, "current")?;

        if !current_sig.param_types.is_empty() {
            self.analyzer.push_type_error(
                for_expr.span,
                source,
                format!(
                    "Method 'current()' of type '{}' must take no parameters.",
                    iter_type_name(type_id, &self.analyzer.type_table),
                ),
            );
            return None;
        }

        let next_sig = match self.lookup_method_in_hierarchy(type_id, "next") {
            Some(sig) => sig,
            None => {
                self.analyzer.push_type_error(
                    for_expr.span,
                    source,
                    format!(
                        "Type '{}' implements 'current()' but is missing 'next(): Boolean'.",
                        iter_type_name(type_id, &self.analyzer.type_table),
                    ),
                );
                return None;
            }
        };
        if !next_sig.param_types.is_empty() || next_sig.return_type != SemanticType::Boolean {
            self.analyzer.push_type_error(
                for_expr.span,
                source,
                format!(
                    "Type '{}' implements 'current()' but 'next()' has wrong signature (expected '(): Boolean').",
                    iter_type_name(type_id, &self.analyzer.type_table),
                ),
            );
            return None;
        }

        Some(current_sig.return_type)
    }

    fn try_enumerable(
        &mut self,
        type_id: TypeId,
        for_expr: &ForExpr,
        source: &str,
    ) -> Option<SemanticType> {
        let iter_sig = self.lookup_method_in_hierarchy(type_id, "iter")?;

        if !iter_sig.param_types.is_empty() {
            self.analyzer.push_type_error(
                for_expr.span,
                source,
                format!(
                    "Method 'iter()' of type '{}' must take no parameters.",
                    iter_type_name(type_id, &self.analyzer.type_table),
                ),
            );
            return None;
        }

        let SemanticType::Struct(iter_raw) = iter_sig.return_type else {
            self.analyzer.push_type_error(
                for_expr.span,
                source,
                format!(
                    "Method 'iter()' of type '{}' must return an iterable type.",
                    iter_type_name(type_id, &self.analyzer.type_table),
                ),
            );
            return None;
        };

        let iter_return_id = TypeId(iter_raw);

        let next_sig = match self.lookup_method_in_hierarchy(iter_return_id, "next") {
            Some(sig) => sig,
            None => {
                self.analyzer.push_type_error(
                    for_expr.span,
                    source,
                    format!(
                        "Type '{}' is enumerable but its iterator is missing 'next(): Boolean'.",
                        iter_type_name(type_id, &self.analyzer.type_table),
                    ),
                );
                return None;
            }
        };
        if !next_sig.param_types.is_empty() || next_sig.return_type != SemanticType::Boolean {
            self.analyzer.push_type_error(
                for_expr.span,
                source,
                format!(
                    "Type '{}' is enumerable but its iterator's 'next()' has wrong signature (expected '(): Boolean').",
                    iter_type_name(type_id, &self.analyzer.type_table),
                ),
            );
            return None;
        }

        let current_sig = match self.lookup_method_in_hierarchy(iter_return_id, "current") {
            Some(sig) => sig,
            None => {
                self.analyzer.push_type_error(
                    for_expr.span,
                    source,
                    format!(
                        "Type '{}' is enumerable but its iterator is missing 'current()'.",
                        iter_type_name(type_id, &self.analyzer.type_table),
                    ),
                );
                return None;
            }
        };
        if !current_sig.param_types.is_empty() {
            self.analyzer.push_type_error(
                for_expr.span,
                source,
                format!(
                    "Type '{}' is enumerable but its iterator's 'current()' must take no parameters.",
                    iter_type_name(type_id, &self.analyzer.type_table),
                ),
            );
            return None;
        }

        Some(current_sig.return_type)
    }

    fn lookup_method_in_hierarchy(
        &self,
        type_id: TypeId,
        method_name: &str,
    ) -> Option<FunctionSignature> {
        let mut cursor = Some(type_id);
        while let Some(current) = cursor {
            let key = SymbolCollector::method_symbol_key(current, method_name);
            if let Some(sig) = self.analyzer.functions.get(&key) {
                return Some(sig.clone());
            }
            cursor = self
                .analyzer
                .type_table
                .get_struct(current)
                .and_then(|info| info.parent);
        }
        None
    }
}

fn iter_type_name(type_id: TypeId, table: &TypeTable) -> String {
    table
        .get_struct(type_id)
        .map(|info| info.name.clone())
        .unwrap_or_else(|| "Unknown".to_string())
}
