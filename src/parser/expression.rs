// pub enum Expr {
//     Binary(BinaryExpr),
//     Unary(UnaryExpr),
//     Atomic(Literal),
//     Print(String)
// }

// #[derive(Debug)]
// pub enum Literal {
//     Integer(i64),
//     Float(f64),
//     Boolean(bool),
//     String(String),
// }

// #[derive(Debug)]
// pub struct BinaryExpr {
//     pub left: Box<Expr>,
//     pub op: BinaryOp,
//     pub right: Box<Expr>,
// }

// #[derive(Debug)]
// pub struct UnaryExpr {
//     pub op: UnaryOp,
//     pub expr: Box<Expr>,
// }

// #[derive(Debug)]
// pub enum BinaryOp {
//     Add,
//     Sub,
//     Mul,
//     Div,
// }

// #[derive(Debug)]
// pub enum UnaryOp {
//     Neg,
// }


use core::str;
use std::fmt;

#[derive(Debug)]
pub enum Expr {
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Atomic(Literal),
    Print(String),
}

#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
}

#[derive(Debug)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub op: BinaryOp,
    pub right: Box<Expr>,
}

#[derive(Debug)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub expr: Box<Expr>,
}

#[derive(Debug)]
pub struct PrintExpr {
    pub message: Expr,
}

#[derive(Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug)]
pub enum UnaryOp {
    Neg,
}

#[derive(Debug)]
pub enum EvalError {
    TypeError,
    DivisionByZero,
}

impl Expr {
    pub fn eval(&self) -> Result<Literal, EvalError> {
        match self {
            Expr::Atomic(lit) => Ok(lit.clone()),

            Expr::Unary(unary) => {
                let value = unary.expr.eval()?;
                match (&unary.op, value) {
                    (UnaryOp::Neg, Literal::Integer(i)) => {
                        Ok(Literal::Integer(-i))
                    }
                    (UnaryOp::Neg, Literal::Float(f)) => {
                        Ok(Literal::Float(-f))
                    }
                    _ => Err(EvalError::TypeError),
                }
            }

            Expr::Binary(binary) => {
                let left = binary.left.eval()?;
                let right = binary.right.eval()?;

                match (&binary.op, left, right) {

                    // Integer ops
                    (BinaryOp::Add, Literal::Integer(a), Literal::Integer(b)) =>
                        Ok(Literal::Integer(a + b)),

                    (BinaryOp::Sub, Literal::Integer(a), Literal::Integer(b)) =>
                        Ok(Literal::Integer(a - b)),

                    (BinaryOp::Mul, Literal::Integer(a), Literal::Integer(b)) =>
                        Ok(Literal::Integer(a * b)),

                    (BinaryOp::Div, Literal::Integer(a), Literal::Integer(b)) => {
                        if b == 0 {
                            Err(EvalError::DivisionByZero)
                        } else {
                            Ok(Literal::Integer(a / b))
                        }
                    }

                    // Float ops
                    (BinaryOp::Add, Literal::Float(a), Literal::Float(b)) =>
                        Ok(Literal::Float(a + b)),

                    (BinaryOp::Sub, Literal::Float(a), Literal::Float(b)) =>
                        Ok(Literal::Float(a - b)),

                    (BinaryOp::Mul, Literal::Float(a), Literal::Float(b)) =>
                        Ok(Literal::Float(a * b)),

                    (BinaryOp::Div, Literal::Float(a), Literal::Float(b)) => {
                        if b == 0.0 {
                            Err(EvalError::DivisionByZero)
                        } else {
                            Ok(Literal::Float(a / b))
                        }
                    }

                    _ => Err(EvalError::TypeError),
                }
            }

            Expr::Print(msg) => {
                println!("{}", msg);
                Ok(Literal::Boolean(true))
            }
        }
    }
}