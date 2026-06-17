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
