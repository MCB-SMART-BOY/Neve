//! The `neve run` command.
//! `neve run` 命令。

use crate::{commands::module_graph, output};
use neve_diagnostic::{Severity, emit};
use neve_eval::{
    EvalError, Evaluator, Value,
    compat::{AstEnv, AstEvaluator},
};
use neve_frontend::{
    FrontendDriver, FrontendError, ProgramAnalysis, ProgramParsedModule, parse_module_file,
};
use neve_hir::{ModuleId, ModuleLoadError, ModuleLoader, supports_canonical_std_import};
use neve_std::{std_module_overrides, stdlib};
use neve_syntax::{ImportDef, ItemKind, SourceFile};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunBackend {
    FrontendHir,
    AstCompat,
}

type AstCompatModuleOverrides = HashMap<Vec<String>, Rc<AstEnv>>;
type LoadedAstModules = HashMap<PathBuf, Rc<AstEnv>>;

/// Explicit AST-compat runner for one command-scoped program evaluation.
/// 单个命令作用域程序求值使用的显式 AST 兼容 runner。
struct AstCompatProgramRunner {
    root_dir: PathBuf,
    module_overrides: AstCompatModuleOverrides,
    loaded_modules: LoadedAstModules,
}

impl AstCompatProgramRunner {
    /// Create a new AST-compat runner rooted at one project directory.
    /// 创建一个以项目目录为根的 AST 兼容 runner。
    fn new(root_dir: &Path, module_overrides: AstCompatModuleOverrides) -> Self {
        Self {
            root_dir: root_dir.to_path_buf(),
            module_overrides,
            loaded_modules: HashMap::new(),
        }
    }

    /// Evaluate one parse-clean module and update the compat module cache.
    /// 求值一个解析成功的模块，并更新兼容模块缓存。
    fn eval_module(
        &mut self,
        file_path: &Path,
        module_path: &[String],
        ast: &SourceFile,
    ) -> Result<Value, String> {
        let base_dir = file_path
            .parent()
            .unwrap_or(self.root_dir.as_path())
            .to_path_buf();

        let mut evaluator = ast_compat_evaluator(
            &self.root_dir,
            &base_dir,
            module_path.to_vec(),
            self.module_overrides.clone(),
            std::mem::take(&mut self.loaded_modules),
        );
        let value = eval_ast_compat_module(&mut evaluator, ast)?;

        let module_env = evaluator.env();
        let mut loaded_modules = evaluator.into_loaded_modules();
        loaded_modules.insert(file_path.to_path_buf(), module_env);
        self.loaded_modules = loaded_modules;

        Ok(value)
    }
}

/// Private execution plan for one `neve run` invocation.
/// 单次 `neve run` 调用的私有执行计划。
enum RunExecutionPlan {
    /// Evaluate through the canonical frontend/HIR path.
    /// 通过规范 frontend/HIR 路径求值。
    FrontendHir {
        analysis: Box<ProgramAnalysis>,
        parsed_modules: Vec<ProgramParsedModule>,
    },
    /// Evaluate a parse-clean program through the explicit AST-compat path.
    /// 通过显式 AST 兼容路径求值一个解析成功的程序。
    AstCompatProgram {
        parsed_modules: Vec<ProgramParsedModule>,
        root_id: ModuleId,
    },
    /// Evaluate one direct std root file through the explicit AST-compat path.
    /// 通过显式 AST 兼容路径求值单个 direct std 根文件。
    AstCompatDirectFile { module_path: Vec<String> },
}

/// Result of executing one private `neve run` plan.
/// 执行单个私有 `neve run` 计划后的结果。
struct RunExecutionResult {
    backend: RunBackend,
    value: Value,
}

/// Run a Neve file.
/// 运行 Neve 文件。
pub fn run(file: &str, verbose: bool, compat_ast: bool) -> Result<(), String> {
    let path = Path::new(file);
    let (backend, value) = run_value(path, verbose, compat_ast)?;

    if verbose {
        output::info(match backend {
            RunBackend::FrontendHir => "Run backend: frontend/HIR",
            RunBackend::AstCompat => "Run backend: AST compat",
        });
    }

    if !matches!(value, Value::Unit) {
        output::success(&format!("{value:?}"));
    }

    Ok(())
}

pub(crate) fn run_value(
    file: &Path,
    verbose: bool,
    compat_ast: bool,
) -> Result<(RunBackend, Value), String> {
    let (root_dir, module_path) = module_graph::resolve_module_path(file)?;
    let result = execute_run_plan(
        plan_run_execution(&root_dir, &module_path, compat_ast)?,
        file,
        &root_dir,
        verbose,
    )?;
    Ok((result.backend, result.value))
}

