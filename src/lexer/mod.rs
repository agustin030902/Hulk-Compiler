mod token;
//mod error;
 
pub use token::Token;
pub use token::TokenKind;


pub struct Lexer{
    input: String,
    position: i32,
    raw: i32,
    column: i32,
    current_char: Option<char>,
    token_list: Vec<Token>,
}

impl Lexer {
    pub fn new(input: String) -> Self {
        let mut lexer = Lexer {
            input,
            raw: 0,
            position: 0,
            column: 0,
            current_char: None,
            token_list: Vec::new(),
        };
        lexer.current_char = lexer.input.chars().nth(0);
        lexer
    }
    
    pub fn lex(&mut self) -> Vec<Token> {
        while let Some(c) = self.current_char {
    
            if c.is_whitespace() {
                self.skip_whitespace();
    
            } else if c.is_ascii_digit() {
                let number_token = self.lex_number();
                self.token_list.push(number_token);
    
            } else if c.is_alphabetic() {
                let ident_token = self.lex_identifier();
                self.token_list.push(ident_token);
    
            } else if c == '"' {
                let string_token = self.lex_string();
                self.token_list.push(string_token);
    
            } else if c == '(' {
                self.token_list.push(Token {
                    kind: TokenKind::LeftParen,
                    value: "(".to_string(),
                    line: self.raw,
                    column: self.column,
                });
                self.advance();
    
            } else if c == ')' {
                self.token_list.push(Token {
                    kind: TokenKind::RightParen,
                    value: ")".to_string(),
                    line: self.raw,
                    column: self.column,
                });
                self.advance();
    
            } else if "+-*/".contains(c) {
                let operator_token = self.lex_operator();
                self.token_list.push(operator_token);
    
            } else {
                self.advance();
            }
        }
    
        self.token_list.push(Token {
            kind: TokenKind::EOF,
            value: String::new(),
            line: self.raw,
            column: self.column,
        });
    
        self.token_list.clone()
    }

    pub fn advance(&mut self) {
        self.column += 1;
        //self.raw += 1;
        self.position += 1;
        self.current_char = self.input.chars().nth(self.position as usize);
    }

    pub fn lex_identifier(&mut self) -> Token {
        let start_raw = self.raw;
        let start_column = self.column;
        let mut value = String::new();
    
        while let Some(c) = self.current_char {
            if c.is_alphanumeric() {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }
    
        let kind = match value.as_str() {
            "print" => TokenKind::Print,
            "true" | "false" => TokenKind::Boolean(value.clone()),
            _ => TokenKind::Identifier(value.clone()),
        };
    
        Token {
            kind,
            value,
            line: start_raw,
            column: start_column,
        }
    }

    pub fn lex_string(&mut self) -> Token {
        let start_raw = self.raw;
        let start_column = self.column;
    
        self.advance(); // consumir comilla inicial
    
        let mut value = String::new();
    
        while let Some(c) = self.current_char {
            if c == '"' {
                break;
            }
            value.push(c);
            self.advance();
        }
    
        self.advance(); // consumir comilla final
    
        Token {
            kind: TokenKind::String(value.clone()),
            value,
            line: start_raw,
            column: start_column,
        }
    }

    pub fn skip_whitespace(&mut self) {
        while let Some(c) = self.current_char {
            if c.is_whitespace() {
                if c == '\n' {
                    self.column = 0;
                    self.raw += 1;
                }
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn lex_number(&mut self) -> Token {
        let start_raw = self.raw;
        let start_column = self.column;
        let mut value = String::new();

        while let Some(c) = self.current_char {
            if c.is_digit(10) || c == '.' {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }

        Token {
            kind: TokenKind::Number(value.clone()),
            value,
            line: start_raw,
            column: start_column,
        }
    }

    pub fn lex_operator(&mut self) -> Token {
        let start_raw = self.raw;
        let start_column = self.column;
        let value = self.current_char.unwrap().to_string();
        self.advance();

        Token {
            kind: match value.as_str() {
                "+" => TokenKind::Add,
                "-" => TokenKind::Minus,
                "*" => TokenKind::Multiply,
                "/" => TokenKind::Divide,
                _ => TokenKind::Unknown,
            },
            value,
            line: start_raw,
            column: start_column,
        }
    }
}   