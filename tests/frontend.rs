//! Integration tests for the frontend analysis pipeline.
//! 前端分析管线的集成测试。

use neve_diagnostic::DiagnosticKind;
use neve_frontend::analyze_source;

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
