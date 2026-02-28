// mod lexer;
// mod parser;

// use parser::Parser;

// fn main() {
//     let x = "3 + 9 +1- 2 * 51 /2.19".to_string();

//     let mut lexer = lexer::Lexer::new(x);
//     let tokens = lexer.lex();

//     let mut parser = Parser::new(tokens);
//     let ast = parser.parse_expression();

//     println!("{:#?}", ast);
// }

mod lexer;
mod parser;

use parser::Parser;

fn run(source: &str) {
    println!("==============================");
    println!("SOURCE:\n{}\n", source);

    let mut lexer = lexer::Lexer::new(source.to_string());
    let tokens = lexer.lex();

    println!("TOKENS:");
    for t in &tokens {
        println!("{:?}", t);
    }

    let mut parser = Parser::new(tokens);
    let ast = parser.parse_expression();

    println!("\nAST:");
    println!("{:#?}", ast);
    println!("==============================\n");
}

fn main() {
    // Tu expresión original
    run("3 + 9 + 1 - 2 * 51 / 2.19");

    // Ahora probamos print
    run(r#"print("Hola mundo")"#);
}