/// Execute one private `neve run` plan and package the backend/value result.
/// 执行单个私有 `neve run` 计划并封装 backend/value 结果。
fn execute_run_plan(
    plan: RunExecutionPlan,
    file: &Path,
    root_dir: &Path,
    verbose: bool,
) -> Result<RunExecutionResult, String> {
    match plan {
        RunExecutionPlan::FrontendHir {
            analysis,
            parsed_modules,
        } => {
            if verbose {
                emit_program_parse_summary(&parsed_modules, root_dir);
            }
            let value = eval_modules_via_hir(&analysis)?;
            Ok(RunExecutionResult {
                backend: RunBackend::FrontendHir,
                value,
            })
        }
        RunExecutionPlan::AstCompatProgram {
            parsed_modules,
            root_id,
        } => {
            if verbose {
                emit_program_parse_summary(&parsed_modules, root_dir);
            }
            let value = eval_modules_via_ast(&parsed_modules, root_id, root_dir)?;
            Ok(RunExecutionResult {
                backend: RunBackend::AstCompat,
                value,
            })
        }
        RunExecutionPlan::AstCompatDirectFile { module_path } => {
            let value = run_direct_value(file, root_dir, &module_path, verbose)?;
            Ok(RunExecutionResult {
                backend: RunBackend::AstCompat,
                value,
            })
        }
    }
}

/// Decide which execution path should own one `neve run` request.
/// 决定单次 `neve run` 请求应由哪条执行路径处理。
fn plan_run_execution(
    root_dir: &Path,
    module_path: &[String],
    compat_ast: bool,
) -> Result<RunExecutionPlan, String> {
    let analysis = match FrontendDriver::new(root_dir).analyze_module_path(module_path) {
        Ok(analysis) => analysis,
        Err(FrontendError::ModuleLoad(ModuleLoadError::NotFound(missing)))
            if is_std_module_path(&missing) =>
        {
            if !compat_ast {
                return Err(
                    "frontend/HIR cannot run direct std modules yet; rerun with --compat-ast"
                        .to_string(),
                );
            }
            return Ok(RunExecutionPlan::AstCompatDirectFile {
                module_path: module_path.to_vec(),
            });
        }
        Err(e) => return Err(format!("frontend error: {e}")),
    };

    emit_program_parse_diagnostics(&analysis)?;
    let parsed_modules = analysis.parsed_modules_in_order();
    if parsed_modules
        .iter()
        .any(|module| source_file_requires_ast_fallback(&module.ast))
    {
        if !compat_ast {
            return Err(
                "frontend/HIR cannot run this std import shape yet; rerun with --compat-ast"
                    .to_string(),
            );
        }
        return Ok(RunExecutionPlan::AstCompatProgram {
            parsed_modules,
            root_id: analysis.root_module_id(),
        });
    }

    Ok(RunExecutionPlan::FrontendHir {
        analysis: Box::new(analysis),
        parsed_modules,
    })
}

