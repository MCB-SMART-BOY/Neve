//! Integration tests for neve-lsp crate.

use neve_diagnostic::ErrorCode;
use neve_lexer::Lexer;
use neve_lsp::{Document, SymbolIndex, generate_semantic_tokens};
use neve_parser::parse;

// Document tests

fn normalize_inference_vars(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        output.push(ch);
        if ch == '?' {
            while matches!(chars.peek(), Some(next) if next.is_ascii_digit()) {
                chars.next();
            }
        }
    }

    // Strip `forall tN. ` added by generic identity inference (B5).
    while let Some(pos) = output.find("forall t") {
        let rest = &output[pos + 7..]; // after "forall "
        // Only strip if followed by `t<digits>. ` (at least "t0. ").
        if rest.len() >= 3
            && rest.as_bytes()[0] == b't'
            && rest.as_bytes()[1].is_ascii_digit()
            && let Some(dot_space) = rest.find(". ")
        {
            output.replace_range(pos..pos + 7 + dot_space + 2, "");
            continue;
        }
        break;
    }

    output
}

fn nth_match_offset(source: &str, needle: &str, index: usize) -> usize {
    source
        .match_indices(needle)
        .nth(index)
        .map(|(offset, _)| offset)
        .unwrap_or_else(|| panic!("expected occurrence {index} of '{needle}' in source"))
}

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
fn test_document_reports_dedicated_missing_method_diagnostic_when_no_fallback_exists() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "let value = 21.missing();".to_string(),
    );
    let diag = doc
        .diagnostics
        .iter()
        .find(|diag| diag.message.contains("no method `missing` found for `Int`"))
        .unwrap_or_else(|| {
            panic!(
                "expected dedicated missing-method diagnostic, got {:?}",
                doc.diagnostics
            )
        });
    assert_eq!(diag.code, Some(ErrorCode::UnknownMethod));
}

#[test]
fn test_document_reports_invalid_try_optional_flow_diagnostic() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "let value = 41?;".to_string(),
    );
    let diag = doc
        .diagnostics
        .iter()
        .find(|diag| {
            diag.message
                .contains("`?` expects Option-like or Result-like value")
        })
        .unwrap_or_else(|| {
            panic!(
                "expected invalid optional-flow diagnostic, got {:?}",
                doc.diagnostics
            )
        });
    assert_eq!(diag.code, Some(ErrorCode::TypeMismatch));
}

#[test]
fn test_document_reports_invalid_coalesce_optional_flow_diagnostic() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "let value = 41 ?? 0;".to_string(),
    );
    let diag = doc
        .diagnostics
        .iter()
        .find(|diag| diag.message.contains("`??` expects Option-like value"))
        .unwrap_or_else(|| {
            panic!(
                "expected invalid coalesce diagnostic, got {:?}",
                doc.diagnostics
            )
        });
    assert_eq!(diag.code, Some(ErrorCode::TypeMismatch));
}

#[test]
fn test_document_reports_invalid_safe_field_boundary_diagnostic() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        r#"let value = 42?.name ?? "default";"#.to_string(),
    );
    let diag = doc
        .diagnostics
        .iter()
        .find(|diag| {
            diag.message
                .contains("safe field access requires a record or Option[Record]")
        })
        .unwrap_or_else(|| {
            panic!(
                "expected invalid safe-field diagnostic, got {:?}",
                doc.diagnostics
            )
        });
    assert_eq!(diag.code, Some(ErrorCode::TypeMismatch));
}

