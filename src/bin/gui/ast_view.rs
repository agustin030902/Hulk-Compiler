//! Vista de árbol del AST con búsqueda: cada nodo se dibuja como un
//! `CollapsingHeader` y las coincidencias con la búsqueda se resaltan.

use eframe::egui::{self, CollapsingHeader};
use hulk_compiler::parser::expression::{
    AsExpr, AssignTarget, BinaryExpr, BinaryOp, BlockExpr, BuiltinCallExpr, DestructiveAssignExpr,
    Expr, FunctionCallExpr, FunctionDecl, IfExpr, IsExpr, LetInExpr, Literal, MemberAccessExpr,
    MethodCallExpr, NewExpr, Program, Span, Statement, UnaryExpr, UnaryOp, WhileExpr,
};

use crate::theme::Theme;

pub fn count_ast_matches(program: &Program, query: &str) -> usize {
    if query.trim().is_empty() {
        return 0;
    }
    let query_lc = query.to_ascii_lowercase();
    let debug_text = format!("{:#?}", program).to_ascii_lowercase();
    debug_text.matches(&query_lc).count()
}

fn match_rich_text(text: impl Into<String>, query: &str, theme: &Theme) -> egui::RichText {
    let text = text.into();
    if query.is_empty() {
        return egui::RichText::new(text).color(theme.text);
    }
    if text
        .to_ascii_lowercase()
        .contains(&query.to_ascii_lowercase())
    {
        egui::RichText::new(text).color(theme.string).strong()
    } else {
        egui::RichText::new(text).color(theme.text_dim)
    }
}

pub fn render_program_tree(ui: &mut egui::Ui, program: &Program, query: &str, theme: &Theme) {
    ui.label(match_rich_text(
        format!(
            "Programa: {} función(es) global(es), {} statement(s) en main",
            program.functions.len(),
            program.statements.len()
        ),
        query,
        theme,
    ));
    ui.separator();

    CollapsingHeader::new(match_rich_text("Funciones globales", query, theme))
        .default_open(true)
        .show(ui, |ui| {
            if program.functions.is_empty() {
                ui.small("Sin funciones declaradas.");
            }
            for (index, function) in program.functions.iter().enumerate() {
                render_function_tree(ui, function, index, query, theme);
            }
        });

    CollapsingHeader::new(match_rich_text("Statements de main", query, theme))
        .default_open(true)
        .show(ui, |ui| {
            if program.statements.is_empty() {
                ui.small("Sin statements globales.");
            }
            for (index, statement) in program.statements.iter().enumerate() {
                render_statement_tree(ui, statement, &format!("main[{index}]"), query, theme);
            }
        });
}

fn render_function_tree(
    ui: &mut egui::Ui,
    function: &FunctionDecl,
    index: usize,
    query: &str,
    theme: &Theme,
) {
    let params = function
        .params
        .iter()
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    CollapsingHeader::new(match_rich_text(
        format!("[{index}] function {}({params})", function.name),
        query,
        theme,
    ))
    .default_open(false)
    .show(ui, |ui| {
        ui.small(match_rich_text(
            format!("span {}", span_text(function.span)),
            query,
            theme,
        ));

        CollapsingHeader::new(match_rich_text("Parámetros", query, theme))
            .default_open(true)
            .show(ui, |ui| {
                if function.params.is_empty() {
                    ui.small("Sin parámetros.");
                }
                for param in &function.params {
                    ui.monospace(format!("{} [{}]", param.name, span_text(param.span)));
                }
            });

        CollapsingHeader::new(match_rich_text("Cuerpo", query, theme))
            .default_open(true)
            .show(ui, |ui| {
                render_expr_tree(ui, &function.body, "body", query, theme);
            });
    });
}

