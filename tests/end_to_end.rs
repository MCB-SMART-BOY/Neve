//! Integration smoke tests for the real frontend/runtime paths.
//!
//! These tests intentionally avoid placeholder helpers. They cover:
//! - parse + lower + type check via `neve_frontend`
//! - runtime parity between AST and HIR evaluators on a supported subset
//! - explicit sentinels for currently known runtime divergence

use neve_common::Int;
use neve_diagnostic::DiagnosticKind;
use neve_eval::{AstEvaluator, EvalError, Evaluator, Value};
use neve_frontend::{AnalysisResult, analyze_source};

fn int(value: i64) -> Int {
    value.into()
}

fn analyze_without_diagnostics(source: &str) -> AnalysisResult {
    let analysis = analyze_source(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected frontend diagnostics: {:?}",
        analysis.diagnostics
    );
    analysis
}

fn eval_ast(analysis: &AnalysisResult) -> Result<Value, EvalError> {
    let mut evaluator = AstEvaluator::new();
    evaluator.eval_file(&analysis.ast)
}

fn eval_hir(analysis: &AnalysisResult) -> Result<Value, EvalError> {
    let mut evaluator = Evaluator::new();
    evaluator.eval_module(&analysis.hir)
}

fn assert_runtime_parity(source: &str, expected: Value) {
    let analysis = analyze_without_diagnostics(source);

    let ast_value = eval_ast(&analysis).expect("AST evaluator should succeed");
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");

    assert_eq!(ast_value, expected, "unexpected AST result");
    assert_eq!(hir_value, expected, "unexpected HIR result");
    assert_eq!(ast_value, hir_value, "AST/HIR runtime split detected");
}

#[test]
fn test_frontend_reports_parse_errors() {
    let analysis = analyze_source("let x =");
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Parser),
        "expected parser diagnostics, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_reports_type_errors() {
    let analysis = analyze_source("let x = 1 + true;");
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected type diagnostics, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_arithmetic_runtime_parity() {
    assert_runtime_parity("let x = 1 + 2 * 3;", Value::Int(int(7)));
}

#[test]
fn test_end_to_end_pipe_runtime_parity() {
    assert_runtime_parity(
        "
        fn double(x) = x * 2;
        let x = 40 |> double |> double;
        ",
        Value::Int(int(160)),
    );
}

#[test]
fn test_end_to_end_record_field_runtime_parity() {
    assert_runtime_parity(
        "
        let config = #{ port = 40, host = \"localhost\" };
        let x = config.port;
        ",
        Value::Int(int(40)),
    );
}

#[test]
fn test_end_to_end_list_match_runtime_parity() {
    assert_runtime_parity(
        "
        fn sum_pair(xs) = match xs {
            [a, b] -> a + b,
            _ -> 0,
        };
        let x = sum_pair([1, 2]);
        ",
        Value::Int(int(3)),
    );
}

#[test]
fn test_end_to_end_enum_match_runtime_parity() {
    assert_runtime_parity(
        "
        enum Option { Some(Int), None };
        let x = Some(1);
        let y = match x {
            Some(v) -> v + 1,
            None -> 0,
        };
        ",
        Value::Int(int(2)),
    );
}

#[test]
fn test_end_to_end_recursive_fibonacci_runtime_parity() {
    assert_runtime_parity(
        "
        fn fib(n) = if n <= 1 then n else fib(n - 1) + fib(n - 2);
        let x = fib(10);
        ",
        Value::Int(int(55)),
    );
}

#[test]
fn test_end_to_end_lazy_force_runtime_parity() {
    assert_runtime_parity(
        "
        let thunk = lazy 42;
        let x = force(thunk);
        ",
        Value::Int(int(42)),
    );
}

#[test]
fn test_end_to_end_or_pattern_runtime_parity() {
    assert_runtime_parity(
        "
        let x = match (1, 2) {
            (0, v) | (1, v) -> v,
            _ -> 0,
        };
        ",
        Value::Int(int(2)),
    );
}

#[test]
fn test_end_to_end_binding_pattern_runtime_parity() {
    assert_runtime_parity(
        "
        let x = match 42 {
            n @ 42 -> n + 1,
            _ -> 0,
        };
        ",
        Value::Int(int(43)),
    );
}
