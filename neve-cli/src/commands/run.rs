//! The `neve run` command — canonical HIR pipeline.

use crate::{
    commands::{diagnostics, module_graph},
    output,
};
use neve_eval::{EvaluableModuleRef, Evaluator, Value, eval_error_to_diagnostic};
use neve_frontend::{FrontendDriver, ProgramAnalysis, ProgramParsedModule};
use neve_std::stdlib;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[cfg(unix)]
fn discover_flake_input_roots(start_dir: &Path) -> HashMap<String, PathBuf> {
    let mut current = start_dir.to_path_buf();
    while current.parent().is_some() {
        let flake_path = current.join("flake.neve");
        if flake_path.exists() {
            match neve_config::flake::Flake::load(&current) {
                Ok(mut flake) => {
                    if let Err(e) = flake.lock_inputs() {
                        output::warning(&format!("failed to lock flake inputs: {e}"));
                    }
                    return flake.collect_input_roots().unwrap_or_default();
                }
                Err(e) => {
                    output::warning(&format!(
                        "failed to load flake.neve at {}: {e}",
                        current.display()
                    ));
                }
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

/// Run a Neve source file through the canonical HIR pipeline and print the result.
/// 通过 canonical HIR 管线运行 Neve 源文件并打印结果。
pub fn run(file: &str, verbose: bool) -> Result<(), String> {
    let path = Path::new(file);
    let value = run_value(path, verbose)?;
    if !matches!(value, Value::Unit) {
        output::success(&format!("{value:?}"));
    }
    Ok(())
}

pub(crate) fn run_value(file: &Path, verbose: bool) -> Result<Value, String> {
    let (root_dir, module_path) = module_graph::resolve_module_path(file)?;
    let flake_inputs = discover_flake_input_roots(&root_dir);
    let mut driver = FrontendDriver::new(&root_dir);
    if !flake_inputs.is_empty() {
        driver = driver.with_flake_inputs(flake_inputs);
    }

    let analysis = driver
        .analyze_module_path(&module_path)
        .map_err(|e| format!("frontend error: {e}"))?;

    if verbose {
        let parsed = analysis.parsed_modules_in_order();
        emit_program_parse_summary(&parsed, &root_dir);
    }
    emit_program_parse_diagnostics(&analysis)?;
    eval_modules_via_hir(&analysis)
}

fn emit_program_parse_diagnostics(analysis: &ProgramAnalysis) -> Result<(), String> {
    let stats = analysis.diagnostic_stats();
    diagnostics::emit_program_diagnostic_entries(&analysis.parser_diagnostic_modules_in_order());
    if stats.parse_errors > 0 {
        return Err(format!("{} parse error(s) found", stats.parse_errors));
    }
    Ok(())
}

fn emit_program_parse_summary(parsed_modules: &[ProgramParsedModule], root_dir: &Path) {
    for parsed in parsed_modules {
        let form_count = parsed.ast.items.len() + usize::from(parsed.ast.tail_expr.is_some());
        let display_path = parsed
            .file_path
            .strip_prefix(root_dir)
            .unwrap_or(&parsed.file_path)
            .display()
            .to_string();
        output::info(&format!(
            "Parsed {form_count} top-level form(s) in {display_path}"
        ));
    }
}

fn eval_modules_via_hir(analysis: &ProgramAnalysis) -> Result<Value, String> {
    let stats = analysis.diagnostic_stats();
    let mut evaluator = Evaluator::new().with_extra_builtins(
        stdlib()
            .into_iter()
            .map(|(name, value)| (name.to_string(), value)),
    );

    diagnostics::emit_program_diagnostic_entries(&analysis.diagnostic_modules_in_order());

    if stats.has_errors() {
        return Err("type error(s) found".to_string());
    }

    let modules = analysis.evaluable_modules_in_order();
    evaluator
        .eval_evaluable_modules(
            modules
                .iter()
                .map(|entry| EvaluableModuleRef::new(&entry.module, &entry.method_resolutions)),
            analysis.root_module_id(),
        )
        .map_err(|e| {
            let diag = eval_error_to_diagnostic(&e, neve_common::Span::DUMMY);
            neve_diagnostic::emit("", "<runtime>", &diag);
            format!("eval error: {}", e)
        })
}