fn emit_program_parse_diagnostics(analysis: &ProgramAnalysis) -> Result<(), String> {
    let mut parse_errors = 0usize;

    for entry in analysis.diagnostic_modules_in_order() {
        for diag in entry
            .diagnostics
            .iter()
            .filter(|diag| diag.kind == neve_diagnostic::DiagnosticKind::Parser)
        {
            emit(&entry.source, &entry.file_path.display().to_string(), diag);
            parse_errors += 1;
        }
    }

    if parse_errors > 0 {
        output::error(&format!("{parse_errors} parse error(s) found"));
        return Err("parse error".to_string());
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
    let mut had_errors = false;
    let mut evaluator = Evaluator::new().with_extra_builtins(std_builtin_values());
    let mut root_value = Value::Unit;

    for entry in analysis.diagnostic_modules_in_order() {
        for diag in &entry.diagnostics {
            emit(&entry.source, &entry.file_path.display().to_string(), diag);
            if diag.severity == Severity::Error {
                had_errors = true;
            }
        }
    }

    if had_errors {
        return Err("type error".to_string());
    }

    for entry in analysis.evaluable_modules_in_order() {
        let value = evaluator
            .eval_module_with_method_resolutions(&entry.module, &entry.method_resolutions)
            .map_err(|e| format!("evaluation error: {e:?}"))?;

        if entry.module_id == analysis.root_module_id() {
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
) -> Result<Value, String> {
    let parsed = parse_module_file(file, module_path)
        .map_err(|e| format!("cannot read file '{}': {}", file.display(), e))?;
    for diag in &parsed.diagnostics {
        emit(
            &parsed.source,
            &parsed.file_path.display().to_string(),
            diag,
        );
    }

    let Some(ast) = parsed.ast else {
        return Err("parse error".to_string());
    };

    let mut runner = AstCompatProgramRunner::new(root_dir, std_module_overrides());
    let value = runner.eval_module(file, &parsed.module_path, &ast)?;

    Ok(value)
}

fn eval_modules_via_ast(
    parsed_modules: &[ProgramParsedModule],
    root_id: ModuleId,
    root_dir: &Path,
) -> Result<Value, String> {
    let mut runner = AstCompatProgramRunner::new(root_dir, std_module_overrides());
    let mut root_value = Value::Unit;

    for parsed in parsed_modules {
        let value = runner.eval_module(&parsed.file_path, &parsed.module_path, &parsed.ast)?;

        if parsed.module_id == root_id {
            root_value = value;
        }
    }

    Ok(root_value)
}

/// Build one explicit AST-compat evaluator against the given module context.
/// 基于给定模块上下文构建一个显式 AST 兼容求值器。
fn ast_compat_evaluator(
    root_dir: &Path,
    base_dir: &Path,
    module_path: Vec<String>,
    module_overrides: AstCompatModuleOverrides,
    loaded_modules: LoadedAstModules,
) -> AstEvaluator {
    AstEvaluator::new()
        .with_module_overrides(module_overrides)
        .with_base_path(base_dir.to_path_buf())
        .with_module_loader(ModuleLoader::new(root_dir))
        .with_module_path(module_path)
        .with_loaded_modules(loaded_modules)
}

/// Evaluate one AST module through the explicit AST-compat path.
/// 通过显式 AST 兼容路径求值单个 AST 模块。
fn eval_ast_compat_module(evaluator: &mut AstEvaluator, ast: &SourceFile) -> Result<Value, String> {
    match evaluator.eval_file(ast) {
        Ok(value) => Ok(value),
        Err(EvalError::ParseDiagnostics {
            path,
            source_text,
            diagnostics,
            ..
        }) => {
            for diag in diagnostics {
                emit(&source_text, &path.display().to_string(), &diag);
            }
            Err("parse error".to_string())
        }
        Err(e) => Err(format!("evaluation error: {e:?}")),
    }
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

    !supports_canonical_std_import(import)
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

        let (backend, value) = run_value(&root.join("main.neve"), false, false).unwrap();
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

        let (backend, value) = run_value(&root.join("main.neve"), false, false).unwrap();
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

        let (backend, value) = run_value(&root.join("main.neve"), false, false).unwrap();
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

        let (backend, value) = run_value(&root.join("main.neve"), false, false).unwrap();
        assert_eq!(backend, RunBackend::FrontendHir);
        assert_eq!(value, Value::Int(2.into()));
    }

    #[test]
    fn run_value_executes_trailing_top_level_expression() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        write_module(root, &["main"], "fn add(x, y) = x + y;\nadd(1, 2)");

        let (backend, value) = run_value(&root.join("main.neve"), false, false).unwrap();
        assert_eq!(backend, RunBackend::FrontendHir);
        assert_eq!(value, Value::Int(3.into()));
    }

    #[test]
    fn run_value_supports_global_print_in_hir() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        write_module(
            root,
            &["main"],
            "let result = { print(\"hello\"); 42 };\nresult",
        );

        let (backend, value) = run_value(&root.join("main.neve"), false, false).unwrap();
        assert_eq!(backend, RunBackend::FrontendHir);
        assert_eq!(value, Value::Int(42.into()));
    }

    #[test]
    fn run_value_prefers_frontend_hir_for_std_root_module_item_imports() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        write_module(
            root,
            &["main"],
            "import std (list); let result = list.len([1, 2]);",
        );

        let (backend, value) = run_value(&root.join("main.neve"), false, false).unwrap();
        assert_eq!(backend, RunBackend::FrontendHir);
        assert_eq!(value, Value::Int(2.into()));
    }

    #[test]
    fn run_value_requires_compat_ast_for_direct_std_root_module() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        write_module(root, &["src", "std", "mod"], "let result = 40 + 2;");

        let err = run_value(&root.join("src").join("std").join("mod.neve"), false, false)
            .expect_err("direct std root module should require --compat-ast");
        assert!(err.contains("--compat-ast"), "unexpected error: {err}");
    }

    #[test]
    fn run_value_uses_ast_compat_for_direct_std_root_module() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        write_module(root, &["src", "std", "mod"], "let result = 40 + 2;");

        let (backend, value) =
            run_value(&root.join("src").join("std").join("mod.neve"), false, true).unwrap();
        assert_eq!(backend, RunBackend::AstCompat);
        assert_eq!(value, Value::Int(42.into()));
    }

    #[test]
    fn run_value_requires_compat_ast_for_std_root_module_import_with_local_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        write_module(root, &["util"], "pub fn add_one(x) = x + 1;");
        write_module(
            root,
            &["main"],
            "import util (add_one); import std; let result = add_one(std.list.len([1, 2]));",
        );

        let err = run_value(&root.join("main.neve"), false, false)
            .expect_err("unsupported std root module import should require --compat-ast");
        assert!(err.contains("--compat-ast"), "unexpected error: {err}");
    }

    #[test]
    fn run_value_uses_ast_compat_for_std_root_module_import_with_local_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        write_module(root, &["util"], "pub fn add_one(x) = x + 1;");
        write_module(
            root,
            &["main"],
            "import util (add_one); import std; let result = add_one(std.list.len([1, 2]));",
        );

        let (backend, value) = run_value(&root.join("main.neve"), false, true).unwrap();
        assert_eq!(backend, RunBackend::AstCompat);
        assert_eq!(value, Value::Int(3.into()));
    }
}
