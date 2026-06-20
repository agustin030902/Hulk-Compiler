use crate::parser::expression::{
    FunctionDecl, FunctionParam, InterfaceDecl, Statement, TypeAnnotation, TypeDecl,
};

use super::super::{
    analyzer::SemanticAnalyzer,
    helper::{FunctionSignature, FunctionSymbol, SemanticType, StructTypeInfo, TypeId},
};
use super::{InterfaceChecker, TypeResolver};

pub(in crate::semantic) struct SymbolCollector;

impl SymbolCollector {
    pub(in crate::semantic) fn method_symbol_key(receiver: TypeId, method_name: &str) -> String {
        format!("type#{}::{}", receiver.0, method_name)
    }

    pub(in crate::semantic) fn collect_types(
        analyzer: &mut SemanticAnalyzer,
        type_decls: &[TypeDecl],
        source: &str,
    ) {
        for type_decl in type_decls {
            if SemanticType::from_annotation_name(&type_decl.name).is_some() {
                analyzer.push_semantic_error(
                    type_decl.name_span,
                    source,
                    format!(
                        "Type '{}' cannot be declared because the name is reserved.",
                        type_decl.name
                    ),
                );
                continue;
            }

            if analyzer.type_symbols.contains_key(&type_decl.name) {
                analyzer.push_semantic_error(
                    type_decl.name_span,
                    source,
                    format!("Type '{}' redeclared.", type_decl.name),
                );
                continue;
            }

            let type_id = analyzer.type_table.register_type(StructTypeInfo {
                name: type_decl.name.clone(),
                constructor_params: Vec::new(),
                fields: Vec::new(),
                methods: Vec::new(),
                parent: None,
                is_interface: false,
            });
            analyzer
                .type_symbols
                .insert(type_decl.name.clone(), type_id);
        }

        for type_decl in type_decls {
            let Some(type_id) = analyzer.type_symbols.get(&type_decl.name).copied() else {
                continue;
            };

            let parent_type_id = if let Some(parent_name) = &type_decl.parent_name {
                match analyzer.type_symbols.get(parent_name).copied() {
                    Some(parent_id) => {
                        if parent_id == type_id {
                            if let Some(parent_span) = type_decl.parent_span {
                                analyzer.push_semantic_error(
                                    parent_span,
                                    source,
                                    format!(
                                        "Circular inheritance detected for type '{}'.",
                                        type_decl.name
                                    ),
                                );
                            }
                            None
                        } else {
                            Some(parent_id)
                        }
                    }
                    None => {
                        if let Some(parent_span) = type_decl.parent_span {
                            analyzer.push_semantic_error(
                                parent_span,
                                source,
                                format!("Parent type '{}' not found.", parent_name),
                            );
                        }
                        None
                    }
                }
            } else {
                Some(analyzer.type_table.object)
            };

            if let Some(parent_id) = parent_type_id {
                if Self::is_circular_inheritance(analyzer, parent_id, type_id) {
                    if let Some(parent_span) = type_decl.parent_span {
                        analyzer.push_semantic_error(
                            parent_span,
                            source,
                            format!("Circular inheritance detected for type '{}'.", type_decl.name),
                        );
                    }
                    continue;
                }
            }

            if let Some(struct_info) = analyzer.type_table.get_struct_mut(type_id) {
                struct_info.parent = parent_type_id;
            }
        }

        for type_decl in type_decls {
            let Some(type_id) = analyzer.type_symbols.get(&type_decl.name).copied() else {
                continue;
            };

            let mut constructor_params = Vec::with_capacity(type_decl.params.len());
            for param in &type_decl.params {
                let param_type = param
                    .type_annotation
                    .as_ref()
                    .and_then(|annotation| {
                        TypeResolver::resolve_annotation_type(analyzer, annotation, source)
                    })
                    .unwrap_or(SemanticType::Unknown);
                constructor_params.push((
                    param.name.clone(),
                    TypeResolver::semantic_type_to_type_id(analyzer, param_type),
                ));
            }

            if let Some(struct_info) = analyzer.type_table.get_struct_mut(type_id) {
                struct_info.constructor_params = constructor_params;
            }
        }
    }