fn render_statement_tree(
    ui: &mut egui::Ui,
    statement: &Statement,
    label: &str,
    query: &str,
    theme: &Theme,
) {
    match statement {
        Statement::Let {
            name, value, span, ..
        } => {
            CollapsingHeader::new(match_rich_text(
                format!("{label}: let {name} = ..."),
                query,
                theme,
            ))
            .default_open(false)
            .show(ui, |ui| {
                ui.small(match_rich_text(
                    format!("span {}", span_text(*span)),
                    query,
                    theme,
                ));
                render_expr_tree(ui, value, "value", query, theme);
            });
        }
        Statement::Assign {
            name, value, span, ..
        } => {
            CollapsingHeader::new(match_rich_text(
                format!("{label}: {name} = ..."),
                query,
                theme,
            ))
            .default_open(false)
            .show(ui, |ui| {
                ui.small(match_rich_text(
                    format!("span {}", span_text(*span)),
                    query,
                    theme,
                ));
                render_expr_tree(ui, value, "value", query, theme);
            });
        }
        Statement::Print { value, span } => {
            CollapsingHeader::new(match_rich_text(format!("{label}: print(...)"), query, theme))
                .default_open(false)
                .show(ui, |ui| {
                    ui.small(match_rich_text(
                        format!("span {}", span_text(*span)),
                        query,
                        theme,
                    ));
                    render_expr_tree(ui, value, "arg", query, theme);
                });
        }
        Statement::Expr { value, span } => {
            CollapsingHeader::new(match_rich_text(
                format!("{label}: expr statement"),
                query,
                theme,
            ))
            .default_open(false)
            .show(ui, |ui| {
                ui.small(match_rich_text(
                    format!("span {}", span_text(*span)),
                    query,
                    theme,
                ));
                render_expr_tree(ui, value, "expr", query, theme);
            });
        }
    }
}

