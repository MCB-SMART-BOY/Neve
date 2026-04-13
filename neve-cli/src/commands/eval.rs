//! The `neve eval` command.
//! `neve eval` 命令。

use crate::commands::run::{RunBackend, run_value as run_file_value};
use crate::output;
use neve_diagnostic::{Severity, emit};
use neve_eval::{Evaluator, Value};
use neve_frontend::analyze_ast;
use neve_parser::parse;
use neve_std::stdlib;
use neve_syntax::{ImportDef, ItemKind, SourceFile};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Run the eval command.
/// 运行 eval 命令。
pub fn run(expr: &str, verbose: bool, compat_ast: bool) -> Result<(), String> {
    // Prepare source for parsing
    // 准备用于解析的源码
    // Strategy: if there's content after the last semicolon that looks like an expression,
    // wrap it in a let binding so it becomes a valid item
    let source = prepare_source(expr);

    let (file, diagnostics) = parse(&source);

    for diag in &diagnostics {
        emit(&source, "<eval>", diag);
    }

    if !diagnostics.is_empty() {
        return Err("parse error".to_string());
    }

    eval_and_print(&file, &source, verbose, compat_ast)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvalBackend {
    FrontendHir,
    AstCompat,
}

/// Prepare the source for parsing by wrapping expressions appropriately.
/// 通过适当包装表达式来准备用于解析的源码。
fn prepare_source(expr: &str) -> String {
    let trimmed = expr.trim();

    if trimmed.is_empty() {
        return String::new();
    }

    // Check if it's already a valid item (starts with keyword)
    // 检查是否已经是有效的项（以关键字开头）
    let is_item = trimmed.starts_with("let ")
        || trimmed.starts_with("fn ")
        || trimmed.starts_with("type ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("enum ")
        || trimmed.starts_with("trait ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("pub ");

    if is_item {
        // It's already an item, just ensure it ends with semicolon
        // 已经是一个项，只需确保以分号结尾
        if trimmed.ends_with(';') {
            return trimmed.to_string();
        } else {
            return format!("{trimmed};");
        }
    }

    // For expressions, wrap in a block-based let binding
    // 对于表达式，包装在基于块的 let 绑定中
    // This handles expressions like `{ let x = 1; x * 2 }` or simple `1 + 2`
    // 这处理像 `{ let x = 1; x * 2 }` 或简单的 `1 + 2` 这样的表达式
    // We wrap the expression: let __result__ = <expr>;
    // 我们包装表达式：let __result__ = <expr>;
    // But if it's a block expression, it will work directly
    // 但如果是块表达式，它将直接工作
    format!("let __result__ = {trimmed};")
}

/// Evaluate and print the result.
/// 求值并打印结果。
fn eval_and_print(
    file: &neve_syntax::SourceFile,
    source: &str,
    verbose: bool,
    compat_ast: bool,
) -> Result<(), String> {
    if verbose {
        output::info(&format!("AST: {file:?}"));
    }

    let root_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (backend, value) = eval_value(file, source, &root_dir, compat_ast)?;

    if verbose {
        output::info(match backend {
            EvalBackend::FrontendHir => "Eval backend: frontend/HIR",
            EvalBackend::AstCompat => "Eval backend: AST compat",
        });
    }

    // Don't print Unit for statements that don't return values
    // 对于不返回值的语句，不打印 Unit
    if !matches!(value, Value::Unit) || source.starts_with("let __result__") {
        output::success(&format!("{value:?}"));
    }

    Ok(())
}

fn eval_value(
    file: &SourceFile,
    source: &str,
    root_dir: &Path,
    compat_ast: bool,
) -> Result<(EvalBackend, Value), String> {
    if source_file_requires_module_graph(file) {
        return eval_via_module_graph(source, root_dir, compat_ast);
    }

    let analysis = analyze_ast(file);
    for diag in &analysis.diagnostics {
        emit(source, "<eval>", diag);
    }
    if analysis
        .diagnostics
        .iter()
        .any(|diag| diag.severity == Severity::Error)
    {
        return Err("type error".to_string());
    }

    let mut evaluator = Evaluator::new()
        .with_method_resolutions(analysis.method_resolutions.clone())
        .with_extra_builtins(std_builtin_values());
    evaluator
        .eval_module(&analysis.hir)
        .map(|value| (EvalBackend::FrontendHir, value))
        .map_err(format_hir_eval_error)
}

fn eval_via_module_graph(
    source: &str,
    root_dir: &Path,
    compat_ast: bool,
) -> Result<(EvalBackend, Value), String> {
    let temp_module = TempEvalModule::create(source, root_dir)?;
    let (backend, value) = run_file_value(temp_module.path(), false, compat_ast)?;
    Ok((map_run_backend(backend), value))
}

fn map_run_backend(backend: RunBackend) -> EvalBackend {
    match backend {
        RunBackend::FrontendHir => EvalBackend::FrontendHir,
        RunBackend::AstCompat => EvalBackend::AstCompat,
    }
}

struct TempEvalModule {
    path: PathBuf,
}

impl TempEvalModule {
    fn create(source: &str, root_dir: &Path) -> Result<Self, String> {
        let path = temp_eval_module_path(root_dir);
        fs::write(&path, source)
            .map_err(|e| format!("cannot write eval temp file '{}': {}", path.display(), e))?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempEvalModule {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn temp_eval_module_path(root_dir: &Path) -> PathBuf {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    root_dir.join(format!("__neve_eval_{pid}_{nanos}.neve"))
}

fn source_file_requires_module_graph(file: &SourceFile) -> bool {
    file.items.iter().any(|item| match &item.kind {
        ItemKind::Import(import) => import_requires_module_graph(import),
        _ => false,
    })
}

fn import_requires_module_graph(import: &ImportDef) -> bool {
    !is_supported_std_import(import)
}

fn is_supported_std_import(import: &ImportDef) -> bool {
    import.path.len() == 2
        && import.path.first().map(|segment| segment.name.as_str()) == Some("std")
}

fn std_builtin_values() -> impl Iterator<Item = (String, Value)> {
    stdlib()
        .into_iter()
        .map(|(name, value)| (name.to_string(), value))
}

fn format_hir_eval_error(err: neve_eval::EvalError) -> String {
    output::error(&format!("{err:?}"));
    "evaluation error".to_string()
}

#[cfg(test)]
mod tests {
    use super::{EvalBackend, eval_value, prepare_source};
    use neve_eval::Value;
    use neve_parser::parse;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn eval_value_prefers_frontend_hir_without_imports() {
        let source = prepare_source("1 + 2");
        let (file, diagnostics) = parse(&source);
        assert!(
            diagnostics.is_empty(),
            "unexpected parse errors: {:?}",
            diagnostics
        );

        let (backend, value) =
            eval_value(&file, &source, TempDir::new().unwrap().path(), false).unwrap();
        assert_eq!(backend, EvalBackend::FrontendHir);
        assert_eq!(value, Value::Int(3.into()));
    }

    #[test]
    fn eval_value_prefers_frontend_hir_for_local_imports() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("math.neve"),
            "pub fn add(x, y) = x + y;",
        )
        .unwrap();

        let source = prepare_source("import math (add); let result = add(1, 2)");
        let (file, diagnostics) = parse(&source);
        assert!(
            diagnostics.is_empty(),
            "unexpected parse errors: {:?}",
            diagnostics
        );

        let (backend, value) = eval_value(&file, &source, temp_dir.path(), false).unwrap();
        assert_eq!(backend, EvalBackend::FrontendHir);
        assert_eq!(value, Value::Int(3.into()));
    }

    #[test]
    fn eval_value_prefers_frontend_hir_for_std_item_imports() {
        let source = prepare_source("import std.list (len); let result = len([1, 2, 3])");
        let (file, diagnostics) = parse(&source);
        assert!(
            diagnostics.is_empty(),
            "unexpected parse errors: {:?}",
            diagnostics
        );

        let (backend, value) =
            eval_value(&file, &source, TempDir::new().unwrap().path(), false).unwrap();
        assert_eq!(backend, EvalBackend::FrontendHir);
        assert_eq!(value, Value::Int(3.into()));
    }

    #[test]
    fn eval_value_prefers_frontend_hir_for_std_module_imports() {
        let source = prepare_source("import std.list; let result = list.len([1, 2])");
        let (file, diagnostics) = parse(&source);
        assert!(
            diagnostics.is_empty(),
            "unexpected parse errors: {:?}",
            diagnostics
        );

        let (backend, value) =
            eval_value(&file, &source, TempDir::new().unwrap().path(), false).unwrap();
        assert_eq!(backend, EvalBackend::FrontendHir);
        assert_eq!(value, Value::Int(2.into()));
    }

    #[test]
    fn eval_value_prefers_frontend_hir_for_std_glob_imports() {
        let source = prepare_source("import std.list (*); let result = len([1, 2, 3])");
        let (file, diagnostics) = parse(&source);
        assert!(
            diagnostics.is_empty(),
            "unexpected parse errors: {:?}",
            diagnostics
        );

        let (backend, value) =
            eval_value(&file, &source, TempDir::new().unwrap().path(), false).unwrap();
        assert_eq!(backend, EvalBackend::FrontendHir);
        assert_eq!(value, Value::Int(3.into()));
    }

    #[test]
    fn eval_value_rejects_implicit_ast_compat_for_unsupported_std_import_shape() {
        let source = prepare_source("import std (list); let result = list.len([1, 2])");
        let (file, diagnostics) = parse(&source);
        assert!(
            diagnostics.is_empty(),
            "unexpected parse errors: {:?}",
            diagnostics
        );

        let err = eval_value(&file, &source, TempDir::new().unwrap().path(), false).unwrap_err();
        assert!(err.contains("--compat-ast"), "unexpected error: {err}");
    }

    #[test]
    fn eval_value_uses_ast_compat_when_explicit_for_unsupported_std_import_shape() {
        let source = prepare_source("import std (list); let result = list.len([1, 2])");
        let (file, diagnostics) = parse(&source);
        assert!(
            diagnostics.is_empty(),
            "unexpected parse errors: {:?}",
            diagnostics
        );

        let (backend, value) =
            eval_value(&file, &source, TempDir::new().unwrap().path(), true).unwrap();
        assert_eq!(backend, EvalBackend::AstCompat);
        assert_eq!(value, Value::Int(2.into()));
    }
}
