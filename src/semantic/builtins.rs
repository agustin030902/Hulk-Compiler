use crate::parser::expression::{
    BinaryExpr, BinaryOp, BlockExpr, DestructiveAssignExpr, Expr, LetInExpr, LetBinding,
    Literal, MemberAccessExpr, MethodCallExpr, MethodDecl, Span, Statement, TypeAnnotation,
    TypeAttribute, TypeDecl, TypeParam, WhileExpr,
};
use crate::semantic::helper::{FunctionSignature, FunctionSymbol, TypeId};
use crate::semantic::SemanticAnalyzer;

pub(crate) const ITERABLE_NAME: &str = "Iterable";
pub(crate) const RANGE_NAME: &str = "Range";
pub(in crate::semantic) const RANGE_NEXT_METHOD: &str = "next";
pub(in crate::semantic) const RANGE_CURRENT_METHOD: &str = "current";
pub(in crate::semantic) const ITERABLE_NEXT_METHOD: &str = "next";
pub(in crate::semantic) const ITERABLE_CURRENT_METHOD: &str = "current";

fn synthetic_span() -> Span {
    Span::new(0, 0)
}

fn id_span() -> Span {
    Span::new(0, 0)
}

fn annotation(name: &str) -> TypeAnnotation {
    TypeAnnotation {
        name: name.to_string(),
        span: synthetic_span(),
    }
}

fn var(name: &str) -> Expr {
    Expr::Variable {
        name: name.to_string(),
        span: id_span(),
    }
}

fn self_member(member: &str) -> Expr {
    Expr::MemberAccess(MemberAccessExpr {
        object: Box::new(Expr::Variable {
            name: "self".to_string(),
            span: id_span(),
        }),
        member: member.to_string(),
        member_span: id_span(),
        span: synthetic_span(),
    })
}

fn int_literal(value: i64) -> Expr {
    Expr::Literal {
        value: Literal::Integer(value),
        span: id_span(),
    }
}

fn binary(left: Expr, op: BinaryOp, right: Expr) -> Expr {
    Expr::Binary(BinaryExpr {
        left: Box::new(left),
        op,
        right: Box::new(right),
        span: synthetic_span(),
    })
}

fn type_id_to_semantic(analyzer: &SemanticAnalyzer, type_id: TypeId) -> crate::semantic::SemanticType {
    use crate::semantic::SemanticType;
    if type_id == analyzer.type_table.object {
        SemanticType::Struct(analyzer.type_table.object.0)
    } else if type_id == analyzer.type_table.number {
        SemanticType::Number
    } else if type_id == analyzer.type_table.boolean {
        SemanticType::Boolean
    } else if type_id == analyzer.type_table.string {
        SemanticType::String
    } else if type_id == analyzer.type_table.unit {
        SemanticType::Unit
    } else if type_id == analyzer.type_table.null {
        SemanticType::Null
    } else {
        SemanticType::Struct(type_id.0)
    }
}

pub(in crate::semantic) fn register_builtin_iterable(analyzer: &mut SemanticAnalyzer) {
    let iterable_id = analyzer.type_table.iterable;
    analyzer
        .type_symbols
        .insert(ITERABLE_NAME.to_string(), iterable_id);

    let next_return = type_id_to_semantic(analyzer, analyzer.type_table.boolean);
    let current_return = type_id_to_semantic(analyzer, analyzer.type_table.object);

    register_protocol_method(
        analyzer,
        iterable_id,
        ITERABLE_NEXT_METHOD,
        Vec::new(),
        next_return,
    );
    register_protocol_method(
        analyzer,
        iterable_id,
        ITERABLE_CURRENT_METHOD,
        Vec::new(),
        current_return,
    );
}