    pub(in crate::semantic) fn collect_interfaces(
        analyzer: &mut SemanticAnalyzer,
        interface_decls: &[InterfaceDecl],
        source: &str,
    ) {
        for interface_decl in interface_decls {
            if SemanticType::from_annotation_name(&interface_decl.name).is_some() {
                analyzer.push_semantic_error(
                    interface_decl.name_span,
                    source,
                    format!(
                        "Interface '{}' cannot be declared because the name is reserved.",
                        interface_decl.name
                    ),
                );
                continue;
            }

            if analyzer.type_symbols.contains_key(&interface_decl.name) {
                analyzer.push_semantic_error(
                    interface_decl.name_span,
                    source,
                    format!("Interface '{}' redeclared.", interface_decl.name),
                );
                continue;
            }

            let type_id = analyzer.type_table.register_type(StructTypeInfo {
                name: interface_decl.name.clone(),
                constructor_params: Vec::new(),
                fields: Vec::new(),
                methods: Vec::new(),
                parent: None,
                is_interface: true,
            });
            analyzer
                .type_symbols
                .insert(interface_decl.name.clone(), type_id);
        }

        for interface_decl in interface_decls {
            let Some(interface_id) = analyzer.type_symbols.get(&interface_decl.name).copied()
            else {
                continue;
            };

            let parent_id = if let Some(parent_name) = &interface_decl.parent_name {
                match analyzer.type_symbols.get(parent_name).copied() {
                    Some(parent) => {
                        let is_interface = analyzer
                            .type_table
                            .get_struct(parent)
                            .is_some_and(|info| info.is_interface);
                        if !is_interface {
                            if let Some(parent_span) = interface_decl.parent_span {
                                analyzer.push_semantic_error(
                                    parent_span,
                                    source,
                                    format!(
                                        "Interface '{}' cannot extend type '{}' (only interfaces can be extended).",
                                        interface_decl.name, parent_name
                                    ),
                                );
                            }
                            None
                        } else if parent == interface_id {
                            if let Some(parent_span) = interface_decl.parent_span {
                                analyzer.push_semantic_error(
                                    parent_span,
                                    source,
                                    format!(
                                        "Circular extension detected for interface '{}'.",
                                        interface_decl.name
                                    ),
                                );
                            }
                            None
                        } else if Self::is_circular_inheritance(analyzer, parent, interface_id) {
                            if let Some(parent_span) = interface_decl.parent_span {
                                analyzer.push_semantic_error(
                                    parent_span,
                                    source,
                                    format!(
                                        "Circular extension detected for interface '{}'.",
                                        interface_decl.name
                                    ),
                                );
                            }
                            None
                        } else {
                            Some(parent)
                        }
                    }
                    None => {
                        if let Some(parent_span) = interface_decl.parent_span {
                            analyzer.push_semantic_error(
                                parent_span,
                                source,
                                format!("Parent interface '{}' not found.", parent_name),
                            );
                        }
                        None
                    }
                }
            } else {
                None
            };

            if let Some(parent_id) = parent_id
                && let Some(info) = analyzer.type_table.get_struct_mut(interface_id)
            {
                info.parent = Some(parent_id);
            }
        }
    }

    pub(in crate::semantic) fn is_interface(
        analyzer: &SemanticAnalyzer,
        type_id: TypeId,
    ) -> bool {
        analyzer
            .type_table
            .get_struct(type_id)
            .is_some_and(|info| info.is_interface)
    }

    fn is_circular_inheritance(
        analyzer: &SemanticAnalyzer,
        parent_id: TypeId,
        child_id: TypeId,
    ) -> bool {
        let mut cursor = Some(parent_id);
        while let Some(current) = cursor {
            if current == child_id {
                return true;
            }
            cursor = analyzer
                .type_table
                .get_struct(current)
                .and_then(|info| info.parent);
        }
        false
    }

    pub(in crate::semantic) fn collect_functions(
        analyzer: &mut SemanticAnalyzer,
        functions: &[FunctionDecl],
        source: &str,
    ) {
        for function in functions {
            if analyzer.function_symbols.contains_key(&function.name) {
                analyzer.push_semantic_error(
                    function.name_span,
                    source,
                    format!("Function '{}' redeclared.", function.name),
                );
                continue;
            }

            let param_names: Vec<String> = function
                .params
                .iter()
                .map(|param| param.name.clone())
                .collect();

            let param_types = function
                .params
                .iter()
                .map(|param| {
                    param
                        .type_annotation
                        .as_ref()
                        .and_then(|annotation| {
                            TypeResolver::resolve_annotation_type(analyzer, annotation, source)
                        })
                        .unwrap_or(SemanticType::Unknown)
                })
                .collect::<Vec<_>>();

            let return_type = function
                .return_type_annotation
                .as_ref()
                .and_then(|annotation| {
                    TypeResolver::resolve_annotation_type(analyzer, annotation, source)
                })
                .unwrap_or(SemanticType::Unknown);

            let param_type_ids = param_types
                .iter()
                .cloned()
                .map(|semantic_type| {
                    TypeResolver::semantic_type_to_type_id(analyzer, semantic_type)
                })
                .collect::<Vec<_>>();
            let return_type_id = TypeResolver::semantic_type_to_type_id(analyzer, return_type.clone());
            let function_type_id = analyzer
                .type_table
                .register_plain_function(param_type_ids, return_type_id);

            let signature = FunctionSignature {
                type_id: function_type_id.0,
                param_names,
                param_types,
                return_type,
            };
            analyzer.function_symbols.insert(
                function.name.clone(),
                FunctionSymbol::new_function(function.name.clone(), function_type_id),
            );
            analyzer.functions.insert(function.name.clone(), signature);
        }
    }

