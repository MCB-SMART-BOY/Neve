//! The `neve eval` command — canonical HIR pipeline.
//! `neve eval` 命令 — 规范 HIR 管线。

use crate::output;
use neve_diagnostic::{Severity, emit};
use neve_eval::{Evaluator, Value};
use neve_frontend::{LoadedSnippetModule, analyze_snippet_ast};
use neve_parser::parse;
use neve_std::stdlib;

/// Run the eval command.
pub fn run(expr: &str, verbose: bool) -> Result<(), String> {
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

/// Prepare the source for parsing by wrapping expressions appropriately.
fn prepare_source(expr: &str) -> String {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return String::new();
    }

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
        if trimmed.ends_with(';') {
            return trimmed.to_string();
        }
        return format!("{trimmed};");
    }

    format!("let __result__ = {trimmed};")
}

fn eval_and_print(
    file: &neve_syntax::SourceFile,
    source: &str,
    verbose: bool,
) -> Result<(), String> {
    if verbose {
        output::info(&format!("AST: {file:?}"));
    }

    let root_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let value = eval_value(file, source, &root_dir)?;

    if !matches!(value, Value::Unit) || source.starts_with("let __result__") {
        output::success(&format!("{value:?}"));
    }

    Ok(())
}

fn eval_value(
    file: &neve_syntax::SourceFile,
    source: &str,
    root_dir: &std::path::Path,
) -> Result<Value, String> {
    let analysis =
        analyze_snippet_ast(file, root_dir).map_err(|e| format!("frontend error: {e}"))?;

    let mut loaded_had_errors = false;
    let mut loaded_had_parse_errors = false;
    for entry in &analysis.loaded_modules {
        emit_loaded_module_diagnostics(entry, &mut loaded_had_errors, &mut loaded_had_parse_errors);
    }

    for diag in &analysis.diagnostics {
        emit(source, "<eval>", diag);
    }
    if loaded_had_parse_errors {
        return Err("parse error".to_string());
    }
    if loaded_had_errors
        || analysis
            .diagnostics
            .iter()
            .any(|diag| diag.severity == Severity::Error)
    {
        return Err("type error".to_string());
    }

    let mut evaluator = Evaluator::new().with_extra_builtins(
        stdlib()
            .into_iter()
            .map(|(name, value)| (name.to_string(), value)),
    );
    for entry in &analysis.evaluable_loaded_modules {
        evaluator
            .eval_module_with_method_resolutions(&entry.module, &entry.method_resolutions)
            .map_err(|e| format!("eval error: {e:?}"))?;
    }

    evaluator
        .eval_module_with_method_resolutions(&analysis.hir, &analysis.semantics.method_resolutions)
        .map_err(|e| format!("eval error: {e:?}"))
}

fn emit_loaded_module_diagnostics(
    entry: &LoadedSnippetModule,
    had_errors: &mut bool,
    had_parse_errors: &mut bool,
) {
    let source = std::fs::read_to_string(&entry.file_path).unwrap_or_default();
    for diag in &entry.diagnostics {
        emit(&source, &entry.file_path.display().to_string(), diag);
        if diag.severity == Severity::Error {
            *had_errors = true;
        }
        if diag.kind == neve_diagnostic::DiagnosticKind::Parser {
            *had_parse_errors = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::prepare_source;

    #[test]
    fn test_prepare_source_expression() {
        let result = prepare_source("1 + 2");
        assert_eq!(result, "let __result__ = 1 + 2;");
    }

    #[test]
    fn test_prepare_source_let() {
        let result = prepare_source("let x = 1;");
        assert_eq!(result, "let x = 1;");
    }

    #[test]
    fn test_prepare_source_fn() {
        let result = prepare_source("fn f(x) = x + 1;");
        assert_eq!(result, "fn f(x) = x + 1;");
    }

    #[test]
    fn test_prepare_source_empty() {
        let result = prepare_source("");
        assert_eq!(result, "");
    }
}
