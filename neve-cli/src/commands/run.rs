//! The `neve run` command.
//! `neve run` 命令。

use crate::{commands::module_graph, output};
use neve_diagnostic::emit;
use neve_eval::{AstEvaluator, EvalError, Value};
use neve_hir::{ModuleId, ModuleLoadError, ModuleLoader};
use neve_parser::parse;
use neve_std::std_module_overrides;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

struct ParsedModule {
    id: ModuleId,
    file_path: PathBuf,
    module_path: Vec<String>,
    ast: neve_syntax::SourceFile,
}

/// Run a Neve file.
/// 运行 Neve 文件。
pub fn run(file: &str, verbose: bool) -> Result<(), String> {
    let path = Path::new(file);
    let (root_dir, module_path) = module_graph::resolve_module_path(path)?;

    let mut loader = ModuleLoader::new(&root_dir);
    let root_id = match loader.load_module(&module_path) {
        Ok(id) => id,
        Err(ModuleLoadError::NotFound(missing)) if is_std_module_path(&missing) => {
            return run_direct(path, &root_dir, &module_path, verbose);
        }
        Err(e) => return Err(format!("module load error: {e}")),
    };

    let mut parse_errors = 0usize;
    let mut parsed_modules = Vec::new();

    for module_id in loader.load_order() {
        let Some(info) = loader.get_module(*module_id) else {
            continue;
        };

        let source = fs::read_to_string(&info.file_path).map_err(|e| {
            format!(
                "cannot read file '{}': {}",
                info.file_path.display(),
                e
            )
        })?;

        // Reuse cached parse diagnostics from the module loader.
        // 复用模块加载器缓存的解析诊断。
        let parse_diagnostics = loader.parsed_diagnostics(*module_id).unwrap_or(&[]);
        for diag in parse_diagnostics {
            emit(&source, &info.file_path.display().to_string(), diag);
        }

        if !parse_diagnostics.is_empty() {
            parse_errors += parse_diagnostics.len();
            continue;
        }

        let Some(ast) = loader.parsed_source(*module_id) else {
            continue;
        };

        if verbose {
            output::info(&format!(
                "Parsed {} items in {}",
                ast.items.len(),
                info.file_path.display()
            ));
        }

        parsed_modules.push(ParsedModule {
            id: *module_id,
            file_path: info.file_path.clone(),
            module_path: info.path.clone(),
            ast: ast.clone(),
        });
    }

    if parse_errors > 0 {
        output::error(&format!("{parse_errors} parse error(s) found"));
        return Err("parse error".to_string());
    }

    let std_overrides = std_module_overrides();
    let mut module_cache = HashMap::new();
    let mut root_value = Value::Unit;

    for parsed in parsed_modules {
        let base_dir = parsed
            .file_path
            .parent()
            .unwrap_or(&root_dir)
            .to_path_buf();

        let mut evaluator = AstEvaluator::new()
            .with_module_overrides(std_overrides.clone())
            .with_base_path(base_dir)
            .with_module_loader(ModuleLoader::new(&root_dir))
            .with_module_path(parsed.module_path.clone())
            .with_loaded_modules(module_cache);

        let value = match evaluator.eval_file(&parsed.ast) {
            Ok(value) => value,
            Err(EvalError::ParseDiagnostics {
                path,
                source_text,
                diagnostics,
                ..
            }) => {
                for diag in diagnostics {
                    emit(&source_text, &path.display().to_string(), &diag);
                }
                return Err("parse error".to_string());
            }
            Err(e) => return Err(format!("evaluation error: {e:?}")),
        };

        let module_env = evaluator.env();
        let mut new_cache = evaluator.into_loaded_modules();
        new_cache.insert(parsed.file_path.clone(), module_env);
        module_cache = new_cache;

        if parsed.id == root_id {
            root_value = value;
        }
    }

    if !matches!(root_value, Value::Unit) {
        output::success(&format!("{root_value:?}"));
    }

    Ok(())
}

fn run_direct(
    file: &Path,
    root_dir: &Path,
    module_path: &[String],
    _verbose: bool,
) -> Result<(), String> {
    let source = fs::read_to_string(file)
        .map_err(|e| format!("cannot read file '{}': {}", file.display(), e))?;

    let (ast, diagnostics) = parse(&source);
    for diag in &diagnostics {
        emit(&source, &file.display().to_string(), diag);
    }

    if !diagnostics.is_empty() {
        return Err("parse error".to_string());
    }

    let base_dir = file.parent().unwrap_or(root_dir).to_path_buf();
    let mut evaluator = AstEvaluator::new()
        .with_module_overrides(std_module_overrides())
        .with_base_path(base_dir)
        .with_module_loader(ModuleLoader::new(root_dir))
        .with_module_path(module_path.to_vec());

    let value = match evaluator.eval_file(&ast) {
        Ok(value) => value,
        Err(EvalError::ParseDiagnostics {
            path,
            source_text,
            diagnostics,
            ..
        }) => {
            for diag in diagnostics {
                emit(&source_text, &path.display().to_string(), &diag);
            }
            return Err("parse error".to_string());
        }
        Err(e) => return Err(format!("evaluation error: {e:?}")),
    };

    if !matches!(value, Value::Unit) {
        output::success(&format!("{value:?}"));
    }

    Ok(())
}

fn is_std_module_path(path: &[String]) -> bool {
    path.first().map(|seg| seg == "std").unwrap_or(false)
}