#[test]
fn test_document_reports_invalid_io_read_file_path_diagnostic() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        r#"
            use std.io = io;
            let value = io.readFilePath("/tmp/file.txt");
        "#
        .to_string(),
    );
    let diag = doc
        .diagnostics
        .iter()
        .find(|diag| diag.message.contains("type mismatch"))
        .unwrap_or_else(|| {
            panic!(
                "expected invalid io.readFilePath diagnostic, got {:?}",
                doc.diagnostics
            )
        });
    assert_eq!(diag.code, Some(ErrorCode::TypeMismatch));
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
fn test_document_hover_maps_function_after_top_level_destructuring() {
    let source =
        "let (text, count) = (\"text\", 2); fn foo(x: Int) -> Int = x; let pair = (text, count);";
    assert_top_level_binding_hovers(source, &[("text", "String"), ("count", "Int")]);
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("foo")
        .and_then(|defs| defs.first())
        .expect("function definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "fn foo: (Int) -> Int");
}

fn assert_top_level_binding_hovers(source: &str, bindings: &[(&str, &str)]) {
    let (_, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {diagnostics:?}");
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    for &(name, ty) in bindings {
        let definition = source.find(name).unwrap();
        let reference = source.rfind(name).unwrap();
        assert_ne!(definition, reference, "{name} should also be referenced");
        let symbol = index
            .find_definition_at(reference)
            .expect("reference should resolve to its own pattern binding");
        assert_eq!(usize::from(symbol.def_span.start), definition, "{name}");
        assert_eq!(symbol.def_span.len(), name.len(), "{name}");
        assert_eq!(
            doc.definition_hovers.get(&symbol.def_span),
            Some(&format!("let {name}: {ty}"))
        );
        assert_eq!(
            doc.semantic_hover_at(reference).map(|(_, hover)| hover),
            Some(format!("{name}: {ty}").as_str())
        );
    }
}

#[test]
fn test_document_nested_top_level_destructuring_preserves_binding_hovers() {
    let source = r#"
        let ({ title, payload = (count, enabled) }, [first, ..rest]) =
            ({ title = "hi", payload = (2, true) }, [1, 2]);
        let result = (title, count, enabled, first, rest);
    "#;
    assert_top_level_binding_hovers(
        source,
        &[
            ("title", "String"),
            ("count", "Int"),
            ("enabled", "Bool"),
            ("first", "Int"),
            ("rest", "List[Int]"),
        ],
    );
}

#[test]
fn test_document_binding_pattern_preserves_initializer_local_hovers() {
    let source = r#"
        let whole @ (text, count) = { let prefix = "text"; (prefix, 2) };
        let result = (whole, text, count);
    "#;
    assert_top_level_binding_hovers(
        source,
        &[
            ("whole", "(String, Int)"),
            ("text", "String"),
            ("count", "Int"),
        ],
    );
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let reference = source.rfind("prefix").unwrap();
    let symbol = index.find_definition_at(reference).unwrap();
    assert_eq!(
        doc.definition_hovers
            .get(&symbol.def_span)
            .map(String::as_str),
        Some("prefix: String")
    );
    assert_eq!(
        doc.semantic_hover_at(reference).map(|(_, hover)| hover),
        Some("prefix: String")
    );
}

#[test]
fn test_document_constructor_or_pattern_preserves_each_binding_hover_span() {
    let source = r#"
        enum Choice { Pick(Int), Other(Int) };
        let Pick(value) | Other(value) = Pick(1);
        let result = value;
    "#;
    let (_, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {diagnostics:?}");
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let definitions: Vec<_> = source
        .match_indices("value)")
        .map(|(offset, _)| offset)
        .collect();
    assert_eq!(definitions.len(), 2);
    for offset in definitions {
        let hover = doc
            .definition_hovers
            .iter()
            .find(|(span, _)| usize::from(span.start) == offset)
            .expect("each or-pattern binding span should have a hover");
        assert_eq!(hover.0.len(), "value".len());
        assert_eq!(hover.1, "let value: Int");
    }
    assert_eq!(
        doc.semantic_hover_at(source.rfind("value").unwrap())
            .map(|(_, hover)| hover),
        Some("value: Int")
    );
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
fn test_document_hover_includes_destructured_function_parameters() {
    let source = "fn sum_pair((x, y): (Int, Int)) -> Int = x + y;";
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");

    for name in ["x", "y"] {
        let symbol = index
            .get_definitions(name)
            .and_then(|defs| defs.first())
            .expect("tuple parameter definition should be indexed");
        let hover = doc
            .definition_hovers
            .get(&symbol.def_span)
            .expect("tuple parameter hover should exist");
        assert_eq!(hover, &format!("{name}: Int"));
    }
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
fn test_document_hover_uses_canonical_assoc_return_for_method_call_binding() {
    let source = r#"
        trait Iterator { type Item; fn first(self) -> Self.Item; };
        impl Iterator for Int {
            type Item = String;
            fn first(self) -> Self.Item = toString(self);
        };
        let value = 1.first();
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_trait_dispatch_precedence_over_callable_target_fallback() {
    let source = r#"
        fn twice(x: Int) -> String = "fallback";
        trait Twice { fn twice(self) -> Int; };
        impl Twice for Int {
            fn twice(self) -> Int = self + self;
        };
        let value = 21.twice();
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Int");
}

#[test]
fn test_document_hover_uses_callable_target_fallback_when_no_method_exists() {
    let source = r#"
        fn twice(x: Int) -> String = "fallback";
        let value = 21.twice();
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_typed_path_adapter_binding_type() {
    let source = r#"
        use std.path = path;
        let nested = path.joinPath(path.fromString("/tmp"), "neve.txt");
        let value = path.extensionPath(nested);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Option[String]");
}

#[test]
fn test_document_hover_uses_std_list_sort_binding_type() {
    let source = r#"
        use std.list = list;
        use std.io = io;
        use std.path = path;
        let value = list.sort(io.readDirEntryPaths(path.fromString("/tmp")));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: List[Path]");
}

#[test]
fn test_document_hover_uses_std_list_max_binding_type() {
    let source = r#"
        use std.list = list;
        let value = list.max([1, 3, 2]);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Option[Int]");
}
#[test]
fn test_document_hover_traverses_std_method_arguments() {
    let source = "use std.list = list; let value = list.map(|x: Int| x, [1]);";
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("x")
        .and_then(|defs| defs.first())
        .expect("lambda parameter should be indexed");

    assert_eq!(
        doc.definition_hovers
            .get(&symbol.def_span)
            .map(String::as_str),
        Some("x: Int")
    );
}

#[test]
fn test_document_hover_uses_std_list_head_binding_type() {
    let source = r#"
        use std.list = list;
        use std.io = io;
        use std.path = path;
        let value = list.head(io.readDirEntryPaths(path.fromString("/tmp")));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Option[Path]");
}

#[test]
fn test_document_hover_uses_std_list_reverse_binding_type() {
    let source = r#"
        use std.list = list;
        use std.io = io;
        use std.path = path;
        let value = list.reverse(io.readDirEntryPaths(path.fromString("/tmp")));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: List[Path]");
}

#[test]
fn test_document_hover_uses_std_list_get_binding_type() {
    let source = r#"
        use std.list = list;
        use std.io = io;
        use std.path = path;
        let value = list.get(0, io.readDirEntryPaths(path.fromString("/tmp")));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Option[Path]");
}

#[test]
fn test_document_hover_uses_std_list_cons_binding_type() {
    let source = r#"
        use std.list = list;
        use std.io = io;
        use std.path = path;
        let value = list.cons(path.fromString("/"), io.readDirEntryPaths(path.fromString("/tmp")));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: List[Path]");
}

#[test]
fn test_document_hover_uses_std_list_take_binding_type() {
    let source = r#"
        use std.list = list;
        use std.io = io;
        use std.path = path;
        let value = list.take(2, io.readDirEntryPaths(path.fromString("/tmp")));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: List[Path]");
}

#[test]
fn test_document_hover_uses_std_list_drop_binding_type() {
    let source = r#"
        use std.list = list;
        use std.io = io;
        use std.path = path;
        let value = list.drop(1, io.readDirEntryPaths(path.fromString("/tmp")));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: List[Path]");
}

#[test]
fn test_document_hover_uses_std_list_contains_binding_type() {
    let source = r#"
        use std.list = list;
        use std.io = io;
        use std.path = path;
        let value = list.contains(path.fromString("/"), io.readDirEntryPaths(path.fromString("/tmp")));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Bool");
}

#[test]
fn test_document_hover_uses_std_list_index_of_binding_type() {
    let source = r#"
        use std.list = list;
        use std.io = io;
        use std.path = path;
        let value = list.indexOf(path.fromString("/"), io.readDirEntryPaths(path.fromString("/tmp")));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Option[Int]");
}

#[test]
fn test_document_hover_uses_std_list_sum_binding_type() {
    let source = r#"
        use std.list = list;
        let value = list.sum([1, 2, 3]);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Int");
}

#[test]
fn test_document_hover_uses_std_list_product_binding_type() {
    let source = r#"
        use std.list = list;
        let value = list.product([2, 3, 4]);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Int");
}

#[test]
fn test_document_hover_uses_std_list_replicate_binding_type() {
    let source = r#"
        use std.list = list;
        use std.path = path;
        let value = list.replicate(2, path.fromString("/tmp"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: List[Path]");
}

#[test]
fn test_document_hover_uses_std_list_zip_binding_type() {
    let source = r#"
        use std.list = list;
        use std.io = io;
        use std.path = path;
        let value = list.zip(io.readDirEntryPaths(path.fromString("/tmp")), [1, 2]);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: List[(Path, Int)]");
}

#[test]
fn test_document_hover_uses_std_list_unzip_binding_type() {
    let source = r#"
        use std.list = list;
        use std.path = path;
        let value = list.unzip([
            (path.fromString("/tmp"), 1),
            (path.fromString("/var"), 2),
        ]);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: (List[Path], List[Int])");
}

#[test]
fn test_document_hover_uses_std_list_fold_right_binding_type() {
    let source = r#"
        use std.list = list;
        fn step(x, acc) = x + acc;
        let value = list.foldRight(0, step, [1, 2, 3]);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Int");
}

#[test]
fn test_document_hover_uses_std_math_constant_binding_type() {
    let source = r#"
        use std.math = math;
        let value = math.inf;
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Float");
}

#[test]
fn test_document_hover_uses_std_math_conversion_binding_types() {
    let source = r#"
        use std.math = math;
        let count = math.toInt(true);
        let ratio = math.toFloat("1.5");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");

    let count = index
        .get_definitions("count")
        .and_then(|defs| defs.first())
        .expect("count definition should be indexed");
    let ratio = index
        .get_definitions("ratio")
        .and_then(|defs| defs.first())
        .expect("ratio definition should be indexed");

    let count_hover = doc
        .definition_hovers
        .get(&count.def_span)
        .expect("count hover should exist");
    let ratio_hover = doc
        .definition_hovers
        .get(&ratio.def_span)
        .expect("ratio hover should exist");

    assert_eq!(count_hover, "let count: Int");
    assert_eq!(ratio_hover, "let ratio: Float");
}

#[test]
fn test_document_hover_uses_std_math_float_predicate_binding_types() {
    let source = r#"
        use std.math = math;
        let a = math.isNan(math.nan);
        let b = math.isInf(math.inf);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");

    let a = index
        .get_definitions("a")
        .and_then(|defs| defs.first())
        .expect("a definition should be indexed");
    let b = index
        .get_definitions("b")
        .and_then(|defs| defs.first())
        .expect("b definition should be indexed");

    let a_hover = doc
        .definition_hovers
        .get(&a.def_span)
        .expect("a hover should exist");
    let b_hover = doc
        .definition_hovers
        .get(&b.def_span)
        .expect("b hover should exist");

    assert_eq!(a_hover, "let a: Bool");
    assert_eq!(b_hover, "let b: Bool");
}

#[test]
fn test_document_hover_uses_std_math_rounding_binding_types() {
    let source = r#"
        use std.math = math;
        let a = math.floor(1.9);
        let b = math.ceil(1.1);
        let c = math.round(1.6);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");

    let a = index
        .get_definitions("a")
        .and_then(|defs| defs.first())
        .expect("a definition should be indexed");
    let b = index
        .get_definitions("b")
        .and_then(|defs| defs.first())
        .expect("b definition should be indexed");
    let c = index
        .get_definitions("c")
        .and_then(|defs| defs.first())
        .expect("c definition should be indexed");

    let a_hover = doc
        .definition_hovers
        .get(&a.def_span)
        .expect("a hover should exist");
    let b_hover = doc
        .definition_hovers
        .get(&b.def_span)
        .expect("b hover should exist");
    let c_hover = doc
        .definition_hovers
        .get(&c.def_span)
        .expect("c hover should exist");

    assert_eq!(a_hover, "let a: Int");
    assert_eq!(b_hover, "let b: Int");
    assert_eq!(c_hover, "let c: Int");
}

#[test]
fn test_document_hover_uses_std_math_unary_float_transform_binding_types() {
    let source = r#"
        use std.math = math;
        let a = math.sqrt(9.0);
        let b = math.log(1.0);
        let c = math.log10(1000.0);
        let d = math.exp(0.0);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");

    let a = index
        .get_definitions("a")
        .and_then(|defs| defs.first())
        .expect("a definition should be indexed");
    let b = index
        .get_definitions("b")
        .and_then(|defs| defs.first())
        .expect("b definition should be indexed");
    let c = index
        .get_definitions("c")
        .and_then(|defs| defs.first())
        .expect("c definition should be indexed");
    let d = index
        .get_definitions("d")
        .and_then(|defs| defs.first())
        .expect("d definition should be indexed");

    let a_hover = doc
        .definition_hovers
        .get(&a.def_span)
        .expect("a hover should exist");
    let b_hover = doc
        .definition_hovers
        .get(&b.def_span)
        .expect("b hover should exist");
    let c_hover = doc
        .definition_hovers
        .get(&c.def_span)
        .expect("c hover should exist");
    let d_hover = doc
        .definition_hovers
        .get(&d.def_span)
        .expect("d hover should exist");

    assert_eq!(a_hover, "let a: Float");
    assert_eq!(b_hover, "let b: Float");
    assert_eq!(c_hover, "let c: Float");
    assert_eq!(d_hover, "let d: Float");
}

#[test]
fn test_document_hover_uses_std_math_trigonometric_binding_types() {
    let source = r#"
        use std.math = math;
        let a = math.sin(0.0);
        let b = math.cos(0.0);
        let c = math.tan(0.0);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");

    let a = index
        .get_definitions("a")
        .and_then(|defs| defs.first())
        .expect("a definition should be indexed");
    let b = index
        .get_definitions("b")
        .and_then(|defs| defs.first())
        .expect("b definition should be indexed");
    let c = index
        .get_definitions("c")
        .and_then(|defs| defs.first())
        .expect("c definition should be indexed");

    let a_hover = doc
        .definition_hovers
        .get(&a.def_span)
        .expect("a hover should exist");
    let b_hover = doc
        .definition_hovers
        .get(&b.def_span)
        .expect("b hover should exist");
    let c_hover = doc
        .definition_hovers
        .get(&c.def_span)
        .expect("c hover should exist");

    assert_eq!(a_hover, "let a: Float");
    assert_eq!(b_hover, "let b: Float");
    assert_eq!(c_hover, "let c: Float");
}

#[test]
fn test_document_hover_keeps_std_math_function_as_inference_hole() {
    let source = r#"
        use std.math = math;
        let value = math.abs(1);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(normalize_inference_vars(hover), "let value: ?");
}

#[test]
fn test_document_hover_uses_io_read_file_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.readFilePath(path.fromString("/tmp/file.txt"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_io_read_file_bytes_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.readFileBytesPath(path.fromString("/tmp/file.bin"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Bytes");
}

#[test]
fn test_document_hover_uses_io_read_dir_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.readDirPath(path.fromString("/tmp"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: List[String]");
}

#[test]
fn test_document_hover_uses_io_read_dir_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.readDir("/tmp");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: List[String]");
}

#[test]
fn test_document_hover_uses_io_read_dir_entry_paths_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.readDirEntryPaths(path.fromString("/tmp"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: List[Path]");
}

#[test]
fn test_document_hover_uses_io_write_file_bytes_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let bytes = io.readFileBytesPath(path.fromString("/tmp/file.bin"));
        let value = io.writeFileBytesPath(path.fromString("/tmp/file.out"), bytes);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ()");
}

#[test]
fn test_document_hover_uses_io_write_file_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.writeFilePath(path.fromString("/tmp/file.out"), "hello");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ()");
}

#[test]
fn test_document_hover_uses_io_write_file_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.writeFile("/tmp/file.out", "hello");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ()");
}

#[test]
fn test_document_hover_uses_io_append_file_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.appendFilePath(path.fromString("/tmp/file.out"), "hello");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ()");
}

#[test]
fn test_document_hover_uses_io_append_file_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.appendFile("/tmp/file.out", "hello");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ()");
}

#[test]
fn test_document_hover_uses_io_append_file_bytes_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let bytes = io.readFileBytesPath(path.fromString("/tmp/file.bin"));
        let value = io.appendFileBytesPath(path.fromString("/tmp/file.out"), bytes);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ()");
}

#[test]
fn test_document_hover_uses_io_current_dir_path_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.currentDirPath();
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Path");
}

#[test]
fn test_document_hover_uses_io_current_dir_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.currentDir();
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_io_get_env_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.getEnv("HOME");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Option[String]");
}

#[test]
fn test_document_hover_uses_io_home_dir_path_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.homeDirPath();
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Option[Path]");
}

#[test]
fn test_document_hover_uses_io_home_dir_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.homeDir();
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Option[String]");
}

#[test]
fn test_document_hover_uses_io_current_system_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.currentSystem();
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_io_create_dir_all_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.createDirAll("/tmp/neve-dir");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ()");
}

#[test]
fn test_document_hover_uses_io_create_dir_all_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.createDirAllPath(path.fromString("/tmp/neve-dir"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ()");
}

#[test]
fn test_document_hover_uses_io_remove_dir_all_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.removeDirAllPath(path.fromString("/tmp/neve-dir"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ()");
}

#[test]
fn test_document_hover_uses_io_remove_dir_all_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.removeDirAll("/tmp/neve-dir");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ()");
}

#[test]
fn test_document_hover_uses_io_path_exists_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.pathExists("/tmp/file.txt");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Bool");
}

#[test]
fn test_document_hover_uses_io_is_dir_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.isDir("/tmp");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Bool");
}

#[test]
fn test_document_hover_uses_io_is_file_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.isFile("/tmp/file.txt");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Bool");
}

