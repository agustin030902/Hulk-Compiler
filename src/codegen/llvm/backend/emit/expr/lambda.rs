//! Emisión de lambdas como closures reales.
//!
//! Cada lambda se eleva a una función LLVM `@hulk_lambda_N(i8* %__env, …)` y
//! su valor en runtime es un puntero a heap con layout
//! `[fnptr i8*][captura0][captura1]…` (8 bytes por captura). Las variables
//! libres del cuerpo se capturan **por valor** en el momento de creación.
//! Llamar a un valor función carga el fnptr de la cabecera y lo invoca
//! pasando el propio closure como entorno.

use std::collections::HashSet;

use crate::parser::expression::{AssignTarget, Expr, LambdaExpr, Statement};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType, VariableInfo};

impl LlvmBackend {
    /// Resuelve el nombre de una anotación (simple, arreglo o tipo función
    /// canónico `(A,B)->C`) al ValueType correspondiente.
    fn resolve_annotation_value_type(&mut self, name: &str) -> Option<ValueType> {
        if name.starts_with('(') {
            let (param_names, ret_name) = split_function_type_name(name)?;
            let mut params = Vec::with_capacity(param_names.len());
            for param_name in &param_names {
                params.push(self.resolve_annotation_value_type(param_name)?);
            }
            let ret = self.resolve_annotation_value_type(&ret_name)?;
            return self.function_type_for(&params, ret);
        }
        self.resolve_elem_type_name(name)
    }

    /// Búsqueda estructural inversa de la firma en `function_types`.
    fn function_type_for(&self, params: &[ValueType], ret: ValueType) -> Option<ValueType> {
        self.function_types
            .iter()
            .find(|(_, (entry_params, entry_ret))| {
                entry_params.len() == params.len()
                    && entry_params
                        .iter()
                        .zip(params.iter())
                        .all(|(a, b)| *a == *b || self.are_compatible_value_types(*a, *b))
                    && (*entry_ret == ret || self.are_compatible_value_types(*entry_ret, ret))
            })
            .map(|(id, _)| ValueType::Function(*id))
    }

