use std::collections::HashMap;
#[cfg(test)]
mod tests;

use crate::{
    error::{CompilerError, ErrorCategory, offset_to_line_column},
    parser::expression::{BinaryOp, Expr, Literal, Program, Span, Statement, UnaryOp},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticType {
    Number,
    Boolean,
    String,
    Unknown,
}

impl SemanticType {
    fn display_name(self) -> &'static str {
        match self {
            SemanticType::Number => "Number",
            SemanticType::Boolean => "Boolean",
            SemanticType::String => "String",
            SemanticType::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Default)]
pub struct SemanticAnalyzer {
    symbols: HashMap<String, SemanticType>,
    errors: Vec<CompilerError>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn analyze(&mut self, program: &Program, source: &str) -> Vec<CompilerError> {
        self.symbols.clear();
        self.errors.clear();

        for statement in &program.statements {
            self.check_statement(statement, source);
        }

        self.errors.clone()
    }

    fn check_statement(&mut self, statement: &Statement, source: &str) {
        match statement {
            Statement::Let {
                name,
                name_span,
                value,
                ..
            } => {
                if self.symbols.contains_key(name) {
                    self.push_semantic_error(
                        *name_span,
                        source,
                        format!(
                            "Variable '{}' redeclared. A variable can only be declared once.",
                            name
                        ),
                    );
                    return;
                }

                let value_type = self
                    .check_expr(value, source)
                    .unwrap_or(SemanticType::Unknown);
                self.symbols.insert(name.clone(), value_type);
            }
            Statement::Print { value, .. } => {
                let _ = self.check_expr(value, source);
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr, source: &str) -> Option<SemanticType> {
        match expr {
            Expr::Literal { value, .. } => Some(self.type_of_literal(value)),
            Expr::Variable { name, span } => {
                if let Some(var_type) = self.symbols.get(name).copied() {
                    Some(var_type)
                } else {
                    self.push_semantic_error(
                        *span,
                        source,
                        format!(
                            "Variable '{}' is used before declaration. Declare it with 'let' first.",
                            name
                        ),
                    );
                    None
                }
            }
            Expr::Unary(unary) => {
                let expr_type = self.check_expr(&unary.expr, source)?;
                match unary.op {
                    UnaryOp::Neg => {
                        if expr_type == SemanticType::Unknown {
                            return Some(SemanticType::Unknown);
                        }

                        if expr_type == SemanticType::Number {
                            Some(SemanticType::Number)
                        } else {
                            self.push_type_error(
                                unary.span,
                                source,
                                format!(
                                    "Unary '-' expects Number, but got {}.",
                                    expr_type.display_name()
                                ),
                            );
                            None
                        }
                    }
                }
            }
            Expr::Binary(binary) => {
                let left_type = self.check_expr(&binary.left, source);
                let right_type = self.check_expr(&binary.right, source);

                let (Some(left_type), Some(right_type)) = (left_type, right_type) else {
                    return None;
                };

                match binary.op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                        if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown
                        {
                            return Some(SemanticType::Unknown);
                        }

                        if left_type == SemanticType::Number && right_type == SemanticType::Number {
                            Some(SemanticType::Number)
                        } else {
                            let op_name = op_symbol(binary.op.clone());
                            self.push_type_error(
                                binary.span,
                                source,
                                format!(
                                    "Operator '{}' expects Number and Number, but got {} and {}.",
                                    op_name,
                                    left_type.display_name(),
                                    right_type.display_name()
                                ),
                            );
                            None
                        }
                    }
                    BinaryOp::Concat => {
                        if left_type == SemanticType::Unknown || right_type == SemanticType::Unknown
                        {
                            return Some(SemanticType::Unknown);
                        }

                        if is_valid_concat_pair(left_type, right_type) {
                            Some(SemanticType::String)
                        } else {
                            self.push_type_error(
                                binary.span,
                                source,
                                format!(
                                    "Operator '@' expects (String, String), (String, Number), or (Number, String), but got {} and {}.",
                                    left_type.display_name(),
                                    right_type.display_name()
                                ),
                            );
                            None
                        }
                    }
                }
            }
        }
    }

    fn type_of_literal(&self, literal: &Literal) -> SemanticType {
        match literal {
            Literal::Integer(_) | Literal::Float(_) => SemanticType::Number,
            Literal::Boolean(_) => SemanticType::Boolean,
            Literal::String(_) => SemanticType::String,
        }
    }

    fn push_type_error(&mut self, span: Span, source: &str, message: String) {
        let (line, column) = offset_to_line_column(source, span.start);
        self.errors.push(CompilerError::new(
            ErrorCategory::Type,
            message,
            line,
            column,
        ));
    }

    fn push_semantic_error(&mut self, span: Span, source: &str, message: String) {
        let (line, column) = offset_to_line_column(source, span.start);
        self.errors.push(CompilerError::new(
            ErrorCategory::Semantic,
            message,
            line,
            column,
        ));
    }
}

fn op_symbol(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Concat => "@",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
    }
}

fn is_valid_concat_pair(left: SemanticType, right: SemanticType) -> bool {
    matches!(
        (left, right),
        (SemanticType::String, SemanticType::String)
            | (SemanticType::String, SemanticType::Number)
            | (SemanticType::Number, SemanticType::String)
    )
}