#[test]
fn test_document_hover_uses_io_hash_file_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.hashFile("/tmp/file.txt");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_io_read_file_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.readFile("/tmp/file.txt");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_io_hash_file_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.hashFilePath(path.fromString("/tmp/file.txt"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_io_command_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.command("printf", ["neve"]);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Command");
}

#[test]
fn test_document_hover_uses_io_command_with_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.commandWith(#{ program = "printf", args = ["neve"], cwd = "/tmp" });
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Command");
}

#[test]
fn test_document_hover_uses_io_exec_command_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.execCommand(io.command("rustc", ["--version"]));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ProcessResult");
}

#[test]
fn test_document_hover_uses_io_pipeline_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])]);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Pipeline");
}

#[test]
fn test_document_hover_uses_io_pipeline_with_redirects_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.pipelineWithRedirects(
            io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])]),
            [io.redirectStdoutPath(path.fromString("/tmp/neve.out"))]
        );
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Pipeline");
}

#[test]
fn test_document_hover_uses_io_exec_pipeline_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.execPipeline(
            io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])])
        );
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ProcessResult");
}

#[test]
fn test_document_hover_uses_io_exec_pipeline_with_redirect_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.execPipeline(
            io.pipelineWithRedirects(
                io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])]),
                [io.redirectStdoutPath(path.fromString("/tmp/neve.out"))]
            )
        );
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ProcessResult");
}

