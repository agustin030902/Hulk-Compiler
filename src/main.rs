mod lexer;
fn main() {
    let x = "3 +      9 +1- 2 * 51 /2.19; \n \n 3 + 9*3.9001".to_string();
    let mut lexer = lexer::Lexer::new(x);
    let tokens = lexer.lex();

    for token in tokens {
        println!("{:?}", token);
    }

}

#[derive(Clone, Debug)]
struct Dog {
    name: String,
    age: i32,
}

impl Dog {
    pub fn new(name: String, age: i32) -> Self {
        Self { name, age }
    }

    pub fn set_age(&mut self, age: i32) {
        self.age = age;
    }

    pub fn age(&self) -> i32 {
        self.age
    }

    pub fn name(&self) -> &String {
        &self.name
    }
}

fn dogs_names(dogs: Vec<Dog>) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for dog in dogs {
        names.push(dog.name);
    }
    names
}