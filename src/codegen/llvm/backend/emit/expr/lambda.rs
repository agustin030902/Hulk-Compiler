//! Emisión de lambdas como closures reales.
//!
//! Cada lambda se eleva a una función LLVM `@hulk_lambda_N(i8* %__env, …)` y
//! su valor en runtime es un puntero a heap con layout
//! `[fnptr i8*][captura0][captura1]…` (8 bytes por captura). Las variables
//! libres del cuerpo se capturan **por valor** en el momento de creación.
//! Llamar a un valor función carga el fnptr de la cabecera y lo invoca
//! pasando el propio closure como entorno.

use std::collections::HashSet;

use crate::parser::expression::{Expr, LambdaExpr};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::state::{ValueRef, ValueType, VariableInfo};

impl LlvmBackend {
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

        let closure = self.next_temp();
        self.emit_body(format!(
            "{closure} = load i8*, i8** {}",
            closure_var.ptr_name
        ));

        let context = format!("Function value '{name}'");
        let arg_values = self.emit_coerced_args(&context, args, &param_types, Some(&closure))?;

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

        let result = self.emit_call_instruction(return_type, &fnptr, &arg_values.join(", "));

        Some(ValueRef {
            value_type: return_type,
            repr: result,
        })
    }
}
