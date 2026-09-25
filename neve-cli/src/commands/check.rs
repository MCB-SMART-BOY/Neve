//! The `neve check` command.
//! `neve check` 命令。

use crate::{commands::module_graph, output};
use neve_diagnostic::{DiagnosticKind, emit};
use neve_frontend::FrontendDriver;
use neve_hir::{ExprKind, ItemKind, StmtKind};
use neve_std::is_effectful_builtin;
use std::collections::{HashMap, HashSet};
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
            let Some(semantics) = analysis.semantics(entry.module_id) else {
                continue;
            };
            collect_effectful_calls(
                &entry.module,
                &entry.method_resolutions,
                &semantics.effectful_definitions,
                &mut effectful,
            );
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

struct EffectContext<'a> {
    method_resolutions: &'a HashMap<neve_common::Span, neve_hir::DefId>,
    effectful_definitions: &'a HashSet<neve_hir::DefId>,
}

fn collect_effectful_calls(
    module: &neve_hir::Module,
    method_resolutions: &HashMap<neve_common::Span, neve_hir::DefId>,
    effectful_definitions: &HashSet<neve_hir::DefId>,
    out: &mut Vec<(String, neve_common::Span)>,
) {
    let context = EffectContext {
        method_resolutions,
        effectful_definitions,
    };

    for item in &module.items {
        match &item.kind {
            ItemKind::Fn(fn_def) => walk_expr(&fn_def.body, &context, out),
            ItemKind::Expr(expr) => walk_expr(expr, &context, out),
            ItemKind::Impl(impl_def) => {
                for method in &impl_def.items {
                    walk_expr(&method.body, &context, out);
                }
            }
            _ => {}
        }
    }
}

fn walk_expr(
    expr: &neve_hir::Expr,
    context: &EffectContext<'_>,
    out: &mut Vec<(String, neve_common::Span)>,
) {
    match &expr.kind {
        ExprKind::Builtin(_) | ExprKind::Global(_) => {}
        ExprKind::Call(func, args) => {
            walk_callee(func, context, out);
            for a in args {
                walk_expr(a, context, out);
            }
        }
        ExprKind::MethodCall {
            receiver,
            target,
            args,
            ..
        } => {
            walk_expr(receiver, context, out);
            if let Some(def_id) = context.method_resolutions.get(&expr.span) {
                if context.effectful_definitions.contains(def_id) {
                    out.push(("effectful method".to_string(), expr.span));
                }
            } else {
                walk_callee(target, context, out);
            }
            for a in args {
                walk_expr(a, context, out);
            }
        }
        ExprKind::Binary(_, left, right) => {
            walk_expr(left, context, out);
            walk_expr(right, context, out);
        }
        ExprKind::Index { base, index } => {
            walk_expr(base, context, out);
            walk_expr(index, context, out);
        }
        ExprKind::Unary(_, op) => walk_expr(op, context, out),
        ExprKind::If(cond, then_body, else_body) => {
            walk_expr(cond, context, out);
            walk_expr(then_body, context, out);
            walk_expr(else_body, context, out);
        }
        ExprKind::Block(stmts, tail) => {
            for s in stmts {
                match &s.kind {
                    StmtKind::Let { value, .. } => walk_expr(value, context, out),
                    StmtKind::Expr(e) => walk_expr(e, context, out),
                }
            }
            if let Some(e) = tail {
                walk_expr(e, context, out);
            }
        }
        ExprKind::Let { value, body, .. } => {
            walk_expr(value, context, out);
            walk_expr(body, context, out);
        }
        ExprKind::Match(scrutinee, arms) => {
            walk_expr(scrutinee, context, out);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    walk_expr(guard, context, out);
                }
                walk_expr(&arm.body, context, out);
            }
        }
        ExprKind::Field(base, _) => walk_expr(base, context, out),
        ExprKind::SafeField { base, .. } => walk_expr(base, context, out),
        ExprKind::TupleIndex(base, _) => walk_expr(base, context, out),
        ExprKind::Try(inner) => walk_expr(inner, context, out),
        ExprKind::Coalesce { value, default } => {
            walk_expr(value, context, out);
            walk_expr(default, context, out);
        }
        ExprKind::ListComp { body, generators } => {
            walk_expr(body, context, out);
            for g in generators {
                walk_expr(&g.iter, context, out);
                if let Some(condition) = &g.condition {
                    walk_expr(condition, context, out);
                }
            }
        }
        ExprKind::Record(fields) => {
            for (_, v) in fields {
                walk_expr(v, context, out);
            }
        }
        ExprKind::List(items) | ExprKind::Tuple(items) => {
            for item in items {
                walk_expr(item, context, out);
            }
        }
        ExprKind::Lambda { body, .. } => walk_expr(body, context, out),
        ExprKind::Lazy(inner) => walk_expr(inner, context, out),
        ExprKind::Interpolated(parts) => {
            for part in parts {
                if let neve_hir::StringPart::Expr(e) = part {
                    walk_expr(e, context, out);
                }
            }
        }
        _ => {}
    }
}

fn walk_callee(
    expr: &neve_hir::Expr,
    context: &EffectContext<'_>,
    out: &mut Vec<(String, neve_common::Span)>,
) {
    match &expr.kind {
        ExprKind::Builtin(name) if is_effectful_builtin(name) => {
            out.push((name.clone(), expr.span));
        }
        ExprKind::Global(def_id) if context.effectful_definitions.contains(def_id) => {
            out.push(("effectful function".to_string(), expr.span));
        }
        ExprKind::Builtin(_) | ExprKind::Global(_) => {}
        _ => walk_expr(expr, context, out),
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

    #[test]
    fn check_rejects_imported_effectful_function() {
        let dir = tempdir().expect("temporary directory should be created");
        let dependency = dir.path().join("dependency.neve");
        let main = dir.path().join("main.neve");
        std::fs::write(
            &dependency,
            "use std.io = io;\nfn read_file(path: String) -> String = io.readFile(path);\n",
        )
        .expect("dependency fixture should be written");
        std::fs::write(
            &main,
            "use dependency (read_file);\nfn outer(path: String) -> String = read_file(path);\n",
        )
        .expect("main fixture should be written");

        let result = run(
            main.to_str().expect("temporary path should be valid UTF-8"),
            false,
            false,
        );

        assert_eq!(result, Err("effect check failed".to_string()));
    }
}
