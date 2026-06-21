use crate::lexer::Lexer;
use crate::parser::Parser;

use super::SemanticAnalyzer;

fn analyze_source(source: &str) -> Vec<crate::error::CompilerError> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.lex();
    assert!(
        !lexer.has_errors(),
        "lexer produced errors: {:?}",
        lexer.errors()
    );

    let mut parser = Parser::new(source);
    let program = parser.parse_program(tokens);
    assert!(
        !parser.has_errors(),
        "parser produced errors: {:?}",
        parser.errors()
    );

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(
        &program.expect("parser did not produce program for semantic analysis"),
        source,
    )
}

#[test]
fn allows_covariant_return_type() {
    let source = r#"
    type Animal { }
    type Dog inherits Animal { }
    interface Walker {
        walk(): Animal;
    }
    type FourLeggedWalker {
        walk(): Dog => new Dog();
    }
    let w: Walker = new FourLeggedWalker();
    "#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors for covariant return, got: {:?}",
        errors
    );
}

#[test]
fn allows_contravariant_parameter_type() {
    let source = r#"
    type Animal { }
    type Dog inherits Animal { }
    interface Feeder {
        feed(animal: Dog): Unit;
    }
    type GenericFeeder {
        feed(animal: Animal): Unit => print("fed");
    }
    let f: Feeder = new GenericFeeder();
    "#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors for contravariant param, got: {:?}",
        errors
    );
}

#[test]
fn allows_exact_signature_match() {
    let source = r#"
    interface Walker {
        walk(): Number;
    }
    type SimpleWalker {
        walk(): Number => 42;
    }
    let w: Walker = new SimpleWalker();
    "#;

    let errors = analyze_source(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors for exact signature match, got: {:?}",
        errors
    );
}

#[test]
fn rejects_covariant_parameter_type() {
    let source = r#"
    type Animal { }
    type Dog inherits Animal { }
    interface Feeder {
        feed(animal: Animal): Unit;
    }
    type PickyFeeder {
        feed(animal: Dog): Unit => print("fed dog");
    }
    let f: Feeder = new PickyFeeder();
    "#;

    let errors = analyze_source(source);
    assert!(
        !errors.is_empty(),
        "expected semantic error for covariant param (too restrictive), got none"
    );
    assert!(
        errors[0].message.contains("conform"),
        "expected conformance error message, got: {}",
        errors[0].message
    );
}

#[test]
fn rejects_contravariant_return_type() {
    let source = r#"
    type Animal { }
    type Dog inherits Animal { }
    interface DogProvider {
        provide(): Dog;
    }
    type AnimalProvider {
        provide(): Animal => new Animal();
    }
    let p: DogProvider = new AnimalProvider();
    "#;

    let errors = analyze_source(source);
    assert!(
        !errors.is_empty(),
        "expected semantic error for contravariant return (too general), got none"
    );
    assert!(
        errors[0].message.contains("conform"),
        "expected conformance error message, got: {}",
        errors[0].message
    );
}
