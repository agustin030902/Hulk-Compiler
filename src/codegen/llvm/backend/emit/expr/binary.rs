use crate::parser::expression::{BinaryExpr, BinaryOp};

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::{
    module_writer::format_ptr_global,
    state::{ValueRef, ValueType},
};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_binary_expr(
        &mut self,
        binary: &BinaryExpr,
    ) -> Option<ValueRef> {
        let left = self.emit_expr(&binary.left)?;
        let right = self.emit_expr(&binary.right)?;

        match binary.op {
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

                let instruction = match binary.op {
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
                self.emit_numeric_comparison(binary.op.clone(), &left, &right)
            }
            BinaryOp::Equal | BinaryOp::NotEqual => {
                self.emit_equality(binary.op.clone(), &left, &right)
            }
            BinaryOp::And | BinaryOp::Or => {
                self.emit_boolean_binary(binary.op.clone(), &left, &right)
            }
        }
    }

    pub(in crate::codegen::llvm) fn emit_numeric_comparison(
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

    pub(in crate::codegen::llvm) fn emit_equality(
        &mut self,
        op: BinaryOp,
        left: &ValueRef,
        right: &ValueRef,
    ) -> Option<ValueRef> {
        if !self.is_assignable_value_type(left.value_type, right.value_type)
            && !self.is_assignable_value_type(right.value_type, left.value_type)
        {
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
            ValueType::Null | ValueType::Function | ValueType::Struct(_) => {
                let predicate = match op {
                    BinaryOp::Equal => "eq",
                    BinaryOp::NotEqual => "ne",
                    _ => unreachable!("non-equality operator in emit_equality"),
                };

                let result = self.next_temp();
                self.emit_body(format!(
                    "{result} = icmp {predicate} i8* {}, {}",
                    left.repr, right.repr
                ));
                Some(ValueRef {
                    value_type: ValueType::Bool,
                    repr: result,
                })
            }
            ValueType::Unit => {
                self.semantic_error("Equality operators do not support Unit values");
                None
            }
        }
    }

    pub(in crate::codegen::llvm) fn emit_boolean_binary(
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

    pub(in crate::codegen::llvm) fn emit_concat(
        &mut self,
        left: &ValueRef,
        right: &ValueRef,
    ) -> Option<ValueRef> {
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
                    left.value_type.display_name(),
                    right.value_type.display_name()
                ));
                return None;
            }
        };

        let result_slot = self.next_temp();
        self.emit_body(format!("{result_slot} = alloca i8*"));

        let call_tmp = self.next_temp();
        let fmt_ptr = format_ptr_global(fmt_name, 5);
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
}
