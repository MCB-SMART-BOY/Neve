//! Integration tests for the multi-module frontend driver.
//! 多模块前端驱动的集成测试。

use std::fs;
use std::path::Path;

use neve_diagnostic::DiagnosticKind;
use neve_frontend::FrontendDriver;
use tempfile::TempDir;

fn create_test_module(dir: &Path, path: &[&str], content: &str) {
    let mut full_path = dir.to_path_buf();
    for (i, segment) in path.iter().enumerate() {
        full_path.push(segment);
        if i < path.len() - 1 {
            fs::create_dir_all(&full_path).unwrap();
        }
    }
    full_path.set_extension("neve");
    fs::write(full_path, content).unwrap();
}

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
