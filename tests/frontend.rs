//! Integration tests for the frontend analysis pipeline.
//! 前端分析管线的集成测试。

use neve_diagnostic::DiagnosticKind;
use neve_frontend::{analyze_snippet_ast, analyze_source};
use neve_parser::parse;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_frontend_reports_parse_errors() {
    let result = analyze_source("let x =");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Parser),
        "expected parser diagnostics"
    );
}

#[test]
fn test_frontend_reports_type_errors() {
    let result = analyze_source("let x = 1 + true;");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected type diagnostics"
    );
}

#[test]
fn test_frontend_formats_named_types_readably_in_diagnostics() {
    let result = analyze_source(
        r#"
            struct User {};
            fn broken(x: User) -> Int = x.name;
        "#,
    );

    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected type diagnostics, got {:?}",
        result.diagnostics
    );

    for diagnostic in &result.diagnostics {
        assert!(
            !diagnostic.message.contains("Type#"),
            "unexpected raw type placeholder in message: {:?}",
            diagnostic
        );
        for label in &diagnostic.labels {
            assert!(
                !label.message.contains("Type#"),
                "unexpected raw type placeholder in label: {:?}",
                diagnostic
            );
        }
        for note in &diagnostic.notes {
            assert!(
                !note.contains("Type#"),
                "unexpected raw type placeholder in note: {:?}",
                diagnostic
            );
        }
        if let Some(help) = &diagnostic.help {
            assert!(
                !help.contains("Type#"),
                "unexpected raw type placeholder in help: {:?}",
                diagnostic
            );
        }
    }
}

#[test]
fn test_frontend_accepts_record_field_access_after_record_binding() {
    let result = analyze_source(
        r#"
            let config = #{ port = 40, host = "localhost" };
            let x = config.port;
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_lazy_force_pipeline() {
    let result = analyze_source(
        r#"
            let thunk = lazy 42;
            let x = force(thunk);
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_or_and_binding_patterns() {
    let result = analyze_source(
        r#"
            let a = match (1, 2) { (0, v) | (1, v) -> v, _ -> 0 };
            let b = match 42 { n @ 42 -> n, _ -> 0 };
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_list_rest_patterns() {
    let result = analyze_source(
        r#"
            let x = match [1, 2, 3, 4] {
                [first, ..middle, last] -> match middle {
                    [a, b] -> first + a + b + last,
                    _ -> 0,
                },
                _ -> 0,
            };
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_reports_impl_method_type_errors() {
    let result = analyze_source(
        r#"
            struct Counter {};
            impl Counter {
                fn value(self) -> Int = true;
            };
        "#,
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected impl method type diagnostics, got {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_reports_trait_impl_signature_mismatch() {
    let result = analyze_source(
        r#"
            trait Show { fn show(self) -> Int; };
            struct Counter {};
            impl Show for Counter {
                fn show(self) -> String = "counter";
            };
        "#,
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected trait impl signature diagnostics, got {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_self_and_assoc_type_use_sites() {
    let result = analyze_source(
        r#"
            trait Iterator {
                type Item;
                fn first(self) -> Self.Item;
            };
            struct Counter {};
            impl Iterator for Counter {
                type Item = Int;
                fn first(self) -> Self.Item = 1;
            };
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_try_on_option_and_result_like_enums() {
    let result = analyze_source(
        r#"
            enum Option { Some(Int), None };
            enum Result { Ok(Int), Err(String) };
            let a = Some(41)? + 1;
            let b = Ok(1)? + 1;
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_coalesce_on_safe_field_and_option_enum() {
    let result = analyze_source(
        r#"
            enum Option { Some(Int), None };
            let a = Some(41) ?? 0;
            let r = #{ name = "test" };
            let b = r?.missing ?? "default";
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_snippet_accepts_local_imports_against_root_dir() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(
        temp_dir.path().join("math.neve"),
        "pub fn add(x, y) = x + y;",
    )
    .unwrap();

    let source = "import math (add); let result = add(1, 2);";
    let (ast, diagnostics) = parse(source);
    assert!(
        diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        diagnostics
    );

    let analysis =
        analyze_snippet_ast(&ast, temp_dir.path()).expect("snippet analysis should succeed");
    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected snippet diagnostics: {:?}",
        analysis.diagnostics
    );
    assert!(
        analysis
            .loaded_modules
            .iter()
            .any(|entry| entry.file_path.ends_with("math.neve") && entry.diagnostics.is_empty()),
        "expected successfully loaded dependency module, got {:?}",
        analysis.loaded_modules
    );
}

#[test]
fn test_frontend_snippet_reports_loaded_module_diagnostics() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("math.neve"), "pub fn add(x, y) = ;").unwrap();

    let source = "import math (add); let result = add(1, 2);";
    let (ast, diagnostics) = parse(source);
    assert!(
        diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        diagnostics
    );

    let analysis =
        analyze_snippet_ast(&ast, temp_dir.path()).expect("snippet analysis should succeed");
    assert!(
        analysis
            .loaded_modules
            .iter()
            .flat_map(|entry| entry.diagnostics.iter())
            .any(|diag| diag.kind == DiagnosticKind::Parser),
        "expected loaded parser diagnostics, got {:?}",
        analysis.loaded_modules
    );
}

#[test]
fn test_frontend_accepts_trait_method_call_analysis() {
    let result = analyze_source(
        r#"
            trait Show { fn show(self) -> String; };
            impl Show for Int {
                fn show(self) -> String = toString(self);
            };
            let x = 1.show();
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_item_and_module_imports() {
    let result = analyze_source(
        r#"
            import std.list (len);
            import std.string as string;
            let a = len([1, 2, 3]);
            let b = string.len("abc");
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_glob_imports() {
    let result = analyze_source(
        r#"
            import std.list (*);
            let x = len([1, 2, 3]);
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_option_and_result_builtins() {
    let result = analyze_source(
        r#"
            import std.option as option;
            import std.result as result;
            let a = option.some(41)? + 1;
            let b = option.none ?? 5;
            let c = result.ok(1)? + 1;
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_path_builtins() {
    let result = analyze_source(
        r#"
            import std.path as path;
            let a = path.join("a", "b");
            let b = path.parent("/tmp/file.txt") ?? "/";
            let c = path.is_absolute("/tmp/file.txt");
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_and_fetch_builtins() {
    let result = analyze_source(
        r#"
            import std.io as io;
            import std.fetch as fetch;
            let a = io.hashString("abc");
            let b = io.exec("printf", ["neve"]).stdout;
            let c = fetch.path("Cargo.toml").hash;
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_map_and_set_builtins() {
    let result = analyze_source(
        r#"
            import std.Map;
            import std.Set;
            let map = Map.insert("a", 1, Map.empty);
            let set = Set.insert(1, Set.empty);
            let value = Map.getWithDefault("a", 0, map) + Set.size(set);
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}
