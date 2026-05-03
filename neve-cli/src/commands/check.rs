//! The `neve check` command.
//! `neve check` 命令。

use crate::{commands::module_graph, output};
use neve_diagnostic::{DiagnosticKind, emit};
use neve_frontend::FrontendDriver;
use neve_hir::{ExprKind, ItemKind, StmtKind};
use neve_std::is_effectful_builtin;
use std::path::Path;

/// Run type checking on a Neve file.
/// 对 Neve 文件运行类型检查。
pub fn run(file: &str, verbose: bool, allow_effects: bool) -> Result<(), String> {
    let path = Path::new(file);
    let (root_dir, module_path) = module_graph::resolve_module_path(path)?;

    let analysis = FrontendDriver::new(&root_dir)
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
        return Err("parse error".to_string());
    }

    if type_errors > 0 {
        output::error(&format!("{type_errors} type error(s) found"));
        return Err("type error".to_string());
    }

    if !allow_effects {
        let mut effectful = Vec::new();
        for entry in analysis.evaluable_modules_in_order() {
            collect_effectful_calls(&entry.module, &mut effectful);
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

fn collect_effectful_calls(module: &neve_hir::Module, out: &mut Vec<(String, neve_common::Span)>) {
    for item in &module.items {
        match &item.kind {
            ItemKind::Fn(fn_def) => walk_expr(&fn_def.body, out),
            ItemKind::Expr(expr) => walk_expr(expr, out),
            ItemKind::Impl(impl_def) => {
                for method in &impl_def.items {
                    walk_expr(&method.body, out);
                }
            }
            _ => {}
        }
    }
}

fn walk_expr(expr: &neve_hir::Expr, out: &mut Vec<(String, neve_common::Span)>) {
    match &expr.kind {
        ExprKind::Builtin(name) if is_effectful_builtin(name) => {
            out.push((name.clone(), expr.span));
        }
        ExprKind::Call(func, args) => {
            walk_expr(func, out);
            for a in args {
                walk_expr(a, out);
            }
        }
        ExprKind::MethodCall {
            receiver,
            target,
            args,
            ..
        } => {
            walk_expr(receiver, out);
            walk_expr(target, out);
            for a in args {
                walk_expr(a, out);
            }
        }
        ExprKind::Binary(_, left, right) => {
            walk_expr(left, out);
            walk_expr(right, out);
        }
        ExprKind::Unary(_, op) => walk_expr(op, out),
        ExprKind::If(cond, then_body, else_body) => {
            walk_expr(cond, out);
            walk_expr(then_body, out);
            walk_expr(else_body, out);
        }
        ExprKind::Block(stmts, tail) => {
            for s in stmts {
                match &s.kind {
                    StmtKind::Let { value, .. } => walk_expr(value, out),
                    StmtKind::Expr(e) => walk_expr(e, out),
                }
            }
            if let Some(e) = tail {
                walk_expr(e, out);
            }
        }
        ExprKind::Let { value, body, .. } => {
            walk_expr(value, out);
            walk_expr(body, out);
        }
        ExprKind::Match(scrutinee, arms) => {
            walk_expr(scrutinee, out);
            for arm in arms {
                walk_expr(&arm.body, out);
            }
        }
        ExprKind::Field(base, _) => walk_expr(base, out),
        ExprKind::SafeField { base, .. } => walk_expr(base, out),
        ExprKind::TupleIndex(base, _) => walk_expr(base, out),
        ExprKind::Try(inner) => walk_expr(inner, out),
        ExprKind::Coalesce { value, default } => {
            walk_expr(value, out);
            walk_expr(default, out);
        }
        ExprKind::ListComp { body, generators } => {
            walk_expr(body, out);
            for g in generators {
                walk_expr(&g.iter, out);
            }
        }
        ExprKind::Record(fields) => {
            for (_, v) in fields {
                walk_expr(v, out);
            }
        }
        ExprKind::List(items) | ExprKind::Tuple(items) => {
            for item in items {
                walk_expr(item, out);
            }
        }
        ExprKind::Lambda(_, body) => walk_expr(body, out),
        ExprKind::Lazy(inner) => walk_expr(inner, out),
        ExprKind::Interpolated(parts) => {
            for part in parts {
                if let neve_hir::StringPart::Expr(e) = part {
                    walk_expr(e, out);
                }
            }
        }
        _ => {}
    }
}
