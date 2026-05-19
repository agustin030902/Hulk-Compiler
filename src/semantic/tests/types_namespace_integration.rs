use crate::{error::ErrorCategory, lexer::Lexer, parser::Parser};

use super::super::SemanticAnalyzer;

fn analyze_with_state(source: &str) -> (SemanticAnalyzer, Vec<crate::error::CompilerError>) {
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
    let errors = analyzer.analyze(&program.expect("program"), source);
    (analyzer, errors)
}

#[test]
fn registers_functions_in_symbol_table_and_types_namespace() {
    let source = r#"
function add(x: Number, y: Number): Number => x + y;
print(add(1, 2));
"#;

    let (analyzer, errors) = analyze_with_state(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );

    let signatures = analyzer.function_signatures();
    let symbols = analyzer.function_symbols();

    let signature = signatures.get("add").expect("missing signature for add");
    let symbol = symbols.get("add").expect("missing symbol for add");

    assert!(symbol.is_function());
    assert!(!symbol.is_method());
    assert_eq!(signature.type_id, symbol.type_id.0);

    let function_info = analyzer
        .type_table()
        .get_function(symbol.type_id)
        .expect("missing function type entry in TypeTable");
    assert!(function_info.is_function());
    assert_eq!(function_info.receiver, None);
}

#[test]
fn keeps_types_namespace_function_entry_synced_after_inference() {
    let source = r#"
function id(x) => x;
function use_it(y: Number): Number => id(y) + 1;
print(use_it(41));
"#;

    let (analyzer, errors) = analyze_with_state(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );

    let id_symbol = analyzer
        .function_symbols()
        .get("id")
        .expect("missing symbol for id");
    let table = analyzer.type_table();
    let id_info = table
        .get_function(id_symbol.type_id)
        .expect("missing function type entry for id");

    assert_eq!(id_info.params, vec![table.number]);
    assert_eq!(id_info.return_type, table.number);
}

#[test]
fn does_not_duplicate_function_symbol_when_redeclared() {
    let source = r#"
function f(x) => x;
function f(y) => y;
print(f(1));
"#;

    let (analyzer, errors) = analyze_with_state(source);

    assert!(
        errors
            .iter()
            .any(|error| error.category == ErrorCategory::Semantic
                && error.message == "Function 'f' redeclared."),
        "expected redeclaration error, got: {:?}",
        errors
    );
    assert_eq!(analyzer.function_symbols().len(), 1);
}

#[test]
fn registers_type_methods_as_function_symbols_with_receiver() {
    let source = r#"
type Point(x: Number) {
    x = x;
    getX() => self.x;
}

let p = new Point(1);
print(p.getX());
"#;

    let (analyzer, errors) = analyze_with_state(source);
    assert!(
        errors.is_empty(),
        "expected no semantic errors, got: {:?}",
        errors
    );

    let point_id = analyzer
        .type_symbols()
        .get("Point")
        .copied()
        .expect("missing type id for Point");
    let method_key = format!("type#{}::getX", point_id.0);

    let symbol = analyzer
        .function_symbols()
        .get(&method_key)
        .expect("missing method symbol for Point.getX");
    assert!(symbol.is_method());
    assert_eq!(symbol.receiver, Some(point_id));

    let method_info = analyzer
        .type_table()
        .get_function(symbol.type_id)
        .expect("missing method function type entry in TypeTable");
    assert_eq!(method_info.receiver, Some(point_id));
    assert!(method_info.params.is_empty());
}