fn render_expr_tree(ui: &mut egui::Ui, expr: &Expr, label: &str, query: &str, theme: &Theme) {
    match expr {
        Expr::Literal { value, span } => {
            ui.label(match_rich_text(
                format!(
                    "{label}: literal {} [{}]",
                    literal_text(value),
                    span_text(*span)
                ),
                query,
                theme,
            ));
        }
        Expr::Variable { name, span } => {
            ui.label(match_rich_text(
                format!("{label}: variable {name} [{}]", span_text(*span)),
                query,
                theme,
            ));
        }
        Expr::Binary(binary) => render_binary_tree(ui, binary, label, query, theme),
        Expr::Unary(unary) => render_unary_tree(ui, unary, label, query, theme),
        Expr::BuiltinCall(call) => render_builtin_call_tree(ui, call, label, query, theme),
        Expr::FunctionCall(call) => render_function_call_tree(ui, call, label, query, theme),
        Expr::MethodCall(call) => render_method_call_tree(ui, call, label, query, theme),
        Expr::MemberAccess(access) => render_member_access_tree(ui, access, label, query, theme),
        Expr::New(new_expr) => render_new_expr_tree(ui, new_expr, label, query, theme),
        Expr::DestructiveAssign(assign) => {
            render_destructive_assign_tree(ui, assign, label, query, theme)
        }
        Expr::LetIn(let_in) => render_let_in_tree(ui, let_in, label, query, theme),
        Expr::Block(block) => render_block_tree(ui, block, label, query, theme),
        Expr::While(while_expr) => render_while_tree(ui, while_expr, label, query, theme),
        Expr::If(if_expr) => render_if_tree(ui, if_expr, label, query, theme),
        Expr::Is(is_expr) => render_is_tree(ui, is_expr, label, query, theme),
        Expr::As(as_expr) => render_as_tree(ui, as_expr, label, query, theme),
        Expr::For(for_expr) => {
            CollapsingHeader::new(match_rich_text(
                format!(
                    "{label}: for ({} in ...) [{}]",
                    for_expr.id,
                    span_text(for_expr.span)
                ),
                query,
                theme,
            ))
            .default_open(true)
            .show(ui, |ui| {
                render_expr_tree(ui, &for_expr.iter, "iterable", query, theme);
                render_block_tree(ui, &for_expr.body, "body", query, theme);
            });
        }
        Expr::BaseCall(call) => {
            CollapsingHeader::new(match_rich_text(
                format!(
                    "{label}: base({} args) [{}]",
                    call.args.len(),
                    span_text(call.span)
                ),
                query,
                theme,
            ))
            .default_open(true)
            .show(ui, |ui| {
                for (i, arg) in call.args.iter().enumerate() {
                    render_expr_tree(ui, arg, &format!("arg[{i}]"), query, theme);
                }
            });
        }
        Expr::ArrayLiteral(literal) => {
            CollapsingHeader::new(match_rich_text(
                format!(
                    "{label}: array literal ({} elems) [{}]",
                    literal.elements.len(),
                    span_text(literal.span)
                ),
                query,
                theme,
            ))
            .default_open(true)
            .show(ui, |ui| {
                for (i, element) in literal.elements.iter().enumerate() {
                    render_expr_tree(ui, element, &format!("elem[{i}]"), query, theme);
                }
            });
        }
        Expr::NewArray(new_array) => {
            CollapsingHeader::new(match_rich_text(
                format!(
                    "{label}: new {}[...] [{}]",
                    new_array.elem_type_name,
                    span_text(new_array.span)
                ),
                query,
                theme,
            ))
            .default_open(true)
            .show(ui, |ui| {
                render_expr_tree(ui, &new_array.size, "size", query, theme);
                if let Some(init) = &new_array.init {
                    render_expr_tree(
                        ui,
                        &init.body,
                        &format!("init {} ->", init.var_name),
                        query,
                        theme,
                    );
                }
            });
        }
        Expr::Index(index_expr) => {
            CollapsingHeader::new(match_rich_text(
                format!("{label}: index [{}]", span_text(index_expr.span)),
                query,
                theme,
            ))
            .default_open(true)
            .show(ui, |ui| {
                render_expr_tree(ui, &index_expr.object, "object", query, theme);
                render_expr_tree(ui, &index_expr.index, "index", query, theme);
            });
        }
        Expr::Lambda(lambda) => {
            let params = lambda
                .params
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            CollapsingHeader::new(match_rich_text(
                format!("{label}: lambda ({params}) [{}]", span_text(lambda.span)),
                query,
                theme,
            ))
            .default_open(true)
            .show(ui, |ui| {
                render_expr_tree(ui, &lambda.body, "body", query, theme);
            });
        }
    }
}

fn render_binary_tree(
    ui: &mut egui::Ui,
    binary: &BinaryExpr,
    label: &str,
    query: &str,
    theme: &Theme,
) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: Binary '{}' [{}]",
            binary_op_symbol(&binary.op),
            span_text(binary.span)
        ),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &binary.left, "left", query, theme);
        render_expr_tree(ui, &binary.right, "right", query, theme);
    });
}

fn render_unary_tree(
    ui: &mut egui::Ui,
    unary: &UnaryExpr,
    label: &str,
    query: &str,
    theme: &Theme,
) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: Unary '{}' [{}]",
            unary_op_symbol(&unary.op),
            span_text(unary.span)
        ),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &unary.expr, "expr", query, theme);
    });
}

fn render_builtin_call_tree(
    ui: &mut egui::Ui,
    call: &BuiltinCallExpr,
    label: &str,
    query: &str,
    theme: &Theme,
) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: Builtin {}(...) [{}]",
            call.function.name(),
            span_text(call.span)
        ),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        if call.args.is_empty() {
            ui.small("Sin argumentos");
        }
        for (idx, arg) in call.args.iter().enumerate() {
            render_expr_tree(ui, arg, &format!("arg[{idx}]"), query, theme);
        }
    });
}

