//! Síntesis de interfaces splat: por cada anotación `T*` que aparece en el
//! programa se registra una interfaz `Iterable_T extends Iterable` cuyo
//! `current(): T` refina el retorno. Así el resto del pipeline trata `T*`
//! como una interfaz normal.
//!
//! Debe ejecutarse **antes** de resolver cualquier anotación splat (el
//! `TypeResolver` traduce `T*` al nombre `Iterable_T`, que tiene que existir).

use std::collections::HashSet;

use crate::parser::expression::{Expr, FunctionParam, Program, Statement};
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::semantic::helper::{SemanticType, StructTypeInfo};

use super::signature_collector::SignatureParts;
use super::SymbolCollector;

impl SymbolCollector {
    pub(in crate::semantic) fn inject_splat_interfaces(
        analyzer: &mut SemanticAnalyzer,
        program: &Program,
    ) {
        let mut splat_types: HashSet<String> = HashSet::new();
        Self::collect_splat_annotations_program(program, &mut splat_types);

        for base_type in &splat_types {
            let interface_name = format!("Iterable_{}", base_type);

            if analyzer.type_symbols.contains_key(&interface_name) {
                continue;
            }

            let iterable_id = analyzer.type_table.iterable;
            let interface_id = analyzer.type_table.register_type(StructTypeInfo {
                name: interface_name.clone(),
                constructor_params: Vec::new(),
                fields: Vec::new(),
                methods: Vec::new(),
                parent: Some(iterable_id),
                is_interface: true,
            });
            analyzer
                .type_symbols
                .insert(interface_name.clone(), interface_id);

            // `current(): T` refinado; `next()` se hereda de Iterable.
            let return_type = Self::resolve_base_type(analyzer, base_type);
            let return_type_id = crate::semantic::pipeline::TypeResolver::semantic_type_to_type_id(
                analyzer,
                return_type,
            );

            Self::register_method_symbol(
                analyzer,
                interface_id,
                "current",
                SignatureParts {
                    param_names: Vec::new(),
                    param_types: Vec::new(),
                    param_type_ids: Vec::new(),
                    return_type,
                    return_type_id,
                },
            );
        }
    }

    fn resolve_base_type(analyzer: &SemanticAnalyzer, type_name: &str) -> SemanticType {
        if let Some(primitive) = SemanticType::from_annotation_name(type_name) {
            return primitive;
        }

        analyzer
            .type_symbols
            .get(type_name)
            .copied()
            .map(|type_id| SemanticType::Struct(type_id.0))
            .unwrap_or(SemanticType::Unknown)
    }

    // ── Escaneo del AST buscando anotaciones `T*` ────────────────────────

    fn collect_splat_annotations_program(program: &Program, splat_types: &mut HashSet<String>) {
        for function in &program.functions {
            Self::collect_splat_annotations_params(&function.params, splat_types);
            if let Some(ret) = &function.return_type_annotation {
                if ret.is_splat {
                    splat_types.insert(ret.name.clone());
                }
            }
        }

        for type_decl in &program.types {
            for param in &type_decl.params {
                if let Some(ann) = &param.type_annotation {
                    if ann.is_splat {
                        splat_types.insert(ann.name.clone());
                    }
                }
            }
            for member in &type_decl.methods {
                Self::collect_splat_annotations_params(&member.params, splat_types);
                if let Some(ret) = &member.return_type_annotation {
                    if ret.is_splat {
                        splat_types.insert(ret.name.clone());
                    }
                }
            }
        }

        for iface in &program.interfaces {
            for method in &iface.methods {
                Self::collect_splat_annotations_params(&method.params, splat_types);
                if method.return_type_annotation.is_splat {
                    splat_types.insert(method.return_type_annotation.name.clone());
                }
            }
        }

        Self::collect_splat_annotations_statements(&program.statements, splat_types);
    }

    fn collect_splat_annotations_statements(
        statements: &[Statement],
        splat_types: &mut HashSet<String>,
    ) {
        for stmt in statements {
            match stmt {
                Statement::Let {
                    type_annotation,
                    value,
                    ..
                } => {
                    if let Some(ann) = type_annotation {
                        if ann.is_splat {
                            splat_types.insert(ann.name.clone());
                        }
                    }
                    Self::collect_splat_annotations_expr(value, splat_types);
                }
                Statement::Print { value, .. } => {
                    Self::collect_splat_annotations_expr(value, splat_types);
                }
                Statement::Expr { value, .. } => {
                    Self::collect_splat_annotations_expr(value, splat_types);
                }
                Statement::Assign { value, .. } => {
                    Self::collect_splat_annotations_expr(value, splat_types);
                }
            }
        }
    }

    fn collect_splat_annotations_expr(expr: &Expr, splat_types: &mut HashSet<String>) {
        match expr {
            Expr::LetIn(let_in) => {
                for binding in &let_in.bindings {
                    if let Some(ann) = &binding.type_annotation {
                        if ann.is_splat {
                            splat_types.insert(ann.name.clone());
                        }
                    }
                    Self::collect_splat_annotations_expr(&binding.value, splat_types);
                }
                Self::collect_splat_annotations_expr(&let_in.body, splat_types);
            }
            Expr::Block(block) => {
                Self::collect_splat_annotations_statements(&block.statements, splat_types);
            }
            Expr::If(if_expr) => {
                Self::collect_splat_annotations_expr(&if_expr.condition, splat_types);
                Self::collect_splat_annotations_expr(&if_expr.then_branch, splat_types);
                Self::collect_splat_annotations_expr(&if_expr.else_branch, splat_types);
            }
            Expr::While(while_expr) => {
                Self::collect_splat_annotations_expr(&while_expr.condition, splat_types);
                Self::collect_splat_annotations_statements(
                    &while_expr.body.statements,
                    splat_types,
                );
            }
            Expr::FunctionCall(call) => {
                for arg in &call.args {
                    Self::collect_splat_annotations_expr(arg, splat_types);
                }
            }
            Expr::MethodCall(call) => {
                Self::collect_splat_annotations_expr(&call.receiver, splat_types);
                for arg in &call.args {
                    Self::collect_splat_annotations_expr(arg, splat_types);
                }
            }
            Expr::Binary(bin) => {
                Self::collect_splat_annotations_expr(&bin.left, splat_types);
                Self::collect_splat_annotations_expr(&bin.right, splat_types);
            }
            Expr::Unary(unary) => {
                Self::collect_splat_annotations_expr(&unary.expr, splat_types);
            }
            Expr::MemberAccess(access) => {
                Self::collect_splat_annotations_expr(&access.object, splat_types);
            }
            Expr::New(new_expr) => {
                for arg in &new_expr.args {
                    Self::collect_splat_annotations_expr(arg, splat_types);
                }
            }
            Expr::Is(is_expr) => {
                Self::collect_splat_annotations_expr(&is_expr.expr, splat_types);
            }
            Expr::As(as_expr) => {
                Self::collect_splat_annotations_expr(&as_expr.expr, splat_types);
            }
            Expr::DestructiveAssign(da) => {
                Self::collect_splat_annotations_expr(&da.value, splat_types);
            }
            _ => {}
        }
    }

    fn collect_splat_annotations_params(
        params: &[FunctionParam],
        splat_types: &mut HashSet<String>,
    ) {
        for param in params {
            if let Some(ann) = &param.type_annotation {
                if ann.is_splat {
                    splat_types.insert(ann.name.clone());
                }
            }
        }
    }
}
