use std::thread::current;

use crate::{lexer::{Token, TokenKind}, parser::expression::{BinaryExpr, BinaryOp, Expr, Literal, UnaryExpr, UnaryOp, PrintExpr}};
pub mod expression;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    current_token: Token,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        let first = tokens[0].clone();
    
        Self {
            tokens,
            pos: 0,
            current_token: first,
        }
    }


    fn advance(&mut self) {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
            let x = self.pos;
            self.current_token = self.tokens[x].clone();
        }
    }

    fn match_token(&mut self, expected: &TokenKind) -> bool {
        if &self.current_token.kind == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn parse_expression(&mut self) -> Expr {
        match &self.current_token.kind {
            TokenKind::Print => self.parse_print(),
            _ => self.parse_binary(),
        }
    }


    fn parse_print(&mut self) -> Expr {
        // consumir 'print'
        self.advance();
    
        // verificar '('
        if !matches!(self.current_token.kind, TokenKind::LeftParen) {
            panic!("Expected '(' after print");
        }
        self.advance();
    
        // verificar string
        let message = match &self.current_token.kind {
            TokenKind::String(s) => s.clone(),
            _ => panic!("Expected string literal inside print"),
        };
        self.advance();
    
        // verificar ')'
        if !matches!(self.current_token.kind, TokenKind::RightParen) {
            panic!("Expected ')' after string literal");
        }
        self.advance();

        Expr::Print(message)
    
    }
    
    fn parse_binary(&mut self) -> Expr {
        let mut expr = self.parse_factor();

        while matches!(self.current_token.kind, TokenKind::Add | TokenKind::Minus) {
            let op = match self.current_token.kind {
                TokenKind::Add => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => unreachable!(),
            };

            self.advance();
            let right = self.parse_factor();

            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            });
        }

        expr
    }

    fn parse_factor(&mut self) -> Expr {
        let mut expr = self.parse_unary();

        while matches!(self.current_token.kind, TokenKind::Multiply | TokenKind::Divide) {
            let op = match self.current_token.kind {
                TokenKind::Multiply => BinaryOp::Mul,
                TokenKind::Divide => BinaryOp::Div,
                _ => unreachable!(),
            };

            self.advance();
            let right = self.parse_unary();

            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            });
        }

        expr
    }

    fn parse_unary(&mut self) -> Expr {
        if matches!(self.current_token.kind, TokenKind::Minus) {
            self.advance();
            let expr = self.parse_unary();

            return Expr::Unary(UnaryExpr {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Expr {
        let token = self.current_token.clone();

        match token.kind {
            TokenKind::Number(n) => {
                self.advance();

                if n.contains('.') {
                    Expr::Atomic(Literal::Float(n.parse().unwrap()))
                } else {
                    Expr::Atomic(Literal::Integer(n.parse().unwrap()))
                }
            }

            TokenKind::Boolean(b) => {
                self.advance();
                Expr::Atomic(Literal::Boolean(b == "true"))
            }

            TokenKind::String(s) => {
                self.advance();
                Expr::Atomic(Literal::String(s))
            }

            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parse_expression();
                self.advance(); // consume ')'
                expr
            }

            _ => panic!("Unexpected token: {:?}", token),
        }
    }
}