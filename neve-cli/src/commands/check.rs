//! The `neve check` command.
//! `neve check` 命令。

use crate::{commands::module_graph, output};
use neve_diagnostic::{DiagnosticKind, emit};
use neve_frontend::FrontendDriver;
use neve_hir::{ExprKind, ItemKind, StmtKind};
use neve_std::is_effectful_builtin;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Discover flake inputs (Unix only).
#[cfg(unix)]
fn discover_flake_input_roots(start_dir: &Path) -> HashMap<String, PathBuf> {
    let mut current = start_dir.to_path_buf();
    while current.parent().is_some() {
        let flake_path = current.join("flake.neve");
        if flake_path.exists() {
            if let Ok(mut flake) = neve_config::flake::Flake::load(&current) {
                let _ = flake.lock_inputs();
                return flake.collect_input_roots().unwrap_or_default();
            }
            break;
        }
        current = current.parent().unwrap().to_path_buf();
    }
    HashMap::new()
}

#[cfg(not(unix))]
fn discover_flake_input_roots(_start_dir: &Path) -> HashMap<String, PathBuf> {
    HashMap::new()
}

/// Run type checking on a Neve file.
/// 对 Neve 文件运行类型检查。
pub fn run(file: &str, verbose: bool, allow_effects: bool) -> Result<(), String> {
    let path = Path::new(file);
    let (root_dir, module_path) = module_graph::resolve_module_path(path)?;

    let flake_inputs = discover_flake_input_roots(&root_dir);
    let mut driver = FrontendDriver::new(&root_dir);
    if !flake_inputs.is_empty() {
        driver = driver.with_flake_inputs(flake_inputs);
    }
    let analysis = driver
        .analyze_module_path(&module_path)
        .map_err(|e| format!("frontend error: {e}"))?;

    let mut parse_errors = 0usize;
    let mut type_errors = 0usize;

    for entry in analysis.diagnostic_modules_in_order() {
        for diag in &entry.diagnostics {
            emit(&entry.source, &entry.file_path.display().to_string(), diag);
        }

        let parse_diagnostics = entry
            .diagnostics
            .iter()
            .filter(|diag| diag.kind == DiagnosticKind::Parser)
            .count();
        if parse_diagnostics > 0 {
            parse_errors += parse_diagnostics;
            continue;
        }

        if verbose && let Some(ast) = analysis.parsed_source(entry.module_id) {
            let form_count = ast.items.len() + usize::from(ast.tail_expr.is_some());
            output::info(&format!(
                "Parsed {} top-level form(s) in {}",
                form_count,
                entry.file_path.display()
            ));
        }

        if verbose && let Some(module) = analysis.hir_module(entry.module_id) {
            output::info(&format!(
                "Lowered to {} HIR items in {}",
                module.items.len(),
                entry.file_path.display()
            ));
        }

        if !entry.diagnostics.is_empty() {
            type_errors += entry.diagnostics.len();
        }
    }

    if parse_errors > 0 {
        output::error(&format!("{parse_errors} parse error(s) found"));
        return Err(format!("parse error: {parse_errors} diagnostic(s)"));
    }

    if type_errors > 0 {
        output::error(&format!("{type_errors} type error(s) found"));
        return Err(format!("type error: {type_errors} diagnostic(s)"));
    }

    if !allow_effects {
        let mut effectful = Vec::new();
        for entry in analysis.evaluable_modules_in_order() {
            collect_effectful_calls(&entry.module, &entry.method_resolutions, &mut effectful);
        }
        if !effectful.is_empty() {
            for (name, span) in &effectful {
                output::error(&format!(
                    "effect check: effectful call '{name}' at {:?}",
                    span.start
                ));
            }
            output::error(&format!(
                "{} effectful call(s) found (use --allow-effects to permit)",
                effectful.len()
            ));
            return Err("effect check failed".to_string());
        }
    }

    output::success("OK - No errors found");
    Ok(())
}

fn collect_effectful_calls(
    module: &neve_hir::Module,
    method_resolutions: &HashMap<neve_common::Span, neve_hir::DefId>,
    out: &mut Vec<(String, neve_common::Span)>,
) {
    for item in &module.items {
        match &item.kind {
            ItemKind::Fn(fn_def) => walk_expr(&fn_def.body, method_resolutions, out),
            ItemKind::Expr(expr) => walk_expr(expr, method_resolutions, out),
            ItemKind::Impl(impl_def) => {
                for method in &impl_def.items {
                    walk_expr(&method.body, method_resolutions, out);
                }
            }
            _ => {}
        }
    }
}

