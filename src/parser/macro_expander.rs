//! Expansión de macros `define` a nivel de AST.
//!
//! Las macros se expanden por sustitución textual (call-by-name): cada
//! ocurrencia de un parámetro dentro del cuerpo se reemplaza por la expresión
//! del argumento sin evaluarla, de modo que `repeat(5, count := count + 1)`
//! re-evalúa el argumento en cada iteración del cuerpo. Para mantener la
//! higiene, todas las variables ligadas dentro del cuerpo de la macro
//! (let-in, let de bloque, variables de `for`) se renombran a nombres frescos,
//! evitando capturas con variables del sitio de llamada.
//!
//! La expansión ocurre después del parseo y antes del análisis semántico, así
//! que las fases posteriores solo ven HULK plano.

use std::collections::HashMap;

use crate::error::{CompilerError, ErrorCategory};

use super::expression::{
    AssignTarget, Expr, MacroDecl, Program, Statement,
};

const MAX_EXPANSION_DEPTH: usize = 64;

pub struct MacroExpander {
    macros: HashMap<String, MacroDecl>,
    fresh_counter: usize,
    errors: Vec<CompilerError>,
}

/// Sustitución activa para un nombre dentro del cuerpo de una macro.
#[derive(Clone)]
enum Binding {
    /// Parámetro de la macro: se reemplaza por la expresión del argumento.
    Param(Expr),
    /// Variable local de la macro: se renombra a un nombre fresco.
    Renamed(String),
}

impl MacroExpander {
    pub fn expand_program(program: &mut Program) -> Vec<CompilerError> {
        let macros = std::mem::take(&mut program.macros);
        if macros.is_empty() {
            return Vec::new();
        }

        let mut expander = MacroExpander {
            macros: macros
                .into_iter()
                .map(|m| (m.name.clone(), m))
                .collect(),
            fresh_counter: 0,
            errors: Vec::new(),
        };

        for function in &mut program.functions {
            expander.expand_expr(&mut function.body, 0);
        }
        for type_decl in &mut program.types {
            for attribute in &mut type_decl.attributes {
                expander.expand_expr(&mut attribute.value, 0);
            }
            for init in &mut type_decl.parent_init_exprs {
                expander.expand_expr(init, 0);
            }
            for method in &mut type_decl.methods {
                expander.expand_expr(&mut method.body, 0);
            }
        }
        for statement in &mut program.statements {
            expander.expand_statement(statement, 0);
        }

        expander.errors
    }

    fn error(&mut self, message: impl Into<String>) {
        self.errors.push(CompilerError::new(
            ErrorCategory::Semantic,
            message,
            1,
            1,
        ));
    }

    fn fresh_name(&mut self, base: &str) -> String {
        let name = format!("{base}__macro{}", self.fresh_counter);
        self.fresh_counter += 1;
        name
    }

    fn expand_statement(&mut self, statement: &mut Statement, depth: usize) {
        match statement {
            Statement::Let { value, .. }
            | Statement::Assign { value, .. }
            | Statement::Print { value, .. }
            | Statement::Expr { value, .. } => self.expand_expr(value, depth),
        }
    }

    /// Expande recursivamente toda llamada a macro contenida en `expr`.
    fn expand_expr(&mut self, expr: &mut Expr, depth: usize) {
        if depth > MAX_EXPANSION_DEPTH {
            self.error(format!(
                "Macro expansion exceeded the maximum depth of {MAX_EXPANSION_DEPTH} (possible recursive macro)."
            ));
            return;
        }

        if let Expr::FunctionCall(call) = expr {
            if let Some(macro_decl) = self.macros.get(&call.name).cloned() {
                if macro_decl.params.len() != call.args.len() {
                    self.error(format!(
                        "Macro '{}' expects {} argument(s), but got {}.",
                        call.name,
                        macro_decl.params.len(),
                        call.args.len()
                    ));
                    return;
                }

                let mut env: HashMap<String, Binding> = macro_decl
                    .params
                    .iter()
                    .zip(call.args.iter())
                    .map(|(param, arg)| (param.name.clone(), Binding::Param(arg.clone())))
                    .collect();

                let mut body = macro_decl.body.clone();
                self.substitute_expr(&mut body, &mut env);
                *expr = body;

                // El cuerpo sustituido puede contener nuevas llamadas a macro
                // (macros que llaman macros, o macros en los argumentos).
                self.expand_expr(expr, depth + 1);
                return;
            }
        }

        self.visit_children(expr, depth);
    }