    pub(in crate::codegen::llvm) fn emit_lambda(
        &mut self,
        lambda: &LambdaExpr,
    ) -> Option<ValueRef> {
        // Tipos de parámetros y retorno desde las anotaciones (obligatorias).
        let mut param_types = Vec::with_capacity(lambda.params.len());
        for param in &lambda.params {
            let Some(annotation) = &param.type_annotation else {
                self.semantic_error(format!(
                    "Lambda parameter '{}' requires a type annotation.",
                    param.name
                ));
                return None;
            };
            param_types.push(self.resolve_annotation_value_type(&annotation.name)?);
        }
        let Some(return_annotation) = &lambda.return_type_annotation else {
            self.semantic_error("Lambda requires a return type annotation.".to_string());
            return None;
        };
        let return_type = self.resolve_annotation_value_type(&return_annotation.name)?;

        let Some(ValueType::Function(function_type_id)) =
            self.function_type_for(&param_types, return_type)
        else {
            self.semantic_error(
                "Lambda signature was not registered during analysis.".to_string(),
            );
            return None;
        };

        // Variables libres del cuerpo que existen en el entorno actual:
        // se capturan por valor dentro del closure.
        let mut bound: HashSet<String> =
            lambda.params.iter().map(|p| p.name.clone()).collect();
        bound.insert("self".to_string());
        let mut captures: Vec<(String, VariableInfo)> = Vec::new();
        self.collect_free_vars(&lambda.body, &bound, &mut captures);

        let lambda_index = self.lambda_counter;
        self.lambda_counter += 1;
        let llvm_name = format!("hulk_lambda_{lambda_index}");

        // ── Cuerpo de la lambda como función independiente ──
        let saved_body = std::mem::take(&mut self.body_lines);
        let saved_scopes = std::mem::take(&mut self.scopes);
        let saved_block = std::mem::take(&mut self.current_block);
        self.push_scope();

        let mut param_decls = vec!["i8* %__env".to_string()];
        for (param, value_type) in lambda.params.iter().zip(param_types.iter().copied()) {
            param_decls.push(format!("{} %{}", value_type.llvm_type(), param.name));
        }

        for (param, value_type) in lambda.params.iter().zip(param_types.iter().copied()) {
            let ptr_name = self.next_temp();
            let llvm_type = value_type.llvm_type();
            self.emit_body(format!("{ptr_name} = alloca {llvm_type}"));
            self.emit_body(format!(
                "store {llvm_type} %{}, {llvm_type}* {ptr_name}",
                param.name
            ));
            self.bind_current_scope(
                param.name.clone(),
                VariableInfo {
                    ptr_name,
                    value_type,
                },
            );
        }

        // Cargar cada captura desde el entorno a un alloca local.
        for (index, (name, info)) in captures.iter().enumerate() {
            let offset = 8 + index * 8;
            let raw_ptr = self.next_temp();
            self.emit_body(format!(
                "{raw_ptr} = getelementptr i8, i8* %__env, i64 {offset}"
            ));
            let llvm_type = info.value_type.llvm_type();
            let typed_ptr = self.next_temp();
            self.emit_body(format!("{typed_ptr} = bitcast i8* {raw_ptr} to {llvm_type}*"));
            let value = self.next_temp();
            self.emit_body(format!(
                "{value} = load {llvm_type}, {llvm_type}* {typed_ptr}"
            ));
            let local_ptr = self.next_temp();
            self.emit_body(format!("{local_ptr} = alloca {llvm_type}"));
            self.emit_body(format!(
                "store {llvm_type} {value}, {llvm_type}* {local_ptr}"
            ));
            self.bind_current_scope(
                name.clone(),
                VariableInfo {
                    ptr_name: local_ptr,
                    value_type: info.value_type,
                },
            );
        }

        let result = self.emit_expr(&lambda.body);
        let function_body = std::mem::take(&mut self.body_lines);
        self.scopes = saved_scopes;
        self.body_lines = saved_body;
        self.current_block = saved_block;
        let result = result?;

        if !self.are_compatible_value_types(return_type, result.value_type) {
            self.semantic_error(format!(
                "Lambda body produces {} but its annotation expects {}.",
                self.type_name_for_value_type(result.value_type),
                self.type_name_for_value_type(return_type)
            ));
            return None;
        }

        let return_llvm = return_type.llvm_type();
        self.emit_function_line(String::new());
        self.emit_function_line(format!(
            "define {return_llvm} @{llvm_name}({}) {{",
            param_decls.join(", ")
        ));
        self.emit_function_line("entry:".to_string());
        for line in function_body {
            if line.ends_with(':') {
                self.emit_function_line(line);
            } else {
                self.emit_function_line(format!("  {line}"));
            }
        }
        self.emit_function_line(format!("  ret {return_llvm} {}", result.repr));
        self.emit_function_line("}".to_string());

        // ── Construcción del closure en el sitio de creación ──
        let closure_size = 8 + captures.len() * 8;
        let closure = self.next_temp();
        self.emit_body(format!("{closure} = call i8* @malloc(i64 {closure_size})"));

        let param_sig = std::iter::once("i8*".to_string())
            .chain(param_types.iter().map(|t| t.llvm_type().to_string()))
            .collect::<Vec<_>>()
            .join(", ");
        let fnptr_slot = self.next_temp();
        self.emit_body(format!("{fnptr_slot} = bitcast i8* {closure} to i8**"));
        let fnptr = self.next_temp();
        self.emit_body(format!(
            "{fnptr} = bitcast {return_llvm} ({param_sig})* @{llvm_name} to i8*"
        ));
        self.emit_body(format!("store i8* {fnptr}, i8** {fnptr_slot}"));

        for (index, (_, info)) in captures.iter().enumerate() {
            let llvm_type = info.value_type.llvm_type();
            let value = self.next_temp();
            self.emit_body(format!(
                "{value} = load {llvm_type}, {llvm_type}* {}",
                info.ptr_name
            ));
            let offset = 8 + index * 8;
            let raw_ptr = self.next_temp();
            self.emit_body(format!(
                "{raw_ptr} = getelementptr i8, i8* {closure}, i64 {offset}"
            ));
            let typed_ptr = self.next_temp();
            self.emit_body(format!("{typed_ptr} = bitcast i8* {raw_ptr} to {llvm_type}*"));
            self.emit_body(format!(
                "store {llvm_type} {value}, {llvm_type}* {typed_ptr}"
            ));
        }

        Some(ValueRef {
            value_type: ValueType::Function(function_type_id),
            repr: closure,
        })
    }

