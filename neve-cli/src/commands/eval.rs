//! The `neve eval` command.
//! `neve eval` 命令。

use crate::output;
use neve_diagnostic::{emit, Severity};
use neve_eval::{AstEvaluator, EvalError, Evaluator, Value};
use neve_frontend::analyze_ast;
use neve_hir::ModuleLoader;
use neve_parser::parse;
use neve_std::std_module_overrides;
use neve_syntax::{ItemKind, SourceFile};
use std::path::{Path, PathBuf};

/// Run the eval command.
/// 运行 eval 命令。
pub fn run(expr: &str, verbose: bool) -> Result<(), String> {
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

    eval_and_print(&file, &source, verbose)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvalBackend {
    FrontendHir,
    AstFallback,
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
) -> Result<(), String> {
    if verbose {
        output::info(&format!("AST: {file:?}"));
    }

    let root_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (backend, value) = eval_value(file, source, &root_dir)?;

    if verbose {
        output::info(match backend {
            EvalBackend::FrontendHir => "Eval backend: frontend/HIR",
            EvalBackend::AstFallback => "Eval backend: AST fallback",
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
) -> Result<(EvalBackend, Value), String> {
    if has_imports(file) {
        let value = eval_via_ast(file, root_dir)?;
        return Ok((EvalBackend::AstFallback, value));
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

    let mut evaluator =
        Evaluator::new().with_method_resolutions(analysis.method_resolutions.clone());
    evaluator
        .eval_module(&analysis.hir)
        .map(|value| (EvalBackend::FrontendHir, value))
        .map_err(format_hir_eval_error)
}

fn eval_via_ast(file: &SourceFile, root_dir: &Path) -> Result<Value, String> {
    let mut evaluator = AstEvaluator::new()
        .with_module_overrides(std_module_overrides())
        .with_base_path(root_dir.to_path_buf())
        .with_module_loader(ModuleLoader::new(root_dir));

    evaluator.eval_file(file).map_err(format_ast_eval_error)
}

fn has_imports(file: &SourceFile) -> bool {
    file.items
        .iter()
        .any(|item| matches!(item.kind, ItemKind::Import(_)))
}

fn format_ast_eval_error(err: EvalError) -> String {
    match err {
        EvalError::ParseDiagnostics {
            path,
            source_text,
            diagnostics,
            ..
        } => {
            for diag in diagnostics {
                emit(&source_text, &path.display().to_string(), &diag);
            }
            "parse error".to_string()
        }
        other => {
            output::error(&format!("{other:?}"));
            "evaluation error".to_string()
        }
    }
}

fn format_hir_eval_error(err: EvalError) -> String {
    output::error(&format!("{err:?}"));
    "evaluation error".to_string()
}

#[cfg(test)]
mod tests {
    use super::{eval_value, prepare_source, EvalBackend};
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

        let (backend, value) = eval_value(&file, &source, TempDir::new().unwrap().path()).unwrap();
        assert_eq!(backend, EvalBackend::FrontendHir);
        assert_eq!(value, Value::Int(3.into()));
    }

    #[test]
    fn eval_value_falls_back_to_ast_for_imports() {
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

        let (backend, value) = eval_value(&file, &source, temp_dir.path()).unwrap();
        assert_eq!(backend, EvalBackend::AstFallback);
        assert_eq!(value, Value::Int(3.into()));
    }
}