pub(in crate::semantic) fn register_builtin_range(analyzer: &mut SemanticAnalyzer) {
    let range_id = analyzer.type_table.range;
    let number_id = analyzer.type_table.number;
    let boolean_id = analyzer.type_table.boolean;
    analyzer
        .type_symbols
        .insert(RANGE_NAME.to_string(), range_id);

    if let Some(struct_info) = analyzer.type_table.get_struct_mut(range_id) {
        struct_info.constructor_params = vec![
            ("min".to_string(), number_id),
            ("max".to_string(), number_id),
        ];
        struct_info.fields = vec![
            ("min".to_string(), number_id),
            ("max".to_string(), number_id),
            ("current".to_string(), number_id),
        ];
    }

    let number = type_id_to_semantic(analyzer, number_id);
    let boolean = type_id_to_semantic(analyzer, boolean_id);

    register_type_method(analyzer, range_id, RANGE_NEXT_METHOD, Vec::new(), boolean);
    register_type_method(analyzer, range_id, RANGE_CURRENT_METHOD, Vec::new(), number);
}

fn register_protocol_method(
    analyzer: &mut SemanticAnalyzer,
    receiver: TypeId,
    name: &str,
    param_types: Vec<crate::semantic::SemanticType>,
    return_type: crate::semantic::SemanticType,
) {
    let key = crate::semantic::pipeline::SymbolCollector::method_symbol_key(receiver, name);
    if analyzer.function_symbols.contains_key(&key) {
        return;
    }

    let param_type_ids: Vec<TypeId> = param_types
        .iter()
        .copied()
        .map(|semantic_type| match semantic_type {
            crate::semantic::SemanticType::Number => analyzer.type_table.number,
            crate::semantic::SemanticType::Boolean => analyzer.type_table.boolean,
            crate::semantic::SemanticType::String => analyzer.type_table.string,
            crate::semantic::SemanticType::Unit => analyzer.type_table.unit,
            crate::semantic::SemanticType::Null => analyzer.type_table.null,
            crate::semantic::SemanticType::Struct(id) => TypeId(id),
            _ => analyzer.type_table.unknown,
        })
        .collect();
    let return_type_id = match return_type {
        crate::semantic::SemanticType::Number => analyzer.type_table.number,
        crate::semantic::SemanticType::Boolean => analyzer.type_table.boolean,
        crate::semantic::SemanticType::String => analyzer.type_table.string,
        crate::semantic::SemanticType::Unit => analyzer.type_table.unit,
        crate::semantic::SemanticType::Null => analyzer.type_table.null,
        crate::semantic::SemanticType::Struct(id) => TypeId(id),
        _ => analyzer.type_table.unknown,
    };

    let method_type_id = analyzer
        .type_table
        .register_method(receiver, param_type_ids, return_type_id);

    analyzer.function_symbols.insert(
        key.clone(),
        FunctionSymbol::new_method(name.to_string(), method_type_id, receiver),
    );
    analyzer.functions.insert(
        key.clone(),
        FunctionSignature {
            type_id: method_type_id.0,
            param_names: vec![],
            param_types,
            return_type,
        },
    );

    if let Some(info) = analyzer.type_table.get_struct_mut(receiver) {
        info.methods.push((name.to_string(), method_type_id));
    }
}

fn register_type_method(
    analyzer: &mut SemanticAnalyzer,
    receiver: TypeId,
    name: &str,
    param_types: Vec<crate::semantic::SemanticType>,
    return_type: crate::semantic::SemanticType,
) {
    register_protocol_method(analyzer, receiver, name, param_types, return_type);
}

pub(crate) fn build_object_type_decl() -> TypeDecl {
    TypeDecl {
        name: "Object".to_string(),
        name_span: id_span(),
        params: Vec::new(),
        parent_name: None,
        parent_span: None,
        parent_init_exprs: Vec::new(),
        attributes: Vec::new(),
        methods: Vec::new(),
        span: synthetic_span(),
    }
}