    /// Llamada indirecta a un valor función: carga el fnptr de la cabecera
    /// del closure y lo invoca con el closure como primer argumento.
    pub(in crate::codegen::llvm) fn emit_closure_call(
        &mut self,
        closure_var: &VariableInfo,
        function_type_id: u32,
        args: &[Expr],
        name: &str,
    ) -> Option<ValueRef> {
        let Some((param_types, return_type)) =
            self.function_types.get(&function_type_id).cloned()
        else {
            self.semantic_error(format!("'{name}' is not callable."));
            return None;
        };

        if param_types.len() != args.len() {
            self.semantic_error(format!(
                "Function value '{}' expects {} argument(s), but got {}.",
                name,
                param_types.len(),
                args.len()
            ));
            return None;
        }

        let closure = self.next_temp();
        self.emit_body(format!(
            "{closure} = load i8*, i8** {}",
            closure_var.ptr_name
        ));

        let mut arg_values = vec![format!("i8* {closure}")];
        for (arg, expected) in args.iter().zip(param_types.iter().copied()) {
            let value = self.emit_expr(arg)?;
            if !self.are_compatible_value_types(expected, value.value_type) {
                self.semantic_error(format!(
                    "Function value '{}' argument expects {}, but got {}.",
                    name,
                    self.type_name_for_value_type(expected),
                    self.type_name_for_value_type(value.value_type)
                ));
                return None;
            }
            let repr = self
                .value_repr_for_expected_type(expected, &value)
                .unwrap_or_else(|| value.repr.clone());
            arg_values.push(format!("{} {}", expected.llvm_type(), repr));
        }

        let param_sig = std::iter::once("i8*".to_string())
            .chain(param_types.iter().map(|t| t.llvm_type().to_string()))
            .collect::<Vec<_>>()
            .join(", ");
        let return_llvm = return_type.llvm_type();

        let fnptr_slot = self.next_temp();
        self.emit_body(format!("{fnptr_slot} = bitcast i8* {closure} to i8**"));
        let fnptr_raw = self.next_temp();
        self.emit_body(format!("{fnptr_raw} = load i8*, i8** {fnptr_slot}"));
        let fnptr = self.next_temp();
        self.emit_body(format!(
            "{fnptr} = bitcast i8* {fnptr_raw} to {return_llvm} ({param_sig})*"
        ));

        let result = if return_type == ValueType::Unit {
            self.emit_body(format!(
                "call {return_llvm} {fnptr}({})",
                arg_values.join(", ")
            ));
            "0".to_string()
        } else {
            let temp = self.next_temp();
            self.emit_body(format!(
                "{temp} = call {return_llvm} {fnptr}({})",
                arg_values.join(", ")
            ));
            temp
        };

        Some(ValueRef {
            value_type: return_type,
            repr: result,
        })
    }

