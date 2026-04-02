//! The `neve check` command.
//! `neve check` 命令。

use crate::{commands::module_graph, output};
use neve_diagnostic::emit;
use neve_frontend::rewrite_diagnostics_with_module_names;
use neve_hir::ModuleLoader;
use neve_typeck::TypeChecker;
use std::fs;
use std::path::Path;

/// Run type checking on a Neve file.
/// 对 Neve 文件运行类型检查。
pub fn run(file: &str, verbose: bool) -> Result<(), String> {
    let path = Path::new(file);
    let (root_dir, module_path) = module_graph::resolve_module_path(path)?;

    let mut loader = ModuleLoader::new(&root_dir);
    loader
        .load_module(&module_path)
        .map_err(|e| format!("module load error: {e}"))?;

    // Collect global signatures from all loaded modules
    // 收集所有已加载模块的全局签名
    let mut global_types = std::collections::HashMap::new();
    let mut global_spans = std::collections::HashMap::new();
    for module_id in loader.load_order() {
        if let Some(module) = loader.hir_module(*module_id) {
            let (types, spans) = TypeChecker::collect_signatures(module);
            global_types.extend(types);
            global_spans.extend(spans);
        }
    }

    let mut parse_errors = 0usize;
    let mut type_errors = 0usize;

    for module_id in loader.load_order() {
        let Some(module) = loader.hir_module(*module_id) else {
            continue;
        };

        let Some(info) = loader.get_module(*module_id) else {
            continue;
        };

        let file_path = &info.file_path;
        let source = fs::read_to_string(file_path)
            .map_err(|e| format!("cannot read file '{}': {}", file_path.display(), e))?;

        // Reuse cached parse diagnostics from the module loader.
        // 复用模块加载器缓存的解析诊断。
        let parse_diagnostics = loader.parsed_diagnostics(*module_id).unwrap_or(&[]);
        for diag in parse_diagnostics {
            emit(&source, &file_path.display().to_string(), diag);
        }

        if !parse_diagnostics.is_empty() {
            parse_errors += parse_diagnostics.len();
            continue;
        }

        if verbose && let Some(ast) = loader.parsed_source(*module_id) {
            output::info(&format!(
                "Parsed {} items in {}",
                ast.items.len(),
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

        // Type check with shared globals
        // 使用共享全局签名进行类型检查
        let mut checker = TypeChecker::with_global_env(global_types.clone(), global_spans.clone());
        checker.check(module);
        let diagnostics = rewrite_diagnostics_with_module_names(checker.diagnostics(), module);

        for diag in &diagnostics {
            emit(&source, &file_path.display().to_string(), diag);
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