fn render_function_call_tree(
    ui: &mut egui::Ui,
    call: &FunctionCallExpr,
    label: &str,
    query: &str,
    theme: &Theme,
) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: Call {}(...) [{}]",
            call.name,
            span_text(call.span)
        ),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        if call.args.is_empty() {
            ui.small("Sin argumentos");
        }
        for (idx, arg) in call.args.iter().enumerate() {
            render_expr_tree(ui, arg, &format!("arg[{idx}]"), query, theme);
        }
    });
}

fn render_method_call_tree(
    ui: &mut egui::Ui,
    call: &MethodCallExpr,
    label: &str,
    query: &str,
    theme: &Theme,
) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: MethodCall .{}(...) [{}]",
            call.method_name,
            span_text(call.span)
        ),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &call.receiver, "receiver", query, theme);
        if call.args.is_empty() {
            ui.small("Sin argumentos");
        }
        for (idx, arg) in call.args.iter().enumerate() {
            render_expr_tree(ui, arg, &format!("arg[{idx}]"), query, theme);
        }
    });
}

fn render_member_access_tree(
    ui: &mut egui::Ui,
    access: &MemberAccessExpr,
    label: &str,
    query: &str,
    theme: &Theme,
) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: MemberAccess .{} [{}]",
            access.member,
            span_text(access.span)
        ),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &access.object, "object", query, theme);
    });
}

fn render_new_expr_tree(
    ui: &mut egui::Ui,
    new_expr: &NewExpr,
    label: &str,
    query: &str,
    theme: &Theme,
) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: New {}(...) [{}]",
            new_expr.type_name,
            span_text(new_expr.span)
        ),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        if new_expr.args.is_empty() {
            ui.small("Sin argumentos");
        }
        for (idx, arg) in new_expr.args.iter().enumerate() {
            render_expr_tree(ui, arg, &format!("arg[{idx}]"), query, theme);
        }
    });
}

fn render_destructive_assign_tree(
    ui: &mut egui::Ui,
    assign: &DestructiveAssignExpr,
    label: &str,
    query: &str,
    theme: &Theme,
) {
    let target_text = match &assign.target {
        AssignTarget::Variable { name, .. } => name.clone(),
        AssignTarget::Member { member, .. } => format!(".{}", member),
        AssignTarget::Index { .. } => "[index]".to_string(),
    };

    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: DestructiveAssign {} := ... [{}]",
            target_text,
            span_text(assign.span)
        ),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        match &assign.target {
            AssignTarget::Variable { name, .. } => {
                ui.small(match_rich_text(
                    format!("target variable: {name}"),
                    query,
                    theme,
                ));
            }
            AssignTarget::Member { object, member, .. } => {
                ui.small(match_rich_text(
                    format!("target member: .{member}"),
                    query,
                    theme,
                ));
                render_expr_tree(ui, object, "target object", query, theme);
            }
            AssignTarget::Index { object, index, .. } => {
                ui.small(match_rich_text("target index", query, theme));
                render_expr_tree(ui, object, "target object", query, theme);
                render_expr_tree(ui, index, "target index", query, theme);
            }
        }
        render_expr_tree(ui, &assign.value, "value", query, theme);
    });
}

fn render_let_in_tree(
    ui: &mut egui::Ui,
    let_in: &LetInExpr,
    label: &str,
    query: &str,
    theme: &Theme,
) {
    CollapsingHeader::new(match_rich_text(
        format!("{label}: LetIn [{}]", span_text(let_in.span)),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        CollapsingHeader::new(match_rich_text("bindings", query, theme))
            .default_open(true)
            .show(ui, |ui| {
                if let_in.bindings.is_empty() {
                    ui.small("Sin bindings");
                }
                for (idx, binding) in let_in.bindings.iter().enumerate() {
                    CollapsingHeader::new(match_rich_text(
                        format!(
                            "binding[{idx}] {} [{}]",
                            binding.name,
                            span_text(binding.span)
                        ),
                        query,
                        theme,
                    ))
                    .default_open(false)
                    .show(ui, |ui| {
                        render_expr_tree(ui, &binding.value, "value", query, theme);
                    });
                }
            });
        render_expr_tree(ui, &let_in.body, "body", query, theme);
    });
}