#[test]
fn test_document_hover_uses_io_exec_pipeline_with_redirects_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.execPipeline(
            io.pipelineWithRedirects(
                io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])]),
                [
                    io.redirectStdoutPath(path.fromString("/tmp/neve.out")),
                    io.redirectStderrPath(path.fromString("/tmp/neve.err"))
                ]
            )
        );
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ProcessResult");
}

#[test]
fn test_document_hover_uses_io_command_with_redirects_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.commandWithRedirects(
            io.command("printf", ["neve"]),
            [io.redirectStdoutPath(path.fromString("/tmp/neve.out"))]
        );
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Command");
}

#[test]
fn test_document_hover_uses_io_redirect_stdout_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.redirectStdoutPath(path.fromString("/tmp/neve.out"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Redirect");
}

#[test]
fn test_document_hover_uses_io_redirect_stderr_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.redirectStderrPath(path.fromString("/tmp/neve.err"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Redirect");
}

#[test]
fn test_document_hover_uses_io_redirect_stdin_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.redirectStdinPath(path.fromString("/tmp/neve.in"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Redirect");
}

#[test]
fn test_document_hover_uses_io_exec_command_with_redirect_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.execCommand(
            io.commandWithRedirects(
                io.command("printf", ["neve"]),
                [io.redirectStdoutPath(path.fromString("/tmp/neve.out"))]
            )
        );
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ProcessResult");
}

#[test]
fn test_document_hover_uses_io_exec_command_with_redirects_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.execCommand(
            io.commandWithRedirects(
                io.command("printf", ["neve"]),
                [
                    io.redirectStdoutPath(path.fromString("/tmp/neve.out")),
                    io.redirectStderrPath(path.fromString("/tmp/neve.err"))
                ]
            )
        );
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ProcessResult");
}

#[test]
fn test_document_hover_uses_io_task_command_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.taskCommand(io.command("printf", ["neve"]));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Task[ProcessResult]");
}

