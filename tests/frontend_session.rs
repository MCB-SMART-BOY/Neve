//! Integration tests for compatibility frontend session APIs.
//! 兼容前端会话 API 的集成测试。

use std::fs;
use std::path::Path;

use neve_diagnostic::{DiagnosticKind, Severity};
use neve_frontend::{FrontendSession, SessionBuildInputs};
use neve_parser::parse;
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

fn parse_ok(source: &str) -> neve_syntax::SourceFile {
    let (ast, diagnostics) = parse(source);
    assert!(
        diagnostics.is_empty(),
        "unexpected parse diagnostics: {diagnostics:?}"
    );
    ast
}

#[test]
fn test_frontend_session_builds_in_memory_module_against_loaded_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    create_test_module(temp_dir.path(), &["math"], "pub fn add(x, y) = x + y;");

    let ast = parse_ok(
        r#"
            import math (add);
            fn compute() = add(1, 2);
        "#,
    );

    let mut session = FrontendSession::new(temp_dir.path());
    let build = session
        .build_module_from_ast(
            &ast,
            "repl".to_string(),
            Vec::new(),
            &SessionBuildInputs::default(),
        )
        .expect("session build should succeed");

    assert_eq!(
        build.newly_loaded.len(),
        1,
        "expected one loaded dependency"
    );
    assert!(
        !build.global_defs.is_empty(),
        "expected current module globals"
    );

    let analysis = session.analyze_module(&build.module);
    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_session_resolves_imported_bindings_and_module_aliases() {
    let temp_dir = TempDir::new().unwrap();
    create_test_module(temp_dir.path(), &["math"], "pub fn add(x, y) = x + y;");

    let ast = parse_ok(
        r#"
            import math (add);
            import math as ops;
            fn compute() = ops.add(add(1, 2), 3);
        "#,
    );

    let mut session = FrontendSession::new(temp_dir.path());
    session
        .build_module_from_ast(
            &ast,
            "repl".to_string(),
            Vec::new(),
            &SessionBuildInputs::default(),
        )
        .expect("session build should succeed");

    let resolved = session
        .resolve_ast_imports(&ast, &[])
        .expect("import resolution should succeed");

    assert!(
        resolved.bindings.iter().any(|(name, _)| name == "add"),
        "expected imported item binding, got {:?}",
        resolved.bindings
    );
    assert!(
        resolved.module_aliases.iter().any(|alias| alias == "ops"),
        "expected imported module alias, got {:?}",
        resolved.module_aliases
    );
}

#[test]
fn test_frontend_session_attributes_diagnostics_for_newly_loaded_modules() {
    let temp_dir = TempDir::new().unwrap();
    create_test_module(temp_dir.path(), &["broken"], "pub fn bad() = 1 + true;");

    let ast = parse_ok(
        r#"
            import broken (bad);
            fn run() = 1;
        "#,
    );

    let mut session = FrontendSession::new(temp_dir.path());
    let build = session
        .build_module_from_ast(
            &ast,
            "repl".to_string(),
            Vec::new(),
            &SessionBuildInputs::default(),
        )
        .expect("session build should succeed");

    let diagnostics = session.loaded_module_diagnostics(&build.newly_loaded);
    assert_eq!(diagnostics.len(), 1, "expected one broken dependency");
    assert!(diagnostics[0].file_path.ends_with("broken.neve"));
    assert!(
        diagnostics[0]
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type && diag.severity == Severity::Error),
        "expected type error diagnostics, got {:?}",
        diagnostics[0].diagnostics
    );
}
