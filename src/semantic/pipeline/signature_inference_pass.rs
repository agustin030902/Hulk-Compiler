use std::collections::HashMap;

use crate::parser::expression::Program;

use super::super::{
    analyzer::SemanticAnalyzer,
    helper::{FunctionSignature, SemanticType},
};
use super::{SymbolCollector, TypeChecker, TypeResolver};

const MAX_INFERENCE_PASSES: usize = 8;

pub(in crate::semantic) struct SignatureInferencePass;

impl SignatureInferencePass {
    pub(in crate::semantic) fn infer_function_signatures(
        analyzer: &mut SemanticAnalyzer,
        program: &Program,
        source: &str,
    ) -> HashMap<String, FunctionSignature> {
        analyzer.reset_analysis_state();
        analyzer.suppress_errors = true;

        SymbolCollector::collect_types(analyzer, &program.types, source);
        SymbolCollector::collect_functions(analyzer, &program.functions, source);
        SymbolCollector::collect_methods(analyzer, &program.types, source);

        for _ in 0..MAX_INFERENCE_PASSES {
            let before = analyzer.functions.clone();
            analyzer.start_scope_pass();

            {
                let mut checker = TypeChecker::new(analyzer);
                checker.check_program(program, source);
            }

            if analyzer.functions == before {
                break;
            }
        }

        analyzer.suppress_errors = false;
        analyzer.functions.clone()
    }

    pub(in crate::semantic) fn apply_inferred_signatures(
        analyzer: &mut SemanticAnalyzer,
        inferred: &HashMap<String, FunctionSignature>,
    ) {
        for (name, signature) in inferred {
            if let Some(entry) = analyzer.functions.get_mut(name) {
                entry.param_types = signature.param_types.clone();
                entry.return_type = signature.return_type;
            }
        }
        Self::sync_function_type_entries(analyzer);
    }

    pub(in crate::semantic) fn sync_function_type_entries(analyzer: &mut SemanticAnalyzer) {
        let function_names = analyzer
            .function_symbols
            .keys()
            .cloned()
            .collect::<Vec<String>>();

        for name in function_names {
            let Some(symbol) = analyzer.function_symbols.get(&name) else {
                continue;
            };
            let Some(signature) = analyzer.functions.get(&name) else {
                continue;
            };

            let param_type_ids = signature
                .param_types
                .iter()
                .copied()
                .map(|semantic_type| {
                    TypeResolver::semantic_type_to_type_id(analyzer, semantic_type)
                })
                .collect::<Vec<_>>();
            let return_type_id =
                TypeResolver::semantic_type_to_type_id(analyzer, signature.return_type);

            if let Some(function_info) = analyzer.type_table.get_function_mut(symbol.type_id) {
                function_info.params = param_type_ids;
                function_info.return_type = return_type_id;
            }
        }
    }

    pub(in crate::semantic) fn push_unresolved_function_type_errors(
        analyzer: &mut SemanticAnalyzer,
        program: &Program,
        source: &str,
    ) {
        for function in &program.functions {
            let Some(signature) = analyzer.functions.get(&function.name).cloned() else {
                continue;
            };

            for (index, param_type) in signature.param_types.iter().copied().enumerate() {
                if param_type == SemanticType::Unknown {
                    analyzer.push_type_error(
                        function.params[index].span,
                        source,
                        format!(
                            "Could not infer type for parameter '{}' in function '{}'.",
                            function.params[index].name, function.name
                        ),
                    );
                }
            }

            if signature.return_type == SemanticType::Unknown {
                analyzer.push_type_error(
                    function.span,
                    source,
                    format!(
                        "Could not infer return type for function '{}'.",
                        function.name
                    ),
                );
            }
        }

        for type_decl in &program.types {
            let Some(receiver_type_id) = analyzer.type_symbols.get(&type_decl.name).copied() else {
                continue;
            };

            for method in &type_decl.methods {
                let key = SymbolCollector::method_symbol_key(receiver_type_id, &method.name);
                let Some(signature) = analyzer.functions.get(&key).cloned() else {
                    continue;
                };

                for (index, param_type) in signature.param_types.iter().copied().enumerate() {
                    if param_type == SemanticType::Unknown {
                        analyzer.push_type_error(
                            method.params[index].span,
                            source,
                            format!(
                                "Could not infer type for parameter '{}' in method '{}.{}'.",
                                method.params[index].name, type_decl.name, method.name
                            ),
                        );
                    }
                }

                if signature.return_type == SemanticType::Unknown {
                    analyzer.push_type_error(
                        method.span,
                        source,
                        format!(
                            "Could not infer return type for method '{}.{}'.",
                            type_decl.name, method.name
                        ),
                    );
                }
            }
        }
    }
}