fn walk_expr(
    expr: &neve_hir::Expr,
    method_resolutions: &HashMap<neve_common::Span, neve_hir::DefId>,
    out: &mut Vec<(String, neve_common::Span)>,
) {
    match &expr.kind {
        ExprKind::Builtin(_) | ExprKind::Global(_) => {}
        ExprKind::Call(func, args) => {
            walk_callee(func, method_resolutions, out);
            for a in args {
                walk_expr(a, method_resolutions, out);
            }
        }
        ExprKind::MethodCall {
            receiver,
            target,
            args,
            ..
        } => {
            walk_expr(receiver, method_resolutions, out);
            if !method_resolutions.contains_key(&expr.span) {
                walk_callee(target, method_resolutions, out);
            }
            for a in args {
                walk_expr(a, method_resolutions, out);
            }
        }
        ExprKind::Binary(_, left, right) => {
            walk_expr(left, method_resolutions, out);
            walk_expr(right, method_resolutions, out);
        }
        ExprKind::Index { base, index } => {
            walk_expr(base, method_resolutions, out);
            walk_expr(index, method_resolutions, out);
        }
        ExprKind::Unary(_, op) => walk_expr(op, method_resolutions, out),
        ExprKind::If(cond, then_body, else_body) => {
            walk_expr(cond, method_resolutions, out);
            walk_expr(then_body, method_resolutions, out);
            walk_expr(else_body, method_resolutions, out);
        }
        ExprKind::Block(stmts, tail) => {
            for s in stmts {
                match &s.kind {
                    StmtKind::Let { value, .. } => walk_expr(value, method_resolutions, out),
                    StmtKind::Expr(e) => walk_expr(e, method_resolutions, out),
                }
            }
            if let Some(e) = tail {
                walk_expr(e, method_resolutions, out);
            }
        }
        ExprKind::Let { value, body, .. } => {
            walk_expr(value, method_resolutions, out);
            walk_expr(body, method_resolutions, out);
        }
        ExprKind::Match(scrutinee, arms) => {
            walk_expr(scrutinee, method_resolutions, out);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    walk_expr(guard, method_resolutions, out);
                }
                walk_expr(&arm.body, method_resolutions, out);
            }
        }
        ExprKind::Field(base, _) => walk_expr(base, method_resolutions, out),
        ExprKind::SafeField { base, .. } => walk_expr(base, method_resolutions, out),
        ExprKind::TupleIndex(base, _) => walk_expr(base, method_resolutions, out),
        ExprKind::Try(inner) => walk_expr(inner, method_resolutions, out),
        ExprKind::Coalesce { value, default } => {
            walk_expr(value, method_resolutions, out);
            walk_expr(default, method_resolutions, out);
        }
        ExprKind::ListComp { body, generators } => {
            walk_expr(body, method_resolutions, out);
            for g in generators {
                walk_expr(&g.iter, method_resolutions, out);
                if let Some(condition) = &g.condition {
                    walk_expr(condition, method_resolutions, out);
                }
            }
        }
        ExprKind::Record(fields) => {
            for (_, v) in fields {
                walk_expr(v, method_resolutions, out);
            }
        }
        ExprKind::List(items) | ExprKind::Tuple(items) => {
            for item in items {
                walk_expr(item, method_resolutions, out);
            }
        }
        ExprKind::Lambda { body, .. } => walk_expr(body, method_resolutions, out),
        ExprKind::Lazy(inner) => walk_expr(inner, method_resolutions, out),
        ExprKind::Interpolated(parts) => {
            for part in parts {
                if let neve_hir::StringPart::Expr(e) = part {
                    walk_expr(e, method_resolutions, out);
                }
            }
        }
        _ => {}
    }
}

fn walk_callee(
    expr: &neve_hir::Expr,
    method_resolutions: &HashMap<neve_common::Span, neve_hir::DefId>,
    out: &mut Vec<(String, neve_common::Span)>,
) {
    match &expr.kind {
        ExprKind::Builtin(name) if is_effectful_builtin(name) => {
            out.push((name.clone(), expr.span));
        }
        ExprKind::Builtin(_) | ExprKind::Global(_) => {}
        _ => walk_expr(expr, method_resolutions, out),
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use tempfile::tempdir;

    fn run_check(source: &str, allow_effects: bool) -> Result<(), String> {
        let dir = tempdir().expect("temporary directory should be created");
        let path = dir.path().join("main.neve");
        std::fs::write(&path, source).expect("fixture should be written");
        run(
            path.to_str().expect("temporary path should be valid UTF-8"),
            false,
            allow_effects,
        )
    }

    #[test]
    fn check_rejects_effectful_call_inside_index() {
        let result = run_check(
            r#"
use std.io = io;
use std.path = path;
let first = io.readDirEntryPaths(path.fromString("/tmp"))[0];
"#,
            false,
        );

        assert_eq!(result, Err("effect check failed".to_string()));
    }

    #[test]
    fn check_allows_pure_method_named_like_effectful_builtin() {
        let result = run_check(
            r#"
trait Reader { fn read(self) -> String; };
impl Reader for String {
    fn read(self) -> String = self;
};
let value = "pure".read();
"#,
            false,
        );

        assert_eq!(result, Ok(()));
    }
    #[test]
    fn check_allows_effectful_builtin_function_value_reference() {
        let result = run_check(
            r#"
use std.io = io;
let reader = io.readFile;
"#,
            false,
        );

        assert_eq!(result, Ok(()));
    }
}
