//! Integration tests for neve-lsp crate.

use neve_lexer::Lexer;
use neve_lsp::{Document, SymbolIndex, generate_semantic_tokens};
use neve_parser::parse;

// Document tests

#[test]
fn test_document_new() {
    let doc = Document::new("file:///test.neve".to_string(), "let x = 1;".to_string());
    assert!(doc.ast.is_some());
}

#[test]
fn test_document_parse_error() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "let x = ".to_string(), // Incomplete
    );
    // Should still create document even with parse errors
    let _ = doc.diagnostics.len(); // Just verify it exists
}

#[test]
fn test_position_at() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "let x = 1;\nlet y = 2;".to_string(),
    );
    assert_eq!(doc.position_at(0), (0, 0));
    assert_eq!(doc.position_at(11), (1, 0));
}

#[test]
fn test_position_at_end() {
    let doc = Document::new("file:///test.neve".to_string(), "abc".to_string());
    assert_eq!(doc.position_at(0), (0, 0));
    assert_eq!(doc.position_at(1), (0, 1));
    assert_eq!(doc.position_at(2), (0, 2));
}

// Semantic tokens tests

#[test]
fn test_generate_semantic_tokens() {
    let source = "let x = 42;";
    let lexer = Lexer::new(source);
    let (tokens, _) = lexer.tokenize();
    let semantic = generate_semantic_tokens(&tokens, source);

    // Should have: let (keyword), x (variable), 42 (number)
    assert!(semantic.len() >= 3);
}

#[test]
fn test_semantic_tokens_function() {
    let source = "fn add(x, y) = x + y;";
    let lexer = Lexer::new(source);
    let (tokens, _) = lexer.tokenize();
    let semantic = generate_semantic_tokens(&tokens, source);

    // Should include fn keyword, function name, parameters
    assert!(semantic.len() >= 4);
}

// Symbol index tests

#[test]
fn test_function_definition() {
    let source = "fn add(x: Int, y: Int) = x + y;";
    let (ast, _) = parse(source);
    let index = SymbolIndex::from_ast(&ast);

    assert!(index.definitions.contains_key("add"));
    assert!(index.definitions.contains_key("x"));
    assert!(index.definitions.contains_key("y"));
}

#[test]
fn test_variable_references() {
    let source = "let x = 1; let y = x + 2;";
    let (ast, _) = parse(source);
    let index = SymbolIndex::from_ast(&ast);

    let x_refs = index.get_references("x");
    assert!(x_refs.len() >= 2); // Definition + usage
}

#[test]
fn test_find_definition() {
    let source = "fn foo() = 42; let x = foo();";
    let (ast, _) = parse(source);
    let index = SymbolIndex::from_ast(&ast);

    // Find the reference to foo in "foo()"
    let foo_refs: Vec<_> = index
        .references
        .iter()
        .filter(|r| r.name == "foo" && !r.is_write)
        .collect();

    assert!(!foo_refs.is_empty());
}

#[test]
fn test_let_definition() {
    let source = "let myVar = 100;";
    let (ast, _) = parse(source);
    let index = SymbolIndex::from_ast(&ast);

    assert!(index.definitions.contains_key("myVar"));
}

#[test]
fn test_nested_references() {
    // Use block syntax for let expression inside function body
    let source = "fn outer(x) = { let inner = x * 2; inner + x };";
    let (ast, _) = parse(source);
    let index = SymbolIndex::from_ast(&ast);

    // x should be referenced multiple times
    let x_refs = index.get_references("x");
    assert!(x_refs.len() >= 2);
}