    pub(in crate::semantic) fn collect_methods(
        analyzer: &mut SemanticAnalyzer,
        type_decls: &[TypeDecl],
        source: &str,
    ) {
        for type_decl in type_decls {
            let Some(receiver_type_id) = analyzer.type_symbols.get(&type_decl.name).copied() else {
                continue;
            };

            for method in &type_decl.methods {
                let key = Self::method_symbol_key(receiver_type_id, &method.name);
                if analyzer.function_symbols.contains_key(&key) {
                    analyzer.push_semantic_error(
                        method.name_span,
                        source,
                        format!(
                            "Method '{}' redeclared in type '{}'.",
                            method.name, type_decl.name
                        ),
                    );
                    continue;
                }

                let param_names: Vec<String> = method
                    .params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect();

                let param_types = method
                    .params
                    .iter()
                    .map(|param| {
                        param
                            .type_annotation
                            .as_ref()
                            .and_then(|annotation| {
                                TypeResolver::resolve_annotation_type(analyzer, annotation, source)
                            })
                            .unwrap_or(SemanticType::Unknown)
                    })
                    .collect::<Vec<_>>();

                let return_type = method
                    .return_type_annotation
                    .as_ref()
                    .and_then(|annotation| {
                        TypeResolver::resolve_annotation_type(analyzer, annotation, source)
                    })
                    .unwrap_or(SemanticType::Unknown);

                let param_type_ids = param_types
                    .iter()
                    .cloned()
                    .map(|semantic_type| {
                        TypeResolver::semantic_type_to_type_id(analyzer, semantic_type)
                    })
                    .collect::<Vec<_>>();
                let return_type_id = TypeResolver::semantic_type_to_type_id(analyzer, return_type.clone());

                if let Some(parent_signature) =
                    Self::find_method_in_parent(analyzer, receiver_type_id, &method.name)
                {
                    if parent_signature.param_types != param_types
                        || parent_signature.return_type != return_type
                    {
                        analyzer.push_semantic_error(
                            method.name_span,
                            source,
                            format!(
                                "Method '{}' override in type '{}' has different signature than parent.",
                                method.name, type_decl.name
                            ),
                        );
                        continue;
                    }
                }

                let method_type_id = analyzer.type_table.register_method(
                    receiver_type_id,
                    param_type_ids,
                    return_type_id,
                );

                analyzer.function_symbols.insert(
                    key.clone(),
                    FunctionSymbol::new_method(
                        method.name.clone(),
                        method_type_id,
                        receiver_type_id,
                    ),
                );
                analyzer.functions.insert(
                    key.clone(),
                    FunctionSignature {
                        type_id: method_type_id.0,
                        param_names,
                        param_types,
                        return_type,
                    },
                );

                if let Some(info) = analyzer.type_table.get_struct_mut(receiver_type_id) {
                    info.methods.push((method.name.clone(), method_type_id));
                }
            }
        }
    }

    pub(in crate::semantic) fn collect_interface_methods(
        analyzer: &mut SemanticAnalyzer,
        interface_decls: &[InterfaceDecl],
        source: &str,
    ) {
        for interface_decl in interface_decls {
            let Some(receiver_type_id) =
                analyzer.type_symbols.get(&interface_decl.name).copied()
            else {
                continue;
            };

            for method in &interface_decl.methods {
                let key = Self::method_symbol_key(receiver_type_id, &method.name);
                if analyzer.function_symbols.contains_key(&key) {
                    analyzer.push_semantic_error(
                        method.name_span,
                        source,
                        format!(
                            "Method '{}' redeclared in interface '{}'.",
                            method.name, interface_decl.name
                        ),
                    );
                    continue;
                }

                for param in &method.params {
                    if param.type_annotation.is_none() {
                        analyzer.push_semantic_error(
                            param.span,
                            source,
                            format!(
                                "Parameter '{}' in interface method '{}' must have an explicit type annotation.",
                                param.name, method.name
                            ),
                        );
                    }
                }

                let param_names: Vec<String> = method
                    .params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect();

                let param_types = method
                    .params
                    .iter()
                    .map(|param| {
                        param
                            .type_annotation
                            .as_ref()
                            .and_then(|annotation| {
                                TypeResolver::resolve_annotation_type(
                                    analyzer,
                                    annotation,
                                    source,
                                )
                            })
                            .unwrap_or(SemanticType::Unknown)
                    })
                    .collect::<Vec<_>>();

                let return_type = TypeResolver::resolve_annotation_type(
                    analyzer,
                    &method.return_type_annotation,
                    source,
                )
                .unwrap_or(SemanticType::Unknown);

                if return_type == SemanticType::Unknown {
                    analyzer.push_semantic_error(
                        method.return_type_annotation.span,
                        source,
                        format!(
                            "Interface method '{}' must declare a fully resolvable return type.",
                            method.name
                        ),
                    );
                }

                let param_type_ids = param_types
                    .iter()
                    .cloned()
                    .map(|semantic_type| {
                        TypeResolver::semantic_type_to_type_id(analyzer, semantic_type)
                    })
                    .collect::<Vec<_>>();
                let return_type_id =
                    TypeResolver::semantic_type_to_type_id(analyzer, return_type.clone());

                let method_type_id = analyzer.type_table.register_method(
                    receiver_type_id,
                    param_type_ids,
                    return_type_id,
                );

                analyzer.function_symbols.insert(
                    key.clone(),
                    FunctionSymbol::new_method(
                        method.name.clone(),
                        method_type_id,
                        receiver_type_id,
                    ),
                );
                analyzer.functions.insert(
                    key.clone(),
                    FunctionSignature {
                        type_id: method_type_id.0,
                        param_names,
                        param_types,
                        return_type,
                    },
                );

                if let Some(info) = analyzer.type_table.get_struct_mut(receiver_type_id) {
                    info.methods.push((method.name.clone(), method_type_id));
                }
            }
        }

        InterfaceChecker::check_interface_variance(analyzer, interface_decls, source);
    }

