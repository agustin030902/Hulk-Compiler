use std::collections::HashMap;

use crate::{
    codegen::CodegenBackend,
    error::{CompilerError, ErrorCategory},
    parser::expression::{BinaryOp, Expr, Literal, Program, Statement, UnaryOp},
};

#[derive(Debug, Default)]
pub struct LlvmBackend {
    body_lines: Vec<String>,
    global_lines: Vec<String>,
    errors: Vec<CompilerError>,
    variables: HashMap<String, VariableInfo>,
    temp_counter: usize,
    string_counter: usize,
}

#[derive(Debug, Clone)]
struct VariableInfo {
    ptr_name: String,
    value_type: ValueType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    Double,
    Bool,
    StringPtr,
}

#[derive(Debug, Clone)]
struct ValueRef {
    value_type: ValueType,
    repr: String,
}

impl LlvmBackend {
    pub fn new() -> Self {
        Self::default()
    }

    fn reset(&mut self) {
        self.body_lines.clear();
        self.global_lines.clear();
        self.errors.clear();
        self.variables.clear();
        self.temp_counter = 0;
        self.string_counter = 0;
    }

    fn emit_body(&mut self, line: impl Into<String>) {
        self.body_lines.push(line.into());
    }

    fn emit_global(&mut self, line: impl Into<String>) {
        self.global_lines.push(line.into());
    }

    fn next_temp(&mut self) -> String {
        let current = self.temp_counter;
        self.temp_counter += 1;
        format!("%t{}", current)
    }

    fn next_string_name(&mut self) -> String {
        let current = self.string_counter;
        self.string_counter += 1;
        format!("@.str.{}", current)
    }

    fn format_ptr_global(name: &str, bytes: usize) -> String {
        format!(
            "getelementptr inbounds ([{} x i8], [{} x i8]* {}, i64 0, i64 0)",
            bytes, bytes, name
        )
    }

