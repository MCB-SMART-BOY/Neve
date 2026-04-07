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
fn test_document_builds_semantic_hover_for_generic_function() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "fn id<T>(x: T) -> T = x;".to_string(),
    );
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("id")
        .and_then(|defs| defs.first())
        .expect("function definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "fn id: forall T. (T) -> T");
}

#[test]
fn test_document_hover_uses_local_type_names() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "struct User {}; fn id(x: User) -> User = x;".to_string(),
    );
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("id")
        .and_then(|defs| defs.first())
        .expect("function definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "fn id: (User) -> User");
}

#[test]
fn test_document_hover_includes_local_parameters_and_lets() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "fn add(x: Int, y: Int) -> Int = { let sum = x + y; sum };".to_string(),
    );
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");

    let x_symbol = index
        .get_definitions("x")
        .and_then(|defs| defs.first())
        .expect("parameter definition should be indexed");
    let x_hover = doc
        .definition_hovers
        .get(&x_symbol.def_span)
        .expect("parameter semantic hover should exist");
    assert_eq!(x_hover, "x: Int");

    let sum_symbol = index
        .get_definitions("sum")
        .and_then(|defs| defs.first())
        .expect("local let definition should be indexed");
    let sum_hover = doc
        .definition_hovers
        .get(&sum_symbol.def_span)
        .expect("local let semantic hover should exist");
    assert_eq!(sum_hover, "sum: Int");
}

#[test]
fn test_document_hover_includes_typed_lambda_parameters() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "let f = fn(x: Int) x;".to_string(),
    );
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");

    let x_symbol = index
        .get_definitions("x")
        .and_then(|defs| defs.first())
        .expect("lambda parameter definition should be indexed");
    let x_hover = doc
        .definition_hovers
        .get(&x_symbol.def_span)
        .expect("lambda parameter semantic hover should exist");
    assert_eq!(x_hover, "x: Int");
}

#[test]
fn test_document_hover_includes_block_pattern_bindings() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "fn sum_pair() = { let (x, y) = (1, 2); x + y };".to_string(),
    );
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");

    let x_symbol = index
        .get_definitions("x")
        .and_then(|defs| defs.first())
        .expect("tuple binding x should be indexed");
    let x_hover = doc
        .definition_hovers
        .get(&x_symbol.def_span)
        .expect("tuple binding x hover should exist");
    assert_eq!(x_hover, "x: Int");

    let y_symbol = index
        .get_definitions("y")
        .and_then(|defs| defs.first())
        .expect("tuple binding y should be indexed");
    let y_hover = doc
        .definition_hovers
        .get(&y_symbol.def_span)
        .expect("tuple binding y hover should exist");
    assert_eq!(y_hover, "y: Int");
}

#[test]
fn test_document_semantic_hover_includes_local_reference_type() {
    let source = "fn add(x: Int, y: Int) -> Int = { let sum = x + y; sum + x };";
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let offset = source.rmatch_indices("sum").next().unwrap().0;

    let (_, hover) = doc
        .semantic_hover_at(offset)
        .expect("reference semantic hover should exist");
    assert_eq!(hover, "sum: Int");
}

#[test]
fn test_document_semantic_hover_includes_global_reference_type() {
    let source = "fn id<T>(x: T) -> T = x; let y = id(1);";
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let offset = source.rmatch_indices("id").next().unwrap().0;

    let (_, hover) = doc
        .semantic_hover_at(offset)
        .expect("global reference semantic hover should exist");
    assert_eq!(hover, "id: forall T. (T) -> T");
}

#[test]
fn test_document_semantic_hover_includes_expression_type() {
    let source = "fn add(x: Int, y: Int) -> Int = x + y;";
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let offset = source.find('+').unwrap();

    let (_, hover) = doc
        .semantic_hover_at(offset)
        .expect("expression semantic hover should exist");
    assert_eq!(hover, "Int");
}

#[test]
fn test_document_semantic_hover_includes_method_signature() {
    let source = r#"
        trait Twice { fn twice(self) -> Int; };
        impl Twice for Int {
            fn twice(self) -> Int = self + self;
        };
        let x = 21.twice();
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let offset = source.rmatch_indices("twice").next().unwrap().0;

    let (_, hover) = doc
        .semantic_hover_at(offset)
        .expect("method semantic hover should exist");
    assert_eq!(hover, "fn twice: (Int) -> Int");
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

#[test]
fn test_position_at_utf16() {
    let content = "a😀中";
    let emoji_start = content.find('😀').unwrap();
    let emoji_end = emoji_start + "😀".len();

    let doc = Document::new("file:///test.neve".to_string(), content.to_string());

    assert_eq!(doc.position_at(0), (0, 0));
    assert_eq!(doc.position_at(1), (0, 1)); // after 'a'
    assert_eq!(doc.position_at(emoji_end), (0, 3)); // 'a' + emoji (2 UTF-16 units)
    assert_eq!(doc.position_at(content.len()), (0, 4)); // after '中'
}

#[test]
fn test_offset_at_utf16() {
    let content = "a😀中";
    let emoji_start = content.find('😀').unwrap();
    let emoji_end = emoji_start + "😀".len();

    let doc = Document::new("file:///test.neve".to_string(), content.to_string());

    assert_eq!(doc.offset_at(0, 0), 0);
    assert_eq!(doc.offset_at(0, 1), 1); // after 'a'
    assert_eq!(doc.offset_at(0, 2), emoji_start); // inside emoji -> clamp to start
    assert_eq!(doc.offset_at(0, 3), emoji_end); // after emoji
    assert_eq!(doc.offset_at(0, 4), content.len()); // after '中'
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
fn test_trait_assoc_type_definition() {
    let source = "trait Iterator { type Item; fn next(self) -> Item; };";
    let (ast, _) = parse(source);
    let index = SymbolIndex::from_ast(&ast);

    assert!(index.definitions.contains_key("Item"));
}

#[test]
fn test_impl_assoc_type_definition() {
    let source =
        "trait Iterator { type Item; }; struct Foo {}; impl Iterator for Foo { type Item = Int; };";
    let (ast, _) = parse(source);
    let index = SymbolIndex::from_ast(&ast);

    assert!(index.definitions.contains_key("Item"));
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
