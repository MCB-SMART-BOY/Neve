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