    fn find_method_in_parent(
        analyzer: &SemanticAnalyzer,
        type_id: TypeId,
        method_name: &str,
    ) -> Option<FunctionSignature> {
        let parent_id = analyzer.type_table.get_struct(type_id)?.parent?;
        let key = Self::method_symbol_key(parent_id, method_name);
        if let Some(signature) = analyzer.functions.get(&key) {
            return Some(signature.clone());
        }
        Self::find_method_in_parent(analyzer, parent_id, method_name)
    }

    pub(in crate::semantic) fn inject_splat_interfaces(
        analyzer: &mut SemanticAnalyzer,
        program: &crate::parser::expression::Program,
    ) {
        let mut splat_types: std::collections::HashSet<String> = std::collections::HashSet::new();

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

            let return_type = Self::resolve_base_type(analyzer, base_type);

            let return_type_id =
                crate::semantic::pipeline::TypeResolver::semantic_type_to_type_id(
                    analyzer,
                    return_type,
                );

            let method_type_id = analyzer
                .type_table
                .register_method(interface_id, Vec::new(), return_type_id);

            let key = Self::method_symbol_key(interface_id, "current");
            analyzer.function_symbols.insert(
                key.clone(),
                FunctionSymbol::new_method("current".to_string(), method_type_id, interface_id),
            );
            analyzer.functions.insert(
                key.clone(),
                FunctionSignature {
                    type_id: method_type_id.0,
                    param_names: vec![],
                    param_types: vec![],
                    return_type: crate::semantic::pipeline::TypeResolver::type_id_to_semantic_type(
                        analyzer, return_type_id,
                    ),
                },
            );

            if let Some(info) = analyzer.type_table.get_struct_mut(interface_id) {
                info.methods
                    .push(("current".to_string(), method_type_id));
            }
        }
    }

    fn collect_splat_annotations_program(
        program: &crate::parser::expression::Program,
        splat_types: &mut std::collections::HashSet<String>,
    ) {
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
                    splat_types
                        .insert(method.return_type_annotation.name.clone());
                }
            }
        }

        Self::collect_splat_annotations_statements(&program.statements, splat_types);
    }

    fn collect_splat_annotations_statements(
        statements: &[Statement],
        splat_types: &mut std::collections::HashSet<String>,
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

    fn collect_splat_annotations_expr(
        expr: &crate::parser::expression::Expr,
        splat_types: &mut std::collections::HashSet<String>,
    ) {
        use crate::parser::expression::Expr;
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
                Self::collect_splat_annotations_statements(&while_expr.body.statements, splat_types);
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
        splat_types: &mut std::collections::HashSet<String>,
    ) {
        for param in params {
            if let Some(ann) = &param.type_annotation {
                if ann.is_splat {
                    splat_types.insert(ann.name.clone());
                }
            }
        }
    }

    fn resolve_base_type(
        analyzer: &SemanticAnalyzer,
        type_name: &str,
    ) -> crate::semantic::SemanticType {
        if let Some(primitive) =
            crate::semantic::SemanticType::from_annotation_name(type_name)
        {
            return primitive;
        }

        analyzer
            .type_symbols
            .get(type_name)
            .copied()
            .map(|type_id| crate::semantic::SemanticType::Struct(type_id.0))
            .unwrap_or(crate::semantic::SemanticType::Unknown)
    }
}