    fn llvm_type(value_type: ValueType) -> &'static str {
        match value_type {
            ValueType::Double => "double",
            ValueType::Bool => "i1",
            ValueType::StringPtr => "i8*",
        }
    }

    fn semantic_error(&mut self, message: impl Into<String>) {
        self.errors
            .push(CompilerError::new(ErrorCategory::Semantic, message, 1, 1));
    }

    fn emit_program(&mut self, program: &Program) {
        for statement in &program.statements {
            self.emit_statement(statement);
        }
    }

    fn emit_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Let { name, value, .. } => {
                if self.variables.contains_key(name) {
                    self.semantic_error(format!("Variable '{}' already declared", name));
                    return;
                }

                let Some(value_ref) = self.emit_expr(value) else {
                    return;
                };

                let ptr_name = self.next_temp();
                let llvm_ty = Self::llvm_type(value_ref.value_type);
                self.emit_body(format!("{ptr_name} = alloca {llvm_ty}"));
                self.emit_body(format!(
                    "store {llvm_ty} {}, {llvm_ty}* {ptr_name}",
                    value_ref.repr
                ));

                self.variables.insert(
                    name.clone(),
                    VariableInfo {
                        ptr_name,
                        value_type: value_ref.value_type,
                    },
                );
            }
            Statement::Print { value, .. } => {
                let Some(value_ref) = self.emit_expr(value) else {
                    return;
                };

                match value_ref.value_type {
                    ValueType::Double => {
                        let fmt = Self::format_ptr_global("@.fmt.number", 4);
                        let call_tmp = self.next_temp();
                        self.emit_body(format!(
                            "{call_tmp} = call i32 (i8*, ...) @printf(i8* {fmt}, double {})",
                            value_ref.repr
                        ));
                    }
                    ValueType::StringPtr => {
                        let fmt = Self::format_ptr_global("@.fmt.string", 4);
                        let call_tmp = self.next_temp();
                        self.emit_body(format!(
                            "{call_tmp} = call i32 (i8*, ...) @printf(i8* {fmt}, i8* {})",
                            value_ref.repr
                        ));
                    }
                    ValueType::Bool => {
                        let bool_tmp = self.next_temp();
                        self.emit_body(format!("{bool_tmp} = zext i1 {} to i32", value_ref.repr));
                        let fmt = Self::format_ptr_global("@.fmt.bool", 4);
                        let call_tmp = self.next_temp();
                        self.emit_body(format!(
                            "{call_tmp} = call i32 (i8*, ...) @printf(i8* {fmt}, i32 {bool_tmp})"
                        ));
                    }
                }
            }
        }
    }

    fn emit_expr(&mut self, expr: &Expr) -> Option<ValueRef> {
        match expr {
            Expr::Literal { value, .. } => self.emit_literal(value),
            Expr::Variable { name, .. } => self.emit_variable(name),
            Expr::Unary(unary) => self.emit_unary(unary.op.clone(), &unary.expr),
            Expr::Binary(binary) => {
                self.emit_binary(binary.op.clone(), &binary.left, &binary.right)
            }
        }
    }

    fn emit_literal(&mut self, literal: &Literal) -> Option<ValueRef> {
        match literal {
            Literal::Integer(value) => Some(ValueRef {
                value_type: ValueType::Double,
                repr: format_double(*value as f64),
            }),
            Literal::Float(value) => Some(ValueRef {
                value_type: ValueType::Double,
                repr: format_double(*value),
            }),
            Literal::Boolean(value) => Some(ValueRef {
                value_type: ValueType::Bool,
                repr: if *value {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
            }),
            Literal::String(value) => {
                let global_name = self.next_string_name();
                let escaped = escape_llvm_string(value);
                let bytes_len = value.as_bytes().len() + 1;
                self.emit_global(format!(
                    "{global_name} = private unnamed_addr constant [{bytes_len} x i8] c\"{escaped}\""
                ));

                let temp = self.next_temp();
                self.emit_body(format!(
                    "{temp} = getelementptr inbounds [{bytes_len} x i8], [{bytes_len} x i8]* {global_name}, i64 0, i64 0"
                ));

                Some(ValueRef {
                    value_type: ValueType::StringPtr,
                    repr: temp,
                })
            }
        }
    }

    fn emit_variable(&mut self, name: &str) -> Option<ValueRef> {
        let Some(info) = self.variables.get(name).cloned() else {
            self.semantic_error(format!("Variable '{}' is not declared", name));
            return None;
        };

        let loaded = self.next_temp();
        let llvm_ty = Self::llvm_type(info.value_type);
        self.emit_body(format!(
            "{loaded} = load {llvm_ty}, {llvm_ty}* {}",
            info.ptr_name
        ));

        Some(ValueRef {
            value_type: info.value_type,
            repr: loaded,
        })
    }

    fn emit_unary(&mut self, op: UnaryOp, expr: &Expr) -> Option<ValueRef> {
        let value = self.emit_expr(expr)?;

        match op {
            UnaryOp::Neg => {
                if value.value_type != ValueType::Double {
                    self.semantic_error("Unary '-' only supports numeric values");
                    return None;
                }

                let result = self.next_temp();
                self.emit_body(format!("{result} = fneg double {}", value.repr));
                Some(ValueRef {
                    value_type: ValueType::Double,
                    repr: result,
                })
            }
        }
    }

    fn emit_binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr) -> Option<ValueRef> {
        let left = self.emit_expr(left)?;
        let right = self.emit_expr(right)?;

        match op {
            BinaryOp::Concat => self.emit_concat(&left, &right),
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                if left.value_type != ValueType::Double || right.value_type != ValueType::Double {
                    self.semantic_error("Binary arithmetic operators only support numeric values");
                    return None;
                }

                let instruction = match op {
                    BinaryOp::Add => "fadd",
                    BinaryOp::Sub => "fsub",
                    BinaryOp::Mul => "fmul",
                    BinaryOp::Div => "fdiv",
                    BinaryOp::Concat => unreachable!("concat handled in outer match"),
                };

                let result = self.next_temp();
                self.emit_body(format!(
                    "{result} = {instruction} double {}, {}",
                    left.repr, right.repr
                ));

                Some(ValueRef {
                    value_type: ValueType::Double,
                    repr: result,
                })
            }
        }
    }

    fn emit_concat(&mut self, left: &ValueRef, right: &ValueRef) -> Option<ValueRef> {
        let (fmt_name, arg_values) = match (left.value_type, right.value_type) {
            (ValueType::StringPtr, ValueType::StringPtr) => {
                ("@.fmt.concat.ss", format!("i8* {}, i8* {}", left.repr, right.repr))
            }
            (ValueType::StringPtr, ValueType::Double) => {
                ("@.fmt.concat.sn", format!("i8* {}, double {}", left.repr, right.repr))
            }
            (ValueType::Double, ValueType::StringPtr) => {
                ("@.fmt.concat.ns", format!("double {}, i8* {}", left.repr, right.repr))
            }
            _ => {
                self.semantic_error(format!(
                    "Operator '@' expects (String, String), (String, Number), or (Number, String), but got {} and {} in code generation.",
                    value_type_name(left.value_type),
                    value_type_name(right.value_type)
                ));
                return None;
            }
        };

        let result_slot = self.next_temp();
        self.emit_body(format!("{result_slot} = alloca i8*"));

        let call_tmp = self.next_temp();
        let fmt_ptr = Self::format_ptr_global(fmt_name, 5);
        self.emit_body(format!(
            "{call_tmp} = call i32 (i8**, i8*, ...) @asprintf(i8** {result_slot}, i8* {fmt_ptr}, {arg_values})"
        ));

        let loaded = self.next_temp();
        self.emit_body(format!("{loaded} = load i8*, i8** {result_slot}"));

        Some(ValueRef {
            value_type: ValueType::StringPtr,
            repr: loaded,
        })
    }

    fn compose_module(&self) -> String {
        let mut out = vec![
            "; Hulk LLVM IR (intermediate code)".to_string(),
            "declare i32 @printf(i8*, ...)".to_string(),
            "declare i32 @asprintf(i8**, i8*, ...)".to_string(),
            "@.fmt.number = private unnamed_addr constant [4 x i8] c\"%g\\0A\\00\"".to_string(),
            "@.fmt.string = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\"".to_string(),
            "@.fmt.bool = private unnamed_addr constant [4 x i8] c\"%d\\0A\\00\"".to_string(),
            "@.fmt.concat.ss = private unnamed_addr constant [5 x i8] c\"%s%s\\00\"".to_string(),
            "@.fmt.concat.sn = private unnamed_addr constant [5 x i8] c\"%s%g\\00\"".to_string(),
            "@.fmt.concat.ns = private unnamed_addr constant [5 x i8] c\"%g%s\\00\"".to_string(),
        ];

        out.extend(self.global_lines.clone());
        out.push(String::new());
        out.push("define i32 @main() {".to_string());
        out.push("entry:".to_string());

        for line in &self.body_lines {
            out.push(format!("  {line}"));
        }

        out.push("  ret i32 0".to_string());
        out.push("}".to_string());

        out.join("\n")
    }
}

impl CodegenBackend for LlvmBackend {
    fn generate(&mut self, program: &Program) -> Result<String, Vec<CompilerError>> {
        self.reset();
        self.emit_program(program);

        if self.errors.is_empty() {
            Ok(self.compose_module())
        } else {
            Err(self.errors.clone())
        }
    }
}

fn escape_llvm_string(value: &str) -> String {
    let mut escaped = String::new();

    for byte in value.as_bytes() {
        match *byte {
            b'\\' => escaped.push_str("\\5C"),
            b'"' => escaped.push_str("\\22"),
            32..=126 => escaped.push(*byte as char),
            _ => escaped.push_str(&format!("\\{:02X}", byte)),
        }
    }

    escaped.push_str("\\00");
    escaped
}

fn format_double(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        let text = format!("{value:.10}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn value_type_name(value_type: ValueType) -> &'static str {
    match value_type {
        ValueType::Double => "Number",
        ValueType::Bool => "Boolean",
        ValueType::StringPtr => "String",
    }
}