fn render_block_tree(
    ui: &mut egui::Ui,
    block: &BlockExpr,
    label: &str,
    query: &str,
    theme: &Theme,
) {
    CollapsingHeader::new(match_rich_text(
        format!("{label}: Block [{}]", span_text(block.span)),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        if block.statements.is_empty() {
            ui.small("Bloque vacío");
        }
        for (idx, statement) in block.statements.iter().enumerate() {
            render_statement_tree(ui, statement, &format!("stmt[{idx}]"), query, theme);
        }
    });
}

fn render_while_tree(
    ui: &mut egui::Ui,
    while_expr: &WhileExpr,
    label: &str,
    query: &str,
    theme: &Theme,
) {
    CollapsingHeader::new(match_rich_text(
        format!("{label}: While [{}]", span_text(while_expr.span)),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &while_expr.condition, "condition", query, theme);
        render_block_tree(ui, &while_expr.body, "body", query, theme);
    });
}

fn render_if_tree(ui: &mut egui::Ui, if_expr: &IfExpr, label: &str, query: &str, theme: &Theme) {
    CollapsingHeader::new(match_rich_text(
        format!("{label}: If [{}]", span_text(if_expr.span)),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &if_expr.condition, "condition", query, theme);
        render_expr_tree(ui, &if_expr.then_branch, "then", query, theme);

        if !if_expr.elif_branches.is_empty() {
            CollapsingHeader::new(match_rich_text("elif branches", query, theme))
                .default_open(true)
                .show(ui, |ui| {
                    for (idx, branch) in if_expr.elif_branches.iter().enumerate() {
                        CollapsingHeader::new(match_rich_text(
                            format!("elif[{idx}] [{}]", span_text(branch.span)),
                            query,
                            theme,
                        ))
                        .default_open(false)
                        .show(ui, |ui| {
                            render_expr_tree(ui, &branch.condition, "condition", query, theme);
                            render_expr_tree(ui, &branch.body, "body", query, theme);
                        });
                    }
                });
        }

        render_expr_tree(ui, &if_expr.else_branch, "else", query, theme);
    });
}

fn render_is_tree(ui: &mut egui::Ui, is_expr: &IsExpr, label: &str, query: &str, theme: &Theme) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: Is '{}' [{}]",
            is_expr.target_type,
            span_text(is_expr.span),
        ),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &is_expr.expr, "expr", query, theme);
    });
}

fn render_as_tree(ui: &mut egui::Ui, as_expr: &AsExpr, label: &str, query: &str, theme: &Theme) {
    CollapsingHeader::new(match_rich_text(
        format!(
            "{label}: As '{}' [{}]",
            as_expr.target_type,
            span_text(as_expr.span),
        ),
        query,
        theme,
    ))
    .default_open(true)
    .show(ui, |ui| {
        render_expr_tree(ui, &as_expr.expr, "expr", query, theme);
    });
}

fn literal_text(literal: &Literal) -> String {
    match literal {
        Literal::Integer(value) => format!("Integer({value})"),
        Literal::Float(value) => format!("Float({value})"),
        Literal::Boolean(value) => format!("Boolean({value})"),
        Literal::String(value) => format!("String(\"{}\")", value.replace('\n', "\\n")),
        Literal::Null => "Null".to_string(),
    }
}

fn binary_op_symbol(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Pow => "^",
        BinaryOp::Concat => "@",
        BinaryOp::ConcatSpace => "@@",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::Greater => ">",
        BinaryOp::LessEqual => "<=",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

fn unary_op_symbol(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
    }
}

fn span_text(span: Span) -> String {
    format!("{}..{}", span.start, span.end)
}