    /// Recolecta las variables libres de `expr` (no ligadas dentro de la
    /// lambda) que existen en los scopes actuales del backend.
    fn collect_free_vars(
        &self,
        expr: &Expr,
        bound: &HashSet<String>,
        captures: &mut Vec<(String, VariableInfo)>,
    ) {
        match expr {
            Expr::Variable { name, .. } => {
                if !bound.contains(name) && !captures.iter().any(|(n, _)| n == name) {
                    if let Some(info) = self.lookup_var(name) {
                        captures.push((name.clone(), info));
                    }
                }
            }
            Expr::Binary(binary) => {
                self.collect_free_vars(&binary.left, bound, captures);
                self.collect_free_vars(&binary.right, bound, captures);
            }
            Expr::Unary(unary) => self.collect_free_vars(&unary.expr, bound, captures),
            Expr::BuiltinCall(call) => {
                for arg in &call.args {
                    self.collect_free_vars(arg, bound, captures);
                }
            }
            Expr::FunctionCall(call) => {
                // El nombre puede referirse a un valor función capturable.
                if !bound.contains(&call.name)
                    && !captures.iter().any(|(n, _)| n == &call.name)
                {
                    if let Some(info) = self.lookup_var(&call.name) {
                        if matches!(info.value_type, ValueType::Function(_)) {
                            captures.push((call.name.clone(), info));
                        }
                    }
                }
                for arg in &call.args {
                    self.collect_free_vars(arg, bound, captures);
                }
            }
            Expr::BaseCall(call) => {
                for arg in &call.args {
                    self.collect_free_vars(arg, bound, captures);
                }
            }
            Expr::MethodCall(call) => {
                self.collect_free_vars(&call.receiver, bound, captures);
                for arg in &call.args {
                    self.collect_free_vars(arg, bound, captures);
                }
            }
            Expr::MemberAccess(access) => {
                self.collect_free_vars(&access.object, bound, captures)
            }
            Expr::New(new_expr) => {
                for arg in &new_expr.args {
                    self.collect_free_vars(arg, bound, captures);
                }
            }
            Expr::DestructiveAssign(assign) => {
                match &assign.target {
                    AssignTarget::Variable { name, .. } => {
                        if !bound.contains(name) && !captures.iter().any(|(n, _)| n == name) {
                            if let Some(info) = self.lookup_var(name) {
                                captures.push((name.clone(), info));
                            }
                        }
                    }
                    AssignTarget::Member { object, .. } => {
                        self.collect_free_vars(object, bound, captures)
                    }
                    AssignTarget::Index { object, index, .. } => {
                        self.collect_free_vars(object, bound, captures);
                        self.collect_free_vars(index, bound, captures);
                    }
                }
                self.collect_free_vars(&assign.value, bound, captures);
            }
            Expr::LetIn(let_in) => {
                let mut inner = bound.clone();
                for binding in &let_in.bindings {
                    self.collect_free_vars(&binding.value, &inner, captures);
                    inner.insert(binding.name.clone());
                }
                self.collect_free_vars(&let_in.body, &inner, captures);
            }
            Expr::Block(block) => {
                let mut inner = bound.clone();
                for statement in &block.statements {
                    match statement {
                        Statement::Let { name, value, .. } => {
                            self.collect_free_vars(value, &inner, captures);
                            inner.insert(name.clone());
                        }
                        Statement::Assign { name, value, .. } => {
                            if !inner.contains(name)
                                && !captures.iter().any(|(n, _)| n == name)
                            {
                                if let Some(info) = self.lookup_var(name) {
                                    captures.push((name.clone(), info));
                                }
                            }
                            self.collect_free_vars(value, &inner, captures);
                        }
                        Statement::Print { value, .. } | Statement::Expr { value, .. } => {
                            self.collect_free_vars(value, &inner, captures);
                        }
                    }
                }
            }
            Expr::While(while_expr) => {
                self.collect_free_vars(&while_expr.condition, bound, captures);
                self.collect_free_vars(
                    &Expr::Block(while_expr.body.clone()),
                    bound,
                    captures,
                );
            }
            Expr::For(for_expr) => {
                self.collect_free_vars(&for_expr.iter, bound, captures);
                let mut inner = bound.clone();
                inner.insert(for_expr.id.clone());
                self.collect_free_vars(&Expr::Block(for_expr.body.clone()), &inner, captures);
            }
            Expr::If(if_expr) => {
                self.collect_free_vars(&if_expr.condition, bound, captures);
                self.collect_free_vars(&if_expr.then_branch, bound, captures);
                for elif in &if_expr.elif_branches {
                    self.collect_free_vars(&elif.condition, bound, captures);
                    self.collect_free_vars(&elif.body, bound, captures);
                }
                self.collect_free_vars(&if_expr.else_branch, bound, captures);
            }
            Expr::Is(is_expr) => self.collect_free_vars(&is_expr.expr, bound, captures),
            Expr::As(as_expr) => self.collect_free_vars(&as_expr.expr, bound, captures),
            Expr::ArrayLiteral(literal) => {
                for element in &literal.elements {
                    self.collect_free_vars(element, bound, captures);
                }
            }
            Expr::NewArray(new_array) => {
                self.collect_free_vars(&new_array.size, bound, captures);
                if let Some(init) = &new_array.init {
                    let mut inner = bound.clone();
                    inner.insert(init.var_name.clone());
                    self.collect_free_vars(&init.body, &inner, captures);
                }
            }
            Expr::Index(index_expr) => {
                self.collect_free_vars(&index_expr.object, bound, captures);
                self.collect_free_vars(&index_expr.index, bound, captures);
            }
            Expr::Lambda(lambda) => {
                let mut inner = bound.clone();
                for param in &lambda.params {
                    inner.insert(param.name.clone());
                }
                self.collect_free_vars(&lambda.body, &inner, captures);
            }
            Expr::Literal { .. } => {}
        }
    }
}

/// Separa `(A,B)->C` en (["A","B"], "C") respetando paréntesis anidados.
fn split_function_type_name(name: &str) -> Option<(Vec<String>, String)> {
    let inner_start = name.find('(')? + 1;
    let mut depth = 1usize;
    let mut inner_end = None;
    for (offset, ch) in name[inner_start..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    inner_end = Some(inner_start + offset);
                    break;
                }
            }
            _ => {}
        }
    }
    let inner_end = inner_end?;
    let params_text = &name[inner_start..inner_end];
    let ret_name = name[inner_end + 1..].strip_prefix("->")?.to_string();

    let mut params = Vec::new();
    if !params_text.is_empty() {
        let mut depth = 0usize;
        let mut start = 0usize;
        for (offset, ch) in params_text.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                ',' if depth == 0 => {
                    params.push(params_text[start..offset].to_string());
                    start = offset + 1;
                }
                _ => {}
            }
        }
        params.push(params_text[start..].to_string());
    }

    Some((params, ret_name))
}
