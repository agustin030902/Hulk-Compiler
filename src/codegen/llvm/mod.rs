use std::collections::HashMap;

use crate::{
    codegen::CodegenBackend,
    error::{CompilerError, ErrorCategory},
    parser::expression::{
        BinaryOp, BlockExpr, BuiltinFunction, DestructiveAssignExpr, Expr, LetInExpr, Literal,
        Program, Statement, UnaryOp,
    },
};

#[derive(Debug, Default)]
pub struct LlvmBackend {
    body_lines: Vec<String>,
    global_lines: Vec<String>,
    errors: Vec<CompilerError>,
    scopes: Vec<HashMap<String, VariableInfo>>,
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
        self.scopes.clear();
        self.temp_counter = 0;
        self.string_counter = 0;
        self.push_scope();
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

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn current_scope_mut(&mut self) -> &mut HashMap<String, VariableInfo> {
        self.scopes
            .last_mut()
            .expect("a scope should always be present")
    }

    fn is_declared_in_current_scope(&self, name: &str) -> bool {
        self.scopes
            .last()
            .map(|scope| scope.contains_key(name))
            .unwrap_or(false)
    }

    fn lookup_var(&self, name: &str) -> Option<VariableInfo> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn lookup_var_with_index(&self, name: &str) -> Option<(usize, VariableInfo)> {
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(idx, scope)| scope.get(name).cloned().map(|info| (idx, info)))
    }

    fn emit_program(&mut self, program: &Program) {
        for statement in &program.statements {
            let _ = self.emit_statement(statement);
        }
    }

    fn emit_statement(&mut self, statement: &Statement) -> Option<ValueRef> {
        match statement {
            Statement::Let { name, value, .. } => {
                if self.is_declared_in_current_scope(name) {
                    self.semantic_error(format!("Variable '{}' already declared", name));
                    return None;
                }

                let Some(value_ref) = self.emit_expr(value) else {
                    return None;
                };

                let ptr_name = self.next_temp();
                let llvm_ty = Self::llvm_type(value_ref.value_type);
                self.emit_body(format!("{ptr_name} = alloca {llvm_ty}"));
                self.emit_body(format!(
                    "store {llvm_ty} {}, {llvm_ty}* {ptr_name}",
                    value_ref.repr
                ));

                self.current_scope_mut().insert(
                    name.clone(),
                    VariableInfo {
                        ptr_name: ptr_name.clone(),
                        value_type: value_ref.value_type,
                    },
                );

                Some(value_ref)
            }
            Statement::Print { value, .. } => {
                let Some(value_ref) = self.emit_expr(value) else {
                    return None;
                };
                self.emit_print_value(&value_ref);
                Some(value_ref)
            }
            Statement::Expr { value, .. } => self.emit_expr(value),
            Statement::Assign { name, value, .. } => {
                let Some((scope_index, existing)) = self.lookup_var_with_index(name) else {
                    self.semantic_error(format!("Variable '{}' is not declared", name));
                    return None;
                };

                let Some(value_ref) = self.emit_expr(value) else {
                    return None;
                };

                if existing.value_type == value_ref.value_type {
                    let llvm_ty = Self::llvm_type(existing.value_type);
                    self.emit_body(format!(
                        "store {llvm_ty} {}, {llvm_ty}* {}",
                        value_ref.repr, existing.ptr_name
                    ));
                    Some(value_ref)
                } else {
                    let new_ptr_name = self.next_temp();
                    let llvm_ty = Self::llvm_type(value_ref.value_type);
                    self.emit_body(format!("{new_ptr_name} = alloca {llvm_ty}"));
                    self.emit_body(format!(
                        "store {llvm_ty} {}, {llvm_ty}* {new_ptr_name}",
                        value_ref.repr
                    ));
                    self.scopes[scope_index].insert(
                        name.clone(),
                        VariableInfo {
                            ptr_name: new_ptr_name,
                            value_type: value_ref.value_type,
                        },
                    );
                    Some(value_ref)
                }
            }
        }
    }

    fn emit_expr(&mut self, expr: &Expr) -> Option<ValueRef> {
        match expr {
            Expr::Literal { value, .. } => self.emit_literal(value),
            Expr::Variable { name, .. } => self.emit_variable(name),
            Expr::Unary(unary) => self.emit_unary(unary.op.clone(), &unary.expr),
            Expr::Block(block) => self.emit_block_expr(block),
            Expr::DestructiveAssign(assign) => self.emit_destructive_assign(assign),
            Expr::LetIn(let_in) => self.emit_let_in_expr(let_in),
            Expr::BuiltinCall(call) => self.emit_builtin_call(call.function, &call.args),
            Expr::Binary(binary) => {
                self.emit_binary(binary.op.clone(), &binary.left, &binary.right)
            }
        }
    }

    fn emit_builtin_call(&mut self, function: BuiltinFunction, args: &[Expr]) -> Option<ValueRef> {
        match function {
            BuiltinFunction::Print => {
                let Some(arg_expr) = args.first() else {
                    self.semantic_error("Function 'print' expects 1 argument");
                    return None;
                };
                let value = self.emit_expr(arg_expr)?;
                self.emit_print_value(&value);
                Some(value)
            }
            BuiltinFunction::Sin
            | BuiltinFunction::Cos
            | BuiltinFunction::Sqrt
            | BuiltinFunction::Exp => {
                let Some(arg_expr) = args.first() else {
                    self.semantic_error(format!(
                        "Function '{}' expects 1 argument",
                        function.name()
                    ));
                    return None;
                };

                let arg = self.emit_expr(arg_expr)?;
                if arg.value_type != ValueType::Double {
                    self.semantic_error(format!(
                        "Function '{}' only supports numeric values",
                        function.name()
                    ));
                    return None;
                }

                let intrinsic = match function {
                    BuiltinFunction::Sin => "llvm.sin.f64",
                    BuiltinFunction::Cos => "llvm.cos.f64",
                    BuiltinFunction::Sqrt => "llvm.sqrt.f64",
                    BuiltinFunction::Exp => "llvm.exp.f64",
                    BuiltinFunction::Log => unreachable!("log handled in dedicated branch"),
                    BuiltinFunction::Rand => unreachable!("rand handled in dedicated branch"),
                    BuiltinFunction::Print => unreachable!("print handled in dedicated branch"),
                };

                let result = self.next_temp();
                self.emit_body(format!(
                    "{result} = call double @{intrinsic}(double {})",
                    arg.repr
                ));

                Some(ValueRef {
                    value_type: ValueType::Double,
                    repr: result,
                })
            }
            BuiltinFunction::Log => {
                if args.len() != 2 {
                    self.semantic_error("Function 'log' expects 2 arguments");
                    return None;
                }

                let base = self.emit_expr(&args[0])?;
                let value = self.emit_expr(&args[1])?;
                if base.value_type != ValueType::Double || value.value_type != ValueType::Double {
                    self.semantic_error("Function 'log' only supports numeric values");
                    return None;
                }

                let ln_base = self.next_temp();
                self.emit_body(format!(
                    "{ln_base} = call double @llvm.log.f64(double {})",
                    base.repr
                ));

                let ln_value = self.next_temp();
                self.emit_body(format!(
                    "{ln_value} = call double @llvm.log.f64(double {})",
                    value.repr
                ));

                let result = self.next_temp();
                self.emit_body(format!("{result} = fdiv double {ln_value}, {ln_base}"));

                Some(ValueRef {
                    value_type: ValueType::Double,
                    repr: result,
                })
            }
            BuiltinFunction::Rand => {
                if !args.is_empty() {
                    self.semantic_error("Function 'rand' expects 0 arguments");
                    return None;
                }

                let raw = self.next_temp();
                self.emit_body(format!("{raw} = call i32 @rand()"));

                let as_double = self.next_temp();
                self.emit_body(format!("{as_double} = sitofp i32 {raw} to double"));

                let normalized = self.next_temp();
                self.emit_body(format!(
                    "{normalized} = fdiv double {as_double}, 2147483647.0"
                ));

                Some(ValueRef {
                    value_type: ValueType::Double,
                    repr: normalized,
                })
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
        let Some(info) = self.lookup_var(name) else {
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

    fn emit_print_value(&mut self, value_ref: &ValueRef) {
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

    fn emit_block_expr(&mut self, block: &BlockExpr) -> Option<ValueRef> {
        self.push_scope();

        let mut last_value: Option<ValueRef> = None;
        for statement in &block.statements {
            if let Some(value) = self.emit_statement(statement) {
                last_value = Some(value);
            }
        }

        self.pop_scope();

        if let Some(value) = last_value {
            Some(value)
        } else {
            self.semantic_error("Block expression must produce a value");
            None
        }
    }

    fn emit_destructive_assign(&mut self, assign: &DestructiveAssignExpr) -> Option<ValueRef> {
        let Some(existing) = self.lookup_var(&assign.name) else {
            self.semantic_error(format!(
                "Variable '{}' is assigned before declaration. Declare it with 'let' first.",
                assign.name
            ));
            return None;
        };

        let Some(value_ref) = self.emit_expr(&assign.value) else {
            return None;
        };

        if value_ref.value_type != existing.value_type {
            self.semantic_error(format!(
                "Destructive assignment ':=' requires the same type. Variable '{}' is {:?} but expression is {:?}.",
                assign.name, existing.value_type, value_ref.value_type
            ));
            return None;
        }

        let llvm_ty = Self::llvm_type(existing.value_type);
        self.emit_body(format!(
            "store {llvm_ty} {}, {llvm_ty}* {}",
            value_ref.repr, existing.ptr_name
        ));

        Some(value_ref)
    }

    fn emit_let_in_expr(&mut self, let_in: &LetInExpr) -> Option<ValueRef> {
        self.push_scope();

        for binding in &let_in.bindings {
            if self.is_declared_in_current_scope(&binding.name) {
                self.semantic_error(format!(
                    "Variable '{}' redeclared in let-in binding",
                    binding.name
                ));
                continue;
            }

            let Some(value_ref) = self.emit_expr(&binding.value) else {
                return None;
            };

            let ptr_name = self.next_temp();
            let llvm_ty = Self::llvm_type(value_ref.value_type);
            self.emit_body(format!("{ptr_name} = alloca {llvm_ty}"));
            self.emit_body(format!(
                "store {llvm_ty} {}, {llvm_ty}* {ptr_name}",
                value_ref.repr
            ));

            self.current_scope_mut().insert(
                binding.name.clone(),
                VariableInfo {
                    ptr_name,
                    value_type: value_ref.value_type,
                },
            );
        }

        let body_value = self.emit_expr(&let_in.body);

        self.pop_scope();

        body_value
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
            UnaryOp::Not => {
                if value.value_type != ValueType::Bool {
                    self.semantic_error("Unary '!' only supports boolean values");
                    return None;
                }

                let result = self.next_temp();
                self.emit_body(format!("{result} = xor i1 {}, true", value.repr));
                Some(ValueRef {
                    value_type: ValueType::Bool,
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
            BinaryOp::Pow => {
                if left.value_type != ValueType::Double || right.value_type != ValueType::Double {
                    self.semantic_error("Binary arithmetic operators only support numeric values");
                    return None;
                }

                let result = self.next_temp();
                self.emit_body(format!(
                    "{result} = call double @llvm.pow.f64(double {}, double {})",
                    left.repr, right.repr
                ));

                Some(ValueRef {
                    value_type: ValueType::Double,
                    repr: result,
                })
            }
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
                    _ => unreachable!("non-arithmetic operator in arithmetic branch"),
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
            BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEqual | BinaryOp::GreaterEqual => {
                self.emit_numeric_comparison(op, &left, &right)
            }
            BinaryOp::Equal | BinaryOp::NotEqual => self.emit_equality(op, &left, &right),
            BinaryOp::And | BinaryOp::Or => self.emit_boolean_binary(op, &left, &right),
        }
    }

    fn emit_numeric_comparison(
        &mut self,
        op: BinaryOp,
        left: &ValueRef,
        right: &ValueRef,
    ) -> Option<ValueRef> {
        if left.value_type != ValueType::Double || right.value_type != ValueType::Double {
            self.semantic_error("Comparison operators only support numeric values");
            return None;
        }

        let predicate = match op {
            BinaryOp::Less => "olt",
            BinaryOp::Greater => "ogt",
            BinaryOp::LessEqual => "ole",
            BinaryOp::GreaterEqual => "oge",
            _ => unreachable!("non-comparison operator in emit_numeric_comparison"),
        };

        let result = self.next_temp();
        self.emit_body(format!(
            "{result} = fcmp {predicate} double {}, {}",
            left.repr, right.repr
        ));
        Some(ValueRef {
            value_type: ValueType::Bool,
            repr: result,
        })
    }

    fn emit_equality(
        &mut self,
        op: BinaryOp,
        left: &ValueRef,
        right: &ValueRef,
    ) -> Option<ValueRef> {
        if left.value_type != right.value_type {
            self.semantic_error("Equality operators require operands of the same type");
            return None;
        }

        match left.value_type {
            ValueType::Double => {
                let predicate = match op {
                    BinaryOp::Equal => "oeq",
                    BinaryOp::NotEqual => "one",
                    _ => unreachable!("non-equality operator in emit_equality"),
                };

                let result = self.next_temp();
                self.emit_body(format!(
                    "{result} = fcmp {predicate} double {}, {}",
                    left.repr, right.repr
                ));
                Some(ValueRef {
                    value_type: ValueType::Bool,
                    repr: result,
                })
            }
            ValueType::Bool => {
                let predicate = match op {
                    BinaryOp::Equal => "eq",
                    BinaryOp::NotEqual => "ne",
                    _ => unreachable!("non-equality operator in emit_equality"),
                };

                let result = self.next_temp();
                self.emit_body(format!(
                    "{result} = icmp {predicate} i1 {}, {}",
                    left.repr, right.repr
                ));
                Some(ValueRef {
                    value_type: ValueType::Bool,
                    repr: result,
                })
            }
            ValueType::StringPtr => {
                let cmp_tmp = self.next_temp();
                self.emit_body(format!(
                    "{cmp_tmp} = call i32 @strcmp(i8* {}, i8* {})",
                    left.repr, right.repr
                ));

                let predicate = match op {
                    BinaryOp::Equal => "eq",
                    BinaryOp::NotEqual => "ne",
                    _ => unreachable!("non-equality operator in emit_equality"),
                };

                let result = self.next_temp();
                self.emit_body(format!("{result} = icmp {predicate} i32 {cmp_tmp}, 0"));
                Some(ValueRef {
                    value_type: ValueType::Bool,
                    repr: result,
                })
            }
        }
    }

    fn emit_boolean_binary(
        &mut self,
        op: BinaryOp,
        left: &ValueRef,
        right: &ValueRef,
    ) -> Option<ValueRef> {
        if left.value_type != ValueType::Bool || right.value_type != ValueType::Bool {
            self.semantic_error("Logical operators only support boolean values");
            return None;
        }

        let instruction = match op {
            BinaryOp::And => "and",
            BinaryOp::Or => "or",
            _ => unreachable!("non-logical operator in emit_boolean_binary"),
        };

        let result = self.next_temp();
        self.emit_body(format!(
            "{result} = {instruction} i1 {}, {}",
            left.repr, right.repr
        ));
        Some(ValueRef {
            value_type: ValueType::Bool,
            repr: result,
        })
    }

    fn emit_concat(&mut self, left: &ValueRef, right: &ValueRef) -> Option<ValueRef> {
        let (fmt_name, arg_values) = match (left.value_type, right.value_type) {
            (ValueType::StringPtr, ValueType::StringPtr) => (
                "@.fmt.concat.ss",
                format!("i8* {}, i8* {}", left.repr, right.repr),
            ),
            (ValueType::StringPtr, ValueType::Double) => (
                "@.fmt.concat.sn",
                format!("i8* {}, double {}", left.repr, right.repr),
            ),
            (ValueType::Double, ValueType::StringPtr) => (
                "@.fmt.concat.ns",
                format!("double {}, i8* {}", left.repr, right.repr),
            ),
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
            "declare i32 @strcmp(i8*, i8*)".to_string(),
            "declare i32 @rand()".to_string(),
            "declare i64 @time(i64*)".to_string(),
            "declare void @srand(i32)".to_string(),
            "declare double @llvm.sin.f64(double)".to_string(),
            "declare double @llvm.cos.f64(double)".to_string(),
            "declare double @llvm.sqrt.f64(double)".to_string(),
            "declare double @llvm.exp.f64(double)".to_string(),
            "declare double @llvm.log.f64(double)".to_string(),
            "declare double @llvm.pow.f64(double, double)".to_string(),
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
        out.push("  %t_seed_raw = call i64 @time(i64* null)".to_string());
        out.push("  %t_seed_i32 = trunc i64 %t_seed_raw to i32".to_string());
        out.push("  call void @srand(i32 %t_seed_i32)".to_string());

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser};

    fn compile_source(source: &str) -> Result<String, Vec<CompilerError>> {
        let mut lexer = Lexer::new(source.to_string());
        let tokens = lexer.lex();
        assert!(
            lexer.errors().is_empty(),
            "lexer produced errors: {:?}",
            lexer.errors()
        );

        let mut parser = Parser::new(source);
        let program = parser
            .parse_program(tokens)
            .expect("parser should return a program");

        let mut backend = LlvmBackend::new();
        backend.generate(&program)
    }

    #[test]
    fn generates_ir_for_block_expression_scope() {
        let source = "let y = 1; let x = { let x = 9; let z = 1; x + y }; print(x)";
        let ir = compile_source(source).expect("codegen should succeed");

        assert!(
            ir.contains("fadd double"),
            "expected block result to include addition"
        );
        assert!(
            ir.contains("@printf"),
            "expected printf call for print statement"
        );
    }
}
