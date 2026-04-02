//! The `neve run` command.
//! `neve run` 命令。

use crate::{commands::module_graph, output};
use neve_diagnostic::{Severity, emit};
use neve_eval::{AstEvaluator, EvalError, Evaluator, Value};
use neve_frontend::rewrite_diagnostics_with_module_names;
use neve_hir::{ModuleId, ModuleLoadError, ModuleLoader};
use neve_std::{std_module_overrides, stdlib};
use neve_syntax::{ImportDef, ItemKind, SourceFile};
use neve_typeck::TypeChecker;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct ParsedModule {
    id: ModuleId,
    file_path: PathBuf,
    module_path: Vec<String>,
    ast: SourceFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunBackend {
    FrontendHir,
    AstFallback,
}

/// Run a Neve file.
/// 运行 Neve 文件。
pub fn run(file: &str, verbose: bool) -> Result<(), String> {
    let path = Path::new(file);
    let (backend, value) = run_value(path, verbose)?;

    if verbose {
        output::info(match backend {
            RunBackend::FrontendHir => "Run backend: frontend/HIR",
            RunBackend::AstFallback => "Run backend: AST fallback",
        });
    }

    if !matches!(value, Value::Unit) {
        output::success(&format!("{value:?}"));
    }

    Ok(())
}

pub(crate) fn run_value(file: &Path, verbose: bool) -> Result<(RunBackend, Value), String> {
    let (root_dir, module_path) = module_graph::resolve_module_path(file)?;

    let mut loader = ModuleLoader::new(&root_dir);
    let root_id = match loader.load_module(&module_path) {
        Ok(id) => id,
        Err(ModuleLoadError::NotFound(missing)) if is_std_module_path(&missing) => {
            return run_direct_value(file, &root_dir, &module_path, verbose);
        }
        Err(e) => return Err(format!("module load error: {e}")),
    };

    let parsed_modules = collect_parsed_modules(&loader, &root_dir, verbose)?;
    if parsed_modules
        .iter()
        .any(|module| source_file_requires_ast_fallback(&module.ast))
    {
        let value = eval_modules_via_ast(&parsed_modules, root_id, &root_dir)?;
        return Ok((RunBackend::AstFallback, value));
    }

    let value = eval_modules_via_hir(&loader, root_id)?;
    Ok((RunBackend::FrontendHir, value))
}

fn collect_parsed_modules(
    loader: &ModuleLoader,
    root_dir: &Path,
    verbose: bool,
) -> Result<Vec<ParsedModule>, String> {
    let mut parse_errors = 0usize;
    let mut parsed_modules = Vec::new();

    for module_id in loader.load_order() {
        let Some(info) = loader.get_module(*module_id) else {
            continue;
        };

        let source = fs::read_to_string(&info.file_path)
            .map_err(|e| format!("cannot read file '{}': {}", info.file_path.display(), e))?;

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
            let display_path = info
                .file_path
                .strip_prefix(root_dir)
                .unwrap_or(&info.file_path)
                .display()
                .to_string();
            output::info(&format!(
                "Parsed {} items in {}",
                ast.items.len(),
                display_path
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

    Ok(parsed_modules)
}

fn eval_modules_via_ast(
    parsed_modules: &[ParsedModule],
    root_id: ModuleId,
    root_dir: &Path,
) -> Result<Value, String> {
    let std_overrides = std_module_overrides();
    let mut module_cache = HashMap::new();
    let mut root_value = Value::Unit;

    for parsed in parsed_modules {
        let base_dir = parsed.file_path.parent().unwrap_or(root_dir).to_path_buf();

        let mut evaluator = AstEvaluator::new()
            .with_module_overrides(std_overrides.clone())
            .with_base_path(base_dir)
            .with_module_loader(ModuleLoader::new(root_dir))
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

    Ok(root_value)
}

fn eval_modules_via_hir(loader: &ModuleLoader, root_id: ModuleId) -> Result<Value, String> {
    let mut global_types = HashMap::new();
    let mut global_spans = HashMap::new();

    for module_id in loader.load_order() {
        let Some(module) = loader.hir_module(*module_id) else {
            continue;
        };
        let (types, spans) = TypeChecker::collect_signatures(module);
        global_types.extend(types);
        global_spans.extend(spans);
    }

    let mut had_errors = false;
    let mut module_method_resolutions = HashMap::new();

    for module_id in loader.load_order() {
        let Some(module) = loader.hir_module(*module_id) else {
            continue;
        };
        let Some(info) = loader.get_module(*module_id) else {
            continue;
        };

        let source = fs::read_to_string(&info.file_path)
            .map_err(|e| format!("cannot read file '{}': {}", info.file_path.display(), e))?;

        let mut checker = TypeChecker::with_global_env(global_types.clone(), global_spans.clone());
        checker.check(module);
        let method_resolutions = checker.method_resolutions().clone();
        let diagnostics = rewrite_diagnostics_with_module_names(checker.diagnostics(), module);

        for diag in diagnostics {
            emit(&source, &info.file_path.display().to_string(), &diag);
            if diag.severity == Severity::Error {
                had_errors = true;
            }
        }

        module_method_resolutions.insert(*module_id, method_resolutions);
    }

    if had_errors {
        return Err("type error".to_string());
    }

    let mut evaluator = Evaluator::new().with_extra_builtins(std_builtin_values());
    let mut root_value = Value::Unit;

    for module_id in loader.load_order() {
        let Some(module) = loader.hir_module(*module_id) else {
            continue;
        };

        evaluator.set_method_resolutions(
            module_method_resolutions
                .remove(module_id)
                .unwrap_or_default(),
        );

        let value = evaluator
            .eval_module(module)
            .map_err(|e| format!("evaluation error: {e:?}"))?;

        if *module_id == root_id {
            root_value = value;
        }
    }

    Ok(root_value)
}

fn run_direct_value(
    file: &Path,
    root_dir: &Path,
    module_path: &[String],
    _verbose: bool,
) -> Result<(RunBackend, Value), String> {
    let source = fs::read_to_string(file)
        .map_err(|e| format!("cannot read file '{}': {}", file.display(), e))?;

    let (ast, diagnostics) = neve_parser::parse(&source);
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

    Ok((RunBackend::AstFallback, value))
}

fn source_file_requires_ast_fallback(file: &SourceFile) -> bool {
    file.items.iter().any(|item| match &item.kind {
        ItemKind::Import(import) => import_requires_ast_fallback(import),
        _ => false,
    })
}

fn import_requires_ast_fallback(import: &ImportDef) -> bool {
    if !is_std_import(import) {
        return false;
    }

    !is_supported_std_import(import)
}

fn is_supported_std_import(import: &ImportDef) -> bool {
    import.path.len() == 2
        && import.path.first().map(|segment| segment.name.as_str()) == Some("std")
}

fn is_std_import(import: &ImportDef) -> bool {
    import.path.first().map(|segment| segment.name.as_str()) == Some("std")
}

fn std_builtin_values() -> impl Iterator<Item = (String, Value)> {
    stdlib()
        .into_iter()
        .map(|(name, value)| (name.to_string(), value))
}

fn is_std_module_path(path: &[String]) -> bool {
    path.first().map(|seg| seg == "std").unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{RunBackend, run_value};
    use neve_eval::Value;
    use std::fs;
    use tempfile::TempDir;

    fn write_module(dir: &std::path::Path, path: &[&str], content: &str) {
        let mut full_path = dir.to_path_buf();
        for (index, segment) in path.iter().enumerate() {
            full_path.push(segment);
            if index < path.len() - 1 {
                fs::create_dir_all(&full_path).unwrap();
            }
        }
        full_path.set_extension("neve");
        fs::write(full_path, content).unwrap();
    }

    #[test]
    fn run_value_prefers_frontend_hir_without_std_imports() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        write_module(root, &["math"], "pub fn add(x, y) = x + y;");
        write_module(
            root,
            &["main"],
            "import math (add); let result = add(1, 2);",
        );

        let (backend, value) = run_value(&root.join("main.neve"), false).unwrap();
        assert_eq!(backend, RunBackend::FrontendHir);
        assert_eq!(value, Value::Int(3.into()));
    }

    #[test]
    fn run_value_prefers_frontend_hir_for_std_item_imports() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        write_module(
            root,
            &["main"],
            "import std.list (len); let result = len([1, 2]);",
        );

        let (backend, value) = run_value(&root.join("main.neve"), false).unwrap();
        assert_eq!(backend, RunBackend::FrontendHir);
        assert_eq!(value, Value::Int(2.into()));
    }

    #[test]
    fn run_value_prefers_frontend_hir_for_std_module_imports() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        write_module(
            root,
            &["main"],
            "import std.list; let result = list.len([1, 2, 3]);",
        );

        let (backend, value) = run_value(&root.join("main.neve"), false).unwrap();
        assert_eq!(backend, RunBackend::FrontendHir);
        assert_eq!(value, Value::Int(3.into()));
    }

    #[test]
    fn run_value_prefers_frontend_hir_for_std_glob_imports() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        write_module(
            root,
            &["main"],
            "import std.list (*); let result = len([1, 2]);",
        );

        let (backend, value) = run_value(&root.join("main.neve"), false).unwrap();
        assert_eq!(backend, RunBackend::FrontendHir);
        assert_eq!(value, Value::Int(2.into()));
    }
}