#[test]
fn test_document_hover_uses_io_task_pipeline_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.taskPipeline(io.pipeline([
            io.command("printf", ["neve"]),
            io.command("cat", [])
        ]));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Task[ProcessResult]");
}

#[test]
fn test_document_hover_uses_io_await_task_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.awaitTask(io.taskCommand(io.command("rustc", ["--version"])));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ProcessResult");
}

#[test]
fn test_document_hover_uses_io_await_tasks_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.awaitTasks([
            io.taskCommand(io.command("printf", ["neve"])),
            io.taskPipeline(io.pipeline([
                io.command("printf", ["lang"]),
                io.command("cat", [])
            ]))
        ]);
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: List[ProcessResult]");
}

#[test]
fn test_document_hover_uses_io_exec_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.execCommand(io.command("rustc", ["--version"]));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ProcessResult");
}

#[test]
fn test_document_hover_uses_explicit_shell_exec_command_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.execCommand(io.command("sh", ["-c", "rustc --version"]));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ProcessResult");
}

#[test]
fn test_document_hover_uses_io_exec_with_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.execCommand(io.commandWith(#{ program = "rustc", args = ["--version"] }));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: ProcessResult");
}

#[test]
fn test_document_hover_uses_io_process_success_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.processSuccess(io.execCommand(io.command("rustc", ["--version"])));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Bool");
}

#[test]
fn test_document_hover_uses_io_process_stdout_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.processStdout(io.execCommand(io.command("rustc", ["--version"])));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_io_process_code_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.processCode(io.execCommand(io.command("rustc", ["--version"])));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Int");
}

