use crate::parser::expression::Literal;

use super::super::super::LlvmBackend;
use crate::codegen::llvm::helper::{
    module_writer::{escape_llvm_string, format_double},
    state::{ValueRef, ValueType},
};

impl LlvmBackend {
    pub(in crate::codegen::llvm) fn emit_literal(&mut self, literal: &Literal) -> Option<ValueRef> {
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
            Literal::Null => {
                Some(ValueRef {
                    value_type: ValueType::Null,
                    repr: "null".to_string(),
                })
            }
        }
    }
}