pub(crate) fn build_range_type_decl() -> TypeDecl {
    let min_param = TypeParam {
        name: "min".to_string(),
        type_annotation: Some(annotation("Number")),
        span: synthetic_span(),
    };
    let max_param = TypeParam {
        name: "max".to_string(),
        type_annotation: Some(annotation("Number")),
        span: synthetic_span(),
    };

    let min_attr = TypeAttribute {
        name: "min".to_string(),
        name_span: id_span(),
        type_annotation: Some(annotation("Number")),
        value: var("min"),
        span: synthetic_span(),
    };
    let max_attr = TypeAttribute {
        name: "max".to_string(),
        name_span: id_span(),
        type_annotation: Some(annotation("Number")),
        value: var("max"),
        span: synthetic_span(),
    };
    let current_attr = TypeAttribute {
        name: "current".to_string(),
        name_span: id_span(),
        type_annotation: Some(annotation("Number")),
        value: binary(var("min"), BinaryOp::Sub, int_literal(1)),
        span: synthetic_span(),
    };

    let advance = DestructiveAssignExpr {
        target: crate::parser::expression::AssignTarget::Member {
            object: Box::new(Expr::Variable {
                name: "self".to_string(),
                span: id_span(),
            }),
            member: "current".to_string(),
            member_span: id_span(),
            span: synthetic_span(),
        },
        value: Box::new(binary(
            self_member("current"),
            BinaryOp::Add,
            int_literal(1),
        )),
        span: synthetic_span(),
    };

    let condition = binary(
        self_member("current"),
        BinaryOp::Less,
        self_member("max"),
    );

    let next_body = Expr::Block(BlockExpr {
        statements: vec![
            Statement::Expr {
                value: Expr::DestructiveAssign(advance),
                span: synthetic_span(),
            },
            Statement::Expr {
                value: condition,
                span: synthetic_span(),
            },
        ],
        span: synthetic_span(),
    });

    let next_method = MethodDecl {
        name: RANGE_NEXT_METHOD.to_string(),
        name_span: id_span(),
        params: Vec::new(),
        return_type_annotation: Some(annotation("Boolean")),
        body: next_body,
        span: synthetic_span(),
    };

    let current_method = MethodDecl {
        name: RANGE_CURRENT_METHOD.to_string(),
        name_span: id_span(),
        params: Vec::new(),
        return_type_annotation: Some(annotation("Number")),
        body: self_member("current"),
        span: synthetic_span(),
    };

    TypeDecl {
        name: RANGE_NAME.to_string(),
        name_span: id_span(),
        params: vec![min_param, max_param],
        parent_name: Some("Object".to_string()),
        parent_span: Some(synthetic_span()),
        parent_init_exprs: Vec::new(),
        attributes: vec![min_attr, max_attr, current_attr],
        methods: vec![next_method, current_method],
        span: synthetic_span(),
    }
}

#[allow(dead_code)]
pub(in crate::semantic) fn build_for_desugar(
    iterable_var: &str,
    iter_expr: Expr,
    iter_var: &str,
    body: Expr,
) -> Expr {
    let iter_binding = LetBinding {
        name: iterable_var.to_string(),
        type_annotation: None,
        value: iter_expr,
        span: synthetic_span(),
    };

    let iter_ref = var(iterable_var);
    let condition = Expr::MethodCall(MethodCallExpr {
        receiver: Box::new(iter_ref.clone()),
        method_name: "next".to_string(),
        method_name_span: id_span(),
        args: Vec::new(),
        span: synthetic_span(),
    });
    let current_value = Expr::MethodCall(MethodCallExpr {
        receiver: Box::new(iter_ref),
        method_name: "current".to_string(),
        method_name_span: id_span(),
        args: Vec::new(),
        span: synthetic_span(),
    });

    let inner_binding = LetBinding {
        name: iter_var.to_string(),
        type_annotation: None,
        value: current_value,
        span: synthetic_span(),
    };

    let inner_let_in = Expr::LetIn(LetInExpr {
        bindings: vec![inner_binding],
        body: Box::new(body),
        span: synthetic_span(),
    });

    let body_block = Expr::Block(BlockExpr {
        statements: vec![Statement::Expr {
            value: inner_let_in,
            span: synthetic_span(),
        }],
        span: synthetic_span(),
    });

    let while_loop = Expr::While(WhileExpr {
        condition: Box::new(condition),
        body: match body_block {
            Expr::Block(b) => b,
            _ => unreachable!(),
        },
        span: synthetic_span(),
    });

    Expr::LetIn(LetInExpr {
        bindings: vec![iter_binding],
        body: Box::new(while_loop),
        span: synthetic_span(),
    })
}
