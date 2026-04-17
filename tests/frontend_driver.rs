//! Integration tests for the multi-module frontend driver.
//! 多模块前端驱动的集成测试。

mod support;

use neve_diagnostic::DiagnosticKind;
use neve_frontend::{FrontendDriver, parse_module_file};
use support::module_fixtures::create_test_module;
use tempfile::TempDir;

#[test]
fn test_frontend_driver_analyzes_multi_module_program() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_module(
        root,
        &["math"],
        r#"
            pub fn add(x, y) = x + y;
        "#,
    );
    create_test_module(
        root,
        &["main"],
        r#"
            import math (add);
            fn compute() = add(1, 2);
        "#,
    );

    let analysis = FrontendDriver::new(root)
        .analyze_module_path(&["main".into()])
        .unwrap();

    assert_eq!(analysis.load_order().len(), 2);
    assert!(
        !analysis.type_names().is_empty(),
        "expected shared type name map to be populated"
    );

    for module_id in analysis.load_order() {
        assert!(analysis.module_info(*module_id).is_some());
        assert!(analysis.hir_module(*module_id).is_some());
        assert!(analysis.parsed_source(*module_id).is_some());
        assert!(
            analysis.diagnostics(*module_id).unwrap().is_empty(),
            "unexpected diagnostics for module {:?}: {:?}",
            module_id,
            analysis.diagnostics(*module_id).unwrap()
        );
    }
}

#[test]
fn test_frontend_driver_preserves_parse_diagnostics_per_module() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_module(
        root,
        &["bad"],
        r#"
            pub fn broken(x) =
        "#,
    );
    create_test_module(
        root,
        &["main"],
        r#"
            import bad (broken);
            fn run() = 1;
        "#,
    );

    let analysis = FrontendDriver::new(root)
        .analyze_module_path(&["main".into()])
        .unwrap();

    let bad_module = analysis
        .load_order()
        .iter()
        .copied()
        .find(|module_id| {
            analysis
                .module_info(*module_id)
                .map(|info| info.path.as_slice() == ["bad"])
                .unwrap_or(false)
        })
        .expect("bad module should be loaded");

    let diagnostics = analysis.diagnostics(bad_module).unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Parser),
        "expected parser diagnostics, got {:?}",
        diagnostics
    );
    assert!(
        analysis.method_resolutions(bad_module).unwrap().is_empty(),
        "parse-broken module should not have method resolutions"
    );
}

#[test]
fn test_frontend_driver_returns_dependency_first_diagnostic_modules() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_module(
        root,
        &["bad_parse"],
        r#"
            pub fn broken(x) =
        "#,
    );
    create_test_module(root, &["bad_type"], "pub fn bad() = 1 + true;");
    create_test_module(
        root,
        &["main"],
        r#"
            import bad_parse;
            import bad_type;
            fn run() = 1;
        "#,
    );

    let analysis = FrontendDriver::new(root)
        .analyze_module_path(&["main".into()])
        .unwrap();

    let diagnostic_modules = analysis.diagnostic_modules_in_order();
    let paths: Vec<_> = diagnostic_modules
        .iter()
        .map(|entry| {
            entry
                .file_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    assert_eq!(paths, vec!["bad_parse.neve", "bad_type.neve", "main.neve"]);

    let parse_entry = diagnostic_modules
        .iter()
        .find(|entry| entry.file_path.ends_with("bad_parse.neve"))
        .expect("expected parse-broken module diagnostics");
    assert!(
        parse_entry
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Parser),
        "expected parser diagnostics, got {:?}",
        parse_entry.diagnostics
    );
    assert!(
        parse_entry.source.contains("pub fn broken"),
        "expected diagnostic source text, got {:?}",
        parse_entry.source
    );

    let type_entry = diagnostic_modules
        .iter()
        .find(|entry| entry.file_path.ends_with("bad_type.neve"))
        .expect("expected type-broken module diagnostics");
    assert!(
        type_entry
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected type diagnostics, got {:?}",
        type_entry.diagnostics
    );
}

#[test]
fn test_frontend_driver_returns_only_parse_clean_modules_in_dependency_order() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_module(root, &["util"], "pub fn inc(x) = x + 1;");
    create_test_module(
        root,
        &["bad_parse"],
        r#"
            pub fn broken(x) =
        "#,
    );
    create_test_module(
        root,
        &["main"],
        r#"
            import util (inc);
            import bad_parse;
            fn run() = inc(1);
        "#,
    );

    let analysis = FrontendDriver::new(root)
        .analyze_module_path(&["main".into()])
        .unwrap();

    let parsed_paths: Vec<_> = analysis
        .parsed_modules_in_order()
        .into_iter()
        .map(|entry| {
            entry
                .file_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect();

    assert_eq!(parsed_paths, vec!["util.neve", "main.neve"]);
}

#[test]
fn test_frontend_parse_module_file_returns_parse_clean_ast_payload() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_module(root, &["std", "mod"], "let result = 42;");
    let parsed = parse_module_file(root.join("std").join("mod.neve"), &["std".into()]).unwrap();

    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed
    );
    assert_eq!(parsed.module_path, vec!["std"]);
    assert!(parsed.ast.is_some(), "expected parse-clean AST payload");
    assert!(parsed.source.contains("let result = 42;"));
}

#[test]
fn test_frontend_driver_returns_only_evaluable_modules_in_dependency_order() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_module(root, &["util"], "pub fn inc(x) = x + 1;");
    create_test_module(root, &["broken"], "pub fn bad() = 1 + true;");
    create_test_module(
        root,
        &["main"],
        r#"
            import util (inc);
            import broken (bad);
            fn compute() = inc(1);
        "#,
    );

    let analysis = FrontendDriver::new(root)
        .analyze_module_path(&["main".into()])
        .unwrap();

    let names_by_module_id: std::collections::HashMap<_, _> = analysis
        .load_order()
        .iter()
        .filter_map(|module_id| {
            analysis.module_info(*module_id).map(|info| {
                (
                    *module_id,
                    info.file_path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                )
            })
        })
        .collect();
    let evaluable_paths: Vec<_> = analysis
        .evaluable_modules_in_order()
        .into_iter()
        .map(|entry| names_by_module_id.get(&entry.module_id).unwrap().clone())
        .collect();

    assert_eq!(evaluable_paths, vec!["util.neve", "main.neve"]);
}
