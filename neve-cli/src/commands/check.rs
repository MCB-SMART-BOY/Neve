//! The `neve check` command.
//! `neve check` 命令。

use crate::{commands::module_graph, output};
use neve_diagnostic::emit;
use neve_frontend::FrontendDriver;
use std::fs;
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

    for module_id in analysis.load_order() {
        let Some(module) = analysis.hir_module(*module_id) else {
            continue;
        };

        let Some(info) = analysis.module_info(*module_id) else {
            continue;
        };

        let file_path = &info.file_path;
        let source = fs::read_to_string(file_path)
            .map_err(|e| format!("cannot read file '{}': {}", file_path.display(), e))?;

        // Reuse cached parse diagnostics from the module loader.
        // 复用模块加载器缓存的解析诊断。
        let parse_diagnostics = analysis.parsed_diagnostics(*module_id).unwrap_or(&[]);
        let diagnostics = analysis.diagnostics(*module_id).unwrap_or(&[]);
        for diag in diagnostics {
            emit(&source, &file_path.display().to_string(), diag);
        }

        if !parse_diagnostics.is_empty() {
            parse_errors += parse_diagnostics.len();
            continue;
        }

        if verbose && let Some(ast) = analysis.parsed_source(*module_id) {
            let form_count = ast.items.len() + usize::from(ast.tail_expr.is_some());
            output::info(&format!(
                "Parsed {} top-level form(s) in {}",
                form_count,
                file_path.display()
            ));
        }

        if verbose {
            output::info(&format!(
                "Lowered to {} HIR items in {}",
                module.items.len(),
                file_path.display()
            ));
        }

        if !diagnostics.is_empty() {
            type_errors += diagnostics.len();
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
