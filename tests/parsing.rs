// Integration tests for the parser pipeline: Lexer → Parser → AST
//
// Tests the complete parsing flow from source code to AST,
// ensuring all language constructs are properly recognized.

use neve_lexer::Lexer;
use neve_parser::Parser;
use neve_syntax::ast::*;

#[test]
fn test_parse_basic_function() {
    let source = r#"
        fn add(x, y) = x + y;
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);

    match &module.items[0] {
        Item::Fn(func) => {
            assert_eq!(func.name.name, "add");
            assert_eq!(func.params.len(), 2);
        }
        _ => panic!("Expected function definition"),
    }
}

#[test]
fn test_parse_record_literal() {
    let source = r#"
        let config = #{
            name = "Neve",
            version = "0.1.0",
            debug = true,
        };
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);

    match &module.items[0] {
        Item::Let(let_stmt) => {
            assert_eq!(let_stmt.name.name, "config");
            match &let_stmt.value.kind {
                ExprKind::Record(_) => {} // Success
                _ => panic!("Expected record literal"),
            }
        }
        _ => panic!("Expected let binding"),
    }
}

#[test]
fn test_parse_trait_with_associated_types() {
    let source = r#"
        pub trait Iterator {
            type Item;
            type Error: Show;

            fn next(self) -> Option<Self.Item>;
        };
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);

    match &module.items[0] {
        Item::Trait(trait_def) => {
            assert_eq!(trait_def.name.name, "Iterator");
            assert_eq!(trait_def.assoc_types.len(), 2);

            // Check first associated type
            assert_eq!(trait_def.assoc_types[0].name.name, "Item");
            assert!(trait_def.assoc_types[0].bounds.is_empty());

            // Check second associated type with bounds
            assert_eq!(trait_def.assoc_types[1].name.name, "Error");
            assert_eq!(trait_def.assoc_types[1].bounds.len(), 1);
        }
        _ => panic!("Expected trait definition"),
    }
}

#[test]
fn test_parse_impl_with_associated_types() {
    let source = r#"
        impl Iterator for List<a> {
            type Item = a;

            fn next(self) -> Option<Self.Item> {
                match self {
                    [] -> None,
                    [x, ..xs] -> Some(x),
                }
            }
        };
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);

    match &module.items[0] {
        Item::Impl(impl_def) => {
            assert_eq!(impl_def.assoc_type_impls.len(), 1);
            assert_eq!(impl_def.assoc_type_impls[0].name.name, "Item");
        }
        _ => panic!("Expected impl definition"),
    }
}

#[test]
fn test_parse_pattern_matching() {
    let source = r#"
        fn length(list) = match list {
            [] -> 0,
            [_, ..rest] -> 1 + length(rest),
        };
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);

    match &module.items[0] {
        Item::Fn(func) => {
            assert_eq!(func.name.name, "length");
            match &func.body.kind {
                ExprKind::Match(_, arms) => {
                    assert_eq!(arms.len(), 2);
                }
                _ => panic!("Expected match expression"),
            }
        }
        _ => panic!("Expected function definition"),
    }
}

#[test]
fn test_parse_generics() {
    let source = r#"
        fn map<a, b>(f: a -> b, list: List<a>) -> List<b> {
            match list {
                [] -> [],
                [x, ..xs] -> [f(x), ..map(f, xs)],
            }
        }
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);

    match &module.items[0] {
        Item::Fn(func) => {
            assert_eq!(func.name.name, "map");
            assert_eq!(func.generics.len(), 2);
            assert_eq!(func.params.len(), 2);
        }
        _ => panic!("Expected function definition"),
    }
}

#[test]
fn test_parse_module_imports() {
    let source = r#"
        use std::list::{map, filter};
        use self::utils::helper;
        use super::config;

        fn process(data) = map(helper, filter(config.pred, data));
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(result.is_ok());
    let module = result.unwrap();

    // Count use statements
    let use_count = module.items.iter().filter(|item| matches!(item, Item::Use(_))).count();
    assert_eq!(use_count, 3);

    // Count function definitions
    let fn_count = module.items.iter().filter(|item| matches!(item, Item::Fn(_))).count();
    assert_eq!(fn_count, 1);
}

#[test]
fn test_parse_pipe_operator() {
    let source = r#"
        fn process(data) =
            data
            |> filter(isValid)
            |> map(transform)
            |> fold(0, add);
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);

    match &module.items[0] {
        Item::Fn(func) => {
            assert_eq!(func.name.name, "process");
            // Pipe operator creates nested function calls
            match &func.body.kind {
                ExprKind::Call(_, _) => {} // Success
                _ => panic!("Expected function call from pipe"),
            }
        }
        _ => panic!("Expected function definition"),
    }
}

#[test]
fn test_parse_derivation() {
    let source = r#"
        let myPackage = derivation {
            name = "hello",
            version = "1.0.0",

            src = fetchurl {
                url = "https://example.com/hello.tar.gz",
                hash = "sha256-abc123",
            },

            buildPhase = ''
                make -j$NIX_BUILD_CORES
            '',
        };
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

#[test]
fn test_parse_error_recovery() {
    // Test that parser can report errors without crashing
    let source = r#"
        fn broken(x = x +;  // Syntax error
        fn working(y) = y * 2;
    "#;

    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    // Parser should return error but not panic
    assert!(result.is_err() || result.unwrap().items.len() > 0);
}