    fn visit_children(&mut self, expr: &mut Expr, depth: usize) {
        match expr {
            Expr::Binary(binary) => {
                self.expand_expr(&mut binary.left, depth);
                self.expand_expr(&mut binary.right, depth);
            }
            Expr::Unary(unary) => self.expand_expr(&mut unary.expr, depth),
            Expr::BuiltinCall(call) => {
                for arg in &mut call.args {
                    self.expand_expr(arg, depth);
                }
            }
            Expr::FunctionCall(call) => {
                for arg in &mut call.args {
                    self.expand_expr(arg, depth);
                }
            }
            Expr::BaseCall(call) => {
                for arg in &mut call.args {
                    self.expand_expr(arg, depth);
                }
            }
            Expr::MethodCall(call) => {
                self.expand_expr(&mut call.receiver, depth);
                for arg in &mut call.args {
                    self.expand_expr(arg, depth);
                }
            }
            Expr::MemberAccess(access) => self.expand_expr(&mut access.object, depth),
            Expr::New(new_expr) => {
                for arg in &mut new_expr.args {
                    self.expand_expr(arg, depth);
                }
            }
            Expr::DestructiveAssign(assign) => {
                match &mut assign.target {
                    AssignTarget::Member { object, .. } => self.expand_expr(object, depth),
                    AssignTarget::Index { object, index, .. } => {
                        self.expand_expr(object, depth);
                        self.expand_expr(index, depth);
                    }
                    AssignTarget::Variable { .. } => {}
                }
                self.expand_expr(&mut assign.value, depth);
            }
            Expr::LetIn(let_in) => {
                for binding in &mut let_in.bindings {
                    self.expand_expr(&mut binding.value, depth);
                }
                self.expand_expr(&mut let_in.body, depth);
            }
            Expr::Block(block) => {
                for statement in &mut block.statements {
                    self.expand_statement(statement, depth);
                }
            }
            Expr::While(while_expr) => {
                self.expand_expr(&mut while_expr.condition, depth);
                for statement in &mut while_expr.body.statements {
                    self.expand_statement(statement, depth);
                }
            }
            Expr::For(for_expr) => {
                self.expand_expr(&mut for_expr.iter, depth);
                for statement in &mut for_expr.body.statements {
                    self.expand_statement(statement, depth);
                }
            }
            Expr::If(if_expr) => {
                self.expand_expr(&mut if_expr.condition, depth);
                self.expand_expr(&mut if_expr.then_branch, depth);
                for elif in &mut if_expr.elif_branches {
                    self.expand_expr(&mut elif.condition, depth);
                    self.expand_expr(&mut elif.body, depth);
                }
                self.expand_expr(&mut if_expr.else_branch, depth);
            }
            Expr::Is(is_expr) => self.expand_expr(&mut is_expr.expr, depth),
            Expr::As(as_expr) => self.expand_expr(&mut as_expr.expr, depth),
            Expr::ArrayLiteral(literal) => {
                for element in &mut literal.elements {
                    self.expand_expr(element, depth);
                }
            }
            Expr::NewArray(new_array) => {
                self.expand_expr(&mut new_array.size, depth);
                if let Some(init) = &mut new_array.init {
                    self.expand_expr(&mut init.body, depth);
                }
            }
            Expr::Index(index_expr) => {
                self.expand_expr(&mut index_expr.object, depth);
                self.expand_expr(&mut index_expr.index, depth);
            }
            Expr::Lambda(lambda) => self.expand_expr(&mut lambda.body, depth),
            Expr::Literal { .. } | Expr::Variable { .. } => {}
        }
    }

