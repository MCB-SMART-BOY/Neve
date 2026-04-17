//! The `neve check` command.
//! `neve check` 命令。

use crate::{commands::module_graph, output};
use neve_diagnostic::{DiagnosticKind, emit};
use neve_frontend::FrontendDriver;
use std::path::Path;

/// Run type checking on a Neve file.
/// 对 Neve 文件运行类型检查。
pub fn run(file: &str, verbose: bool) -> Result<(), String> {
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

    output::success("OK - No errors found");
    Ok(())
}