#[test]
fn test_document_hover_uses_io_process_stderr_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.processStderr(io.execCommand(io.command("rustc", ["--version"])));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_io_hash_string_binding_type() {
    let source = r#"
        use std.io = io;
        let value = io.hashString("abc");
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_io_path_exists_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.pathExistsPath(path.fromString("/tmp/file.txt"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Bool");
}

#[test]
fn test_document_hover_uses_io_is_dir_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.isDirPath(path.fromString("/tmp"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Bool");
}

#[test]
fn test_document_hover_uses_io_is_file_path_binding_type() {
    let source = r#"
        use std.io = io;
        use std.path = path;
        let value = io.isFilePath(path.fromString("/tmp/file.txt"));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Bool");
}

#[test]
fn test_document_hover_uses_fetch_binding_type() {
    let source = r#"
        use std.fetch = fetch;
        let value = fetch.path("Cargo.toml").hash;
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_fetch_path_with_hash_binding_type() {
    let source = r#"
        use std.fetch = fetch;
        let value = fetch.pathWithHash(
            "Cargo.toml",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ).hash;
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_fetch_url_binding_type() {
    let source = r#"
        use std.fetch = fetch;
        let value = fetch.url("https://example.com/archive.tar.gz").hash;
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_fetch_url_with_hash_binding_type() {
    let source = r#"
        use std.fetch = fetch;
        let value = fetch.urlWithHash(
            "https://example.com/archive.tar.gz",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ).hash;
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_fetch_git_binding_type() {
    let source = r#"
        use std.fetch = fetch;
        let value = fetch.git("/tmp/repo", "main").hash;
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_fetch_git_with_hash_binding_type() {
    let source = r#"
        use std.fetch = fetch;
        let value = fetch.gitWithHash(
            "/tmp/repo",
            "main",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ).hash;
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_map_binding_type() {
    let source = r#"
        use std.Map;
        let value = Map.values(Map.insert("a", 1, Map.empty));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: List[Int]");
}

#[test]
fn test_document_hover_uses_set_binding_type() {
    let source = r#"
        use std.Set;
        let value = Set.isDisjoint(Set.fromList([1]), Set.fromList([2]));
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Bool");
}

#[test]
fn test_document_hover_uses_optional_flow_result_for_try_binding() {
    let source = r#"
        use std.option = option;
        let value = option.some(41)? + 1;
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    assert!(
        doc.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        doc.diagnostics
    );
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Int");
}

#[test]
fn test_document_hover_uses_optional_flow_result_for_coalesce_binding() {
    let source = r#"
        use std.option = option;
        let value = option.none ?? 5;
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    assert!(
        doc.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        doc.diagnostics
    );
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Int");
}

#[test]
fn test_document_hover_uses_optional_flow_result_for_safe_field_coalesce_binding() {
    let source = r#"
        use std.option = option;
        let value = option.some(#{ name = "test" })?.name ?? "default";
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    assert!(
        doc.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        doc.diagnostics
    );
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_optional_flow_result_for_builtin_result_try_binding() {
    let source = r#"
        use std.result = result;
        let value = result.ok(41)? + 1;
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    assert!(
        doc.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        doc.diagnostics
    );
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Int");
}

#[test]
fn test_document_hover_uses_optional_flow_result_for_enum_some_try_binding() {
    let source = r#"
        enum Option { Some(Int), None };
        let value = Some(41)? + 1;
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    assert!(
        doc.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        doc.diagnostics
    );
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: Int");
}

#[test]
fn test_document_hover_uses_optional_flow_result_for_record_safe_field_binding() {
    let source = r#"
        let value = #{ name = "test" }?.name ?? "default";
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    assert!(
        doc.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        doc.diagnostics
    );
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_hover_uses_canonical_default_assoc_alias_return_for_method_call_binding() {
    let source = r#"
        trait Iterator {
            type Alias;
            type Item = Self.Alias;
            fn first(self) -> Self.Item;
        };
        impl Iterator for Int {
            type Alias = String;
            fn first(self) -> Self.Item = toString(self);
        };
        let value = 1.first();
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("value")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(hover, "let value: String");
}

#[test]
fn test_document_semantic_hover_uses_canonical_assoc_projection_for_impl_param_type() {
    let source = r#"
        trait Iterator {
            type Item;
            fn first(self, fallback: Self.Item) -> Self.Item;
        };
        impl Iterator for Int {
            type Item = String;
            fn first(self, fallback: Self.Item) -> Self.Item = fallback;
        };
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let offset = nth_match_offset(source, "Self.Item", 2);

    let (_, hover) = doc
        .semantic_hover_at(offset)
        .expect("impl param type hover should exist");
    assert_eq!(hover, "String");
}

#[test]
fn test_document_semantic_hover_uses_canonical_assoc_projection_for_impl_return_type() {
    let source = r#"
        trait Iterator {
            type Item;
            fn first(self, fallback: Self.Item) -> Self.Item;
        };
        impl Iterator for Int {
            type Item = String;
            fn first(self, fallback: Self.Item) -> Self.Item = fallback;
        };
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let offset = nth_match_offset(source, "Self.Item", 3);

    let (_, hover) = doc
        .semantic_hover_at(offset)
        .expect("impl return type hover should exist");
    assert_eq!(hover, "String");
}

#[test]
fn test_document_semantic_hover_preserves_trait_self_assoc_source_shape() {
    let source = r#"
        trait Iterator {
            type Item;
            fn first(self, fallback: Self.Item) -> Self.Item;
        };
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let offset = nth_match_offset(source, "Self.Item", 0);

    let (_, hover) = doc
        .semantic_hover_at(offset)
        .expect("trait param type hover should exist");
    assert_eq!(hover, "Self.Item");
}

#[test]
fn test_document_semantic_hover_keeps_trait_self_assoc_source_shape_when_impl_is_present() {
    let source = r#"
        trait Iterator {
            type Item;
            fn first(self, fallback: Self.Item) -> Self.Item;
        };
        impl Iterator for Int {
            type Item = String;
            fn first(self, fallback: Self.Item) -> Self.Item = fallback;
        };
    "#;
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());

    let trait_offset = nth_match_offset(source, "Self.Item", 0);
    let impl_offset = nth_match_offset(source, "Self.Item", 2);

    let (_, trait_hover) = doc
        .semantic_hover_at(trait_offset)
        .expect("trait param type hover should exist");
    let (_, impl_hover) = doc
        .semantic_hover_at(impl_offset)
        .expect("impl param type hover should exist");

    assert_eq!(trait_hover, "Self.Item");
    assert_eq!(impl_hover, "String");
}

#[test]
fn test_document_hover_formats_dynamic_record_shape_readably() {
    let source = "let outputs = fn(inputs) inputs.dep.packages.default;";
    let doc = Document::new("file:///test.neve".to_string(), source.to_string());
    let index = doc
        .symbol_index
        .as_ref()
        .expect("symbol index should exist");
    let symbol = index
        .get_definitions("outputs")
        .and_then(|defs| defs.first())
        .expect("let definition should be indexed");
    let hover = doc
        .definition_hovers
        .get(&symbol.def_span)
        .expect("semantic hover should exist");

    assert_eq!(
        normalize_inference_vars(hover),
        "let outputs: ({ dep: { packages: { default: ?, .. }, .. }, .. }) -> ?"
    );
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
fn test_semantic_tokens_positions_use_utf16_code_units() {
    // LSP positions count UTF-16 code units. An astral character (emoji) occupies
    // two units, so counting characters (or bytes) shifts every later token on the
    // line. `let icon = "` is 11 units, the string literal holds 4 units
    // (quote + surrogate pair + quote), so `tag` starts at unit 21 while
    // character counting would report 20 and byte counting 23.
    // LSP 位置以 UTF-16 码元计数：BMP 之外的字符（emoji）占两个码元，按字符或
    // 字节计数都会使该行后续 token 的位置偏移。
    let source = "let icon = \"😀\"; let tag = icon;\n";
    let tokens = neve_lsp::generate_semantic_tokens_from_ast(source);

    let mut decoded = Vec::new();
    let mut line = 0u32;
    let mut column = 0u32;
    for token in &tokens {
        line += token.delta_line;
        column = if token.delta_line == 0 {
            column + token.delta_start
        } else {
            token.delta_start
        };
        decoded.push((line, column, token.length));
    }

    assert_eq!(
        decoded,
        vec![(0, 4, 4), (0, 21, 3), (0, 27, 4)],
        "semantic token positions must be UTF-16 based"
    );
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

fn collect_ast_semantic_tokens(source: &str) -> Vec<(&str, u32, u32)> {
    let (_, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {diagnostics:?}");
    let lines: Vec<_> = source.lines().collect();
    let mut positions = std::collections::BTreeSet::new();
    let mut line = 0;
    let mut column = 0;
    neve_lsp::generate_semantic_tokens_from_ast(source)
        .into_iter()
        .map(|token| {
            line += token.delta_line as usize;
            column = if token.delta_line == 0 {
                column + token.delta_start as usize
            } else {
                token.delta_start as usize
            };
            assert!(positions.insert((line, column)), "duplicate token position");
            (
                &lines[line][column..column + token.length as usize],
                token.token_type,
                token.token_modifiers_bitset,
            )
        })
        .collect()
}

#[test]
fn test_semantic_tokens_nested_pattern_bindings_are_declarations() {
    use neve_lsp::{token_modifiers, token_types};

    let source = r#"
        let whole @ ({ short, renamed = (left, [element]) }, [head, ..rest, tail]) = input;
        fn local() = { let (block_first, { value = block_second }) = input; 0 };
        fn inspect(value) = match value {
            Some((inner, [first, ..middle, last])) -> 0,
            None -> 0
        };
        fn alternatives(value) = match value {
            Some(choice) | Other(choice) -> 0,
            _ -> 0
        };
    "#;
    let tokens = collect_ast_semantic_tokens(source);
    let declarations: Vec<_> = tokens
        .iter()
        .filter(|(_, kind, modifiers)| {
            *kind == token_types::VARIABLE
                && *modifiers == (token_modifiers::DECLARATION | token_modifiers::READONLY)
        })
        .map(|(name, _, _)| *name)
        .collect();
    assert_eq!(
        declarations,
        [
            "whole",
            "short",
            "left",
            "element",
            "head",
            "rest",
            "tail",
            "block_first",
            "block_second",
            "inner",
            "first",
            "middle",
            "last",
            "choice",
            "choice",
        ]
    );
}

#[test]
fn test_semantic_tokens_nested_parameters_preserve_parameter_classification() {
    use neve_lsp::token_types;

    let source = r#"
        fn unpack((fn_first, { field = [fn_second] })) = 0;
        impl Show for Int {
            fn show((impl_first, { impl_second })) = 0;
        };
        let closure = fn((lambda_first, { field = [lambda_second, ..lambda_rest] })) 0;
    "#;
    let tokens = collect_ast_semantic_tokens(source);
    let parameters: Vec<_> = tokens
        .iter()
        .filter(|(_, kind, _)| *kind == token_types::PARAMETER)
        .map(|(name, _, modifiers)| (*name, *modifiers))
        .collect();
    assert_eq!(
        parameters,
        [
            ("fn_first", 0),
            ("fn_second", 0),
            ("impl_first", 0),
            ("impl_second", 0),
            ("lambda_first", 0),
            ("lambda_second", 0),
            ("lambda_rest", 0),
        ]
    );
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

#[test]
fn test_symbol_index_resolves_shadowed_local_definition() {
    let source = "fn outer(x) = { let x = 2; x + x } + x;";
    let (ast, _) = parse(source);
    let index = SymbolIndex::from_ast(&ast);

    let inner_def_offset = source.find("let x = 2").unwrap() + 4;
    let inner_use_offset = source.find("x + x }").unwrap();
    let outer_use_offset = source.rfind("x;").unwrap();
    let param_def_offset = source.find("(x)").unwrap() + 1;

    let inner_symbol = index
        .find_definition_at(inner_use_offset)
        .expect("inner reference should resolve");
    let outer_symbol = index
        .find_definition_at(outer_use_offset)
        .expect("outer reference should resolve");

    assert_eq!(usize::from(inner_symbol.def_span.start), inner_def_offset);
    assert_eq!(usize::from(outer_symbol.def_span.start), param_def_offset);
}

#[test]
fn test_symbol_index_references_respect_shadowing() {
    let source = "fn outer(x) = { let x = 2; x + x } + x;";
    let (ast, _) = parse(source);
    let index = SymbolIndex::from_ast(&ast);

    let inner_use_offset = source.find("x + x }").unwrap();
    let outer_use_offset = source.rfind("x;").unwrap();

    let inner_refs = index.find_references_at(inner_use_offset, true);
    let outer_refs = index.find_references_at(outer_use_offset, true);

    assert_eq!(inner_refs.len(), 3);
    assert_eq!(outer_refs.len(), 2);
}

#[test]
fn test_symbol_index_constructor_patterns_resolve_variants_in_either_order() {
    for source in [
        "enum Choice { Pick(Int), Empty }; fn unwrap(v) = match v { Pick(x) -> x, Empty -> 0 };",
        "fn unwrap(v) = match v { Pick(x) -> x, Empty -> 0 }; enum Choice { Pick(Int), Empty };",
    ] {
        let (ast, diagnostics) = parse(source);
        assert!(diagnostics.is_empty(), "parse errors: {diagnostics:?}");
        let index = SymbolIndex::from_ast(&ast);
        for (name, pattern) in [("Pick", "Pick(x)"), ("Empty", "Empty ->")] {
            let offset = source.find(pattern).unwrap();
            let declaration = index.get_definitions(name).unwrap().first().unwrap();
            let reference = index
                .references
                .iter()
                .find(|reference| usize::from(reference.span.start) == offset)
                .expect("constructor pattern reference should be indexed");
            assert!(!reference.is_write);
            assert_eq!(reference.target_def_span, Some(declaration.def_span));
            assert_eq!(
                index
                    .find_definition_at(offset)
                    .map(|symbol| symbol.def_span),
                Some(declaration.def_span)
            );
            let references =
                index.find_references_at(usize::from(declaration.def_span.start), false);
            assert_eq!(references.len(), 1);
            assert_eq!(references[0].span, reference.span);
        }
    }
}
#[test]
fn test_symbol_index_or_pattern_bindings_share_definition_identity() {
    let source = "enum Choice { Pick(Int), Other(Int) }; fn unwrap(v) = match v { Pick(x) | Other(x) -> x };";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {diagnostics:?}");
    let index = SymbolIndex::from_ast(&ast);
    let definitions = index
        .get_definitions("x")
        .expect("or-pattern binding should be indexed");
    assert_eq!(definitions.len(), 1);

    let first_binding = source.find("Pick(x)").unwrap() + "Pick(".len();
    let second_binding = source.find("Other(x)").unwrap() + "Other(".len();
    let body_reference = source.rfind("-> x").unwrap() + "-> ".len();
    let definition_span = definitions[0].def_span;

    for offset in [first_binding, second_binding, body_reference] {
        assert_eq!(
            index
                .find_definition_at(offset)
                .map(|symbol| symbol.def_span),
            Some(definition_span)
        );
        assert_eq!(
            index
                .find_references_at(offset, true)
                .iter()
                .filter(|reference| reference.target_def_span == Some(definition_span))
                .count(),
            3
        );
    }
}

#[test]
fn test_symbol_index_constructor_pattern_ignores_local_value_shadowing() {
    let source = "enum Choice { Pick(Int), Empty }; fn unwrap(v) = { let Pick @ _ = 0; match v { Pick(x) -> x, Empty -> 0 } };";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {diagnostics:?}");
    let index = SymbolIndex::from_ast(&ast);
    let offset = source.find("Pick(x)").unwrap();
    let symbol = index.find_definition_at(offset).unwrap();
    assert_eq!(symbol.kind, neve_lsp::SymbolKind::Variant);
    assert_eq!(
        usize::from(symbol.def_span.start),
        source.find("Pick(Int)").unwrap()
    );
}

#[test]
fn test_symbol_index_unknown_constructor_does_not_resolve_value_names() {
    for source in [
        "fn Missing(v) = v; fn unpack(v) = match v { Missing(x) -> x, _ -> 0 };",
        "fn unpack(v) = { let Missing @ _ = 0; match v { Missing(x) -> x, _ -> 0 } };",
        "fn unpack(v) = match v { Missing(x) -> x, _ -> 0 };",
    ] {
        let (ast, diagnostics) = parse(source);
        assert!(diagnostics.is_empty(), "parse errors: {diagnostics:?}");
        let index = SymbolIndex::from_ast(&ast);
        let offset = source.find("Missing(x)").unwrap();
        let reference = index
            .references
            .iter()
            .find(|reference| usize::from(reference.span.start) == offset)
            .expect("unresolved constructor reference should be indexed");
        assert!(!reference.is_write);
        assert_eq!(reference.target_def_span, None);
        assert!(index.find_definition_at(offset).is_none());
        assert!(index.find_references_at(offset, true).is_empty());
    }
}

#[test]
fn test_symbol_index_builtin_constructor_patterns_have_no_source_definition() {
    let source = "fn unwrap(v) = match v { Some(x) -> x, None -> 0 };";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {diagnostics:?}");
    let index = SymbolIndex::from_ast(&ast);
    for name in ["Some", "None"] {
        let offset = source.find(name).unwrap();
        let reference = index
            .references
            .iter()
            .find(|reference| usize::from(reference.span.start) == offset)
            .expect("builtin constructor reference should be indexed");
        assert!(!reference.is_write);
        assert_eq!(reference.target_def_span, None);
        assert!(index.find_definition_at(offset).is_none());
        assert!(index.find_references_at(offset, true).is_empty());
    }
}

// =============================================================================
// Handler tests: semantic tokens, folding, imports, highlight
// =============================================================================

#[test]
fn test_semantic_tokens_enum_variants() {
    let tokens = neve_lsp::generate_semantic_tokens_from_ast("enum Color { Red, Green, Blue };\n");
    assert!(!tokens.is_empty());
}

#[test]
fn test_semantic_tokens_struct_fields() {
    let tokens = neve_lsp::generate_semantic_tokens_from_ast("struct Point { x: Int, y: Int };\n");
    assert!(!tokens.is_empty());
}

#[test]
fn test_semantic_tokens_trait_methods() {
    let tokens = neve_lsp::generate_semantic_tokens_from_ast("trait Show { fn show() = \"\"; };\n");
    assert!(!tokens.is_empty());
}

#[test]
fn test_semantic_tokens_impl_methods() {
    let tokens = neve_lsp::generate_semantic_tokens_from_ast(
        "impl Show for Int { fn show() = \"Int\"; };\n",
    );
    assert!(!tokens.is_empty());
}

#[test]
fn test_document_highlight_read_write() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "let x = 1;\nlet y = x + 2;\n".to_string(),
    );
    if let Some(ref idx) = doc.symbol_index {
        let refs = idx.get_references("x");
        assert_eq!(refs.len(), 2, "Expected 2 references to x (def + use)");
    }
}

#[test]
fn test_inlay_hints_type_inference() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "let x = 42;\nlet y = \"hello\";\n".to_string(),
    );
    assert!(doc.semantics.is_some());
    if let Some(ref semantics) = doc.semantics {
        let type_count = semantics.expr_types.len();
        assert!(type_count > 0, "Expected inferred types");
    }
}

#[test]
fn test_folding_ranges_fn() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "fn add(a, b) = {\n  a + b\n};\n".to_string(),
    );
    assert!(doc.ast.is_some());
}

#[test]
fn test_folding_ranges_struct() {
    let doc = Document::new(
        "file:///test.neve".to_string(),
        "struct Point {\n  x: Int,\n  y: Int,\n};\n".to_string(),
    );
    assert!(doc.symbol_index.is_some());
    assert!(
        doc.symbol_index
            .as_ref()
            .is_some_and(|idx| idx.definitions.contains_key("Point"))
    );
}
