//! Integration tests for root-level `std` builtin-module imports.

use neve_common::Int;
use neve_eval::{Evaluator, Value, compat::AstEvaluator};
use neve_frontend::{analyze_snippet_ast, analyze_source};
use neve_parser::parse;
use neve_std::{std_module_overrides, stdlib};
use tempfile::TempDir;

fn int(value: i64) -> Int {
    value.into()
}

#[test]
fn test_frontend_accepts_std_root_module_item_imports() {
    let analysis = analyze_source("import std (list); let result = list.len([1, 2]);");
    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_snippet_analysis_accepts_std_root_module_item_imports() {
    let source = "import std (list); let result = list.len([1, 2]);";
    let (ast, diagnostics) = parse(source);
    assert!(
        diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        diagnostics
    );

    let analysis = analyze_snippet_ast(&ast, TempDir::new().unwrap().path())
        .expect("snippet analysis should succeed");
    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_std_root_module_item_import_runtime_parity() {
    let analysis = analyze_source("import std (list); let result = list.len([1, 2]);");
    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );

    let ast_value = AstEvaluator::new()
        .with_module_overrides(std_module_overrides())
        .eval_file(&analysis.ast)
        .expect("AST evaluator should succeed");

    let hir_value = Evaluator::new()
        .with_extra_builtins(
            stdlib()
                .into_iter()
                .map(|(name, value)| (name.to_string(), value)),
        )
        .eval_module_with_method_resolutions(&analysis.hir, &analysis.semantics.method_resolutions)
        .expect("HIR evaluator should succeed");

    let expected = Value::Int(int(2));
    assert_eq!(ast_value, expected, "unexpected AST result");
    assert_eq!(hir_value, expected, "unexpected HIR result");
    assert_eq!(ast_value, hir_value, "AST/HIR runtime split detected");
}