    /// Aplica el entorno de sustitución al cuerpo clonado de una macro:
    /// parámetros → expresión del argumento, locales → nombre fresco.
    fn substitute_expr(&mut self, expr: &mut Expr, env: &mut HashMap<String, Binding>) {
        match expr {
            Expr::Variable { name, .. } => {
                match env.get(name) {
                    Some(Binding::Param(replacement)) => *expr = replacement.clone(),
                    Some(Binding::Renamed(fresh)) => *name = fresh.clone(),
                    None => {}
                }
            }
            Expr::Binary(binary) => {
                self.substitute_expr(&mut binary.left, env);
                self.substitute_expr(&mut binary.right, env);
            }
            Expr::Unary(unary) => self.substitute_expr(&mut unary.expr, env),
            Expr::BuiltinCall(call) => {
                for arg in &mut call.args {
                    self.substitute_expr(arg, env);
                }
            }
            Expr::FunctionCall(call) => {
                for arg in &mut call.args {
                    self.substitute_expr(arg, env);
                }
            }
            Expr::BaseCall(call) => {
                for arg in &mut call.args {
                    self.substitute_expr(arg, env);
                }
            }
            Expr::MethodCall(call) => {
                self.substitute_expr(&mut call.receiver, env);
                for arg in &mut call.args {
                    self.substitute_expr(arg, env);
                }
            }
            Expr::MemberAccess(access) => self.substitute_expr(&mut access.object, env),
            Expr::New(new_expr) => {
                for arg in &mut new_expr.args {
                    self.substitute_expr(arg, env);
                }
            }
            Expr::DestructiveAssign(assign) => {
                match &mut assign.target {
                    AssignTarget::Variable { name, .. } => match env.get(name) {
                        Some(Binding::Renamed(fresh)) => *name = fresh.clone(),
                        // Asignar a un parámetro solo es válido si el argumento
                        // fue a su vez una variable en el sitio de llamada.
                        Some(Binding::Param(Expr::Variable { name: arg_name, .. })) => {
                            *name = arg_name.clone();
                        }
                        _ => {}
                    },
                    AssignTarget::Member { object, .. } => {
                        self.substitute_expr(object, env);
                    }
                    AssignTarget::Index { object, index, .. } => {
                        self.substitute_expr(object, env);
                        self.substitute_expr(index, env);
                    }
                }
                self.substitute_expr(&mut assign.value, env);
            }
            Expr::LetIn(let_in) => {
                // Las ligaduras son secuenciales: cada valor ve las anteriores.
                let mut shadowed = Vec::new();
                for binding in &mut let_in.bindings {
                    self.substitute_expr(&mut binding.value, env);
                    let fresh = self.fresh_name(&binding.name);
                    shadowed.push((
                        binding.name.clone(),
                        env.insert(binding.name.clone(), Binding::Renamed(fresh.clone())),
                    ));
                    binding.name = fresh;
                }
                self.substitute_expr(&mut let_in.body, env);
                Self::restore(env, shadowed);
            }
            Expr::Block(block) => {
                // Un `let` de bloque liga la variable para el resto del bloque.
                let mut shadowed = Vec::new();
                for statement in &mut block.statements {
                    match statement {
                        Statement::Let { name, value, .. } => {
                            self.substitute_expr(value, env);
                            let fresh = self.fresh_name(name);
                            shadowed.push((
                                name.clone(),
                                env.insert(name.clone(), Binding::Renamed(fresh.clone())),
                            ));
                            *name = fresh;
                        }
                        Statement::Assign { name, value, .. } => {
                            if let Some(Binding::Renamed(fresh)) = env.get(name) {
                                *name = fresh.clone();
                            }
                            self.substitute_expr(value, env);
                        }
                        Statement::Print { value, .. } | Statement::Expr { value, .. } => {
                            self.substitute_expr(value, env);
                        }
                    }
                }
                Self::restore(env, shadowed);
            }
            Expr::While(while_expr) => {
                self.substitute_expr(&mut while_expr.condition, env);
                let mut body = Expr::Block(while_expr.body.clone());
                self.substitute_expr(&mut body, env);
                let Expr::Block(body) = body else {
                    unreachable!("substitute_expr preserves the Block variant");
                };
                while_expr.body = body;
            }
            Expr::For(for_expr) => {
                self.substitute_expr(&mut for_expr.iter, env);
                let fresh = self.fresh_name(&for_expr.id);
                let shadowed = vec![(
                    for_expr.id.clone(),
                    env.insert(for_expr.id.clone(), Binding::Renamed(fresh.clone())),
                )];
                for_expr.id = fresh;
                let mut body = Expr::Block(for_expr.body.clone());
                self.substitute_expr(&mut body, env);
                let Expr::Block(body) = body else {
                    unreachable!("substitute_expr preserves the Block variant");
                };
                for_expr.body = body;
                Self::restore(env, shadowed);
            }
            Expr::If(if_expr) => {
                self.substitute_expr(&mut if_expr.condition, env);
                self.substitute_expr(&mut if_expr.then_branch, env);
                for elif in &mut if_expr.elif_branches {
                    self.substitute_expr(&mut elif.condition, env);
                    self.substitute_expr(&mut elif.body, env);
                }
                self.substitute_expr(&mut if_expr.else_branch, env);
            }
            Expr::Is(is_expr) => self.substitute_expr(&mut is_expr.expr, env),
            Expr::As(as_expr) => self.substitute_expr(&mut as_expr.expr, env),
            Expr::ArrayLiteral(literal) => {
                for element in &mut literal.elements {
                    self.substitute_expr(element, env);
                }
            }
            Expr::NewArray(new_array) => {
                self.substitute_expr(&mut new_array.size, env);
                if let Some(init) = &mut new_array.init {
                    let fresh = self.fresh_name(&init.var_name);
                    let shadowed = vec![(
                        init.var_name.clone(),
                        env.insert(init.var_name.clone(), Binding::Renamed(fresh.clone())),
                    )];
                    init.var_name = fresh;
                    self.substitute_expr(&mut init.body, env);
                    Self::restore(env, shadowed);
                }
            }
            Expr::Index(index_expr) => {
                self.substitute_expr(&mut index_expr.object, env);
                self.substitute_expr(&mut index_expr.index, env);
            }
            Expr::Lambda(lambda) => {
                // Los parámetros de la lambda ligan nombres para su cuerpo.
                let mut shadowed = Vec::new();
                for param in &mut lambda.params {
                    let fresh = self.fresh_name(&param.name);
                    shadowed.push((
                        param.name.clone(),
                        env.insert(param.name.clone(), Binding::Renamed(fresh.clone())),
                    ));
                    param.name = fresh;
                }
                self.substitute_expr(&mut lambda.body, env);
                Self::restore(env, shadowed);
            }
            Expr::Literal { .. } => {}
        }
    }

    fn restore(env: &mut HashMap<String, Binding>, shadowed: Vec<(String, Option<Binding>)>) {
        for (name, previous) in shadowed.into_iter().rev() {
            match previous {
                Some(binding) => {
                    env.insert(name, binding);
                }
                None => {
                    env.remove(&name);
                }
            }
        }
    }
}
