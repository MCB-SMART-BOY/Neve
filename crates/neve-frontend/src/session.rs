//! Compatibility frontend session for incremental and REPL-style consumers.
//! 面向增量场景与 REPL 的兼容前端会话。

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use neve_common::Span;
use neve_hir::{
    DefId, Import as HirImport, ImportKind as HirImportKind, ImportPathPrefix, ModuleId,
    ModuleInfo, ModuleLoadError, ModuleLoader, ModulePath, Resolver, Ty,
};
use neve_syntax::{ImportDef, ImportItems, ItemKind, PathPrefix, SourceFile, Visibility};
use neve_typeck::TypeChecker;

use crate::{
    Diagnostic, Module, ModuleAnalysis, ModuleSemantics, collect_item_names_from_modules,
    collect_module_semantics, rewrite_diagnostics_with_names,
};

/// Diagnostics attributed to one loaded module in a frontend session.
/// Frontend 会话中归属到单个已加载模块的诊断。
#[derive(Debug, Clone)]
pub struct SessionLoadedDiagnostics {
    /// Loaded module id.
    /// 已加载模块 ID。
    pub module_id: ModuleId,
    /// Backing source path on disk.
    /// 模块对应的磁盘路径。
    pub file_path: PathBuf,
    /// Final diagnostics for the module.
    /// 模块的最终诊断。
    pub diagnostics: Vec<Diagnostic>,
}

/// Inputs needed to build one in-memory module against the current session.
/// 基于当前会话构建单个内存模块时所需的输入。
#[derive(Debug, Clone, Default)]
pub struct SessionBuildInputs {
    /// Next available global definition id.
    /// 下一个可用的全局定义 ID。
    pub next_def_id: u32,
    /// Existing globals already visible to the caller.
    /// 调用方当前可见的既有全局定义。
    pub existing_globals: Vec<(String, DefId)>,
    /// Imported definitions already visible to the caller.
    /// 调用方当前可见的导入定义。
    pub imported_defs: Vec<(String, DefId)>,
    /// Imported namespace aliases already visible to the caller.
    /// 调用方当前可见的模块命名空间别名。
    pub imported_module_aliases: Vec<String>,
    /// Imported builtin items already visible to the caller.
    /// 调用方当前可见的 builtin 项导入。
    pub builtin_item_imports: Vec<(String, String)>,
    /// Imported builtin modules already visible to the caller.
    /// 调用方当前可见的 builtin 模块导入。
    pub builtin_module_imports: Vec<(String, String)>,
}

/// Result of building one in-memory module through a frontend session.
/// 通过 frontend 会话构建单个内存模块的结果。
#[derive(Debug, Clone)]
pub struct SessionBuildResult {
    /// Lowered current module.
    /// 当前输入降级后的模块。
    pub module: Module,
    /// Newly loaded dependency modules caused by this input.
    /// 当前输入触发的新加载依赖模块。
    pub newly_loaded: Vec<ModuleId>,
    /// Next global definition id after lowering this input.
    /// 当前输入降级完成后的下一个全局定义 ID。
    pub next_def_id: u32,
    /// Global definitions introduced by the current module resolver.
    /// 当前模块解析器引入的全局定义。
    pub global_defs: HashMap<String, DefId>,
}

/// Resolved import bindings for one in-memory module.
/// 单个内存模块的导入绑定解析结果。
#[derive(Debug, Clone, Default)]
pub struct SessionResolvedImports {
    /// Name bindings imported into the current scope.
    /// 导入到当前作用域的名称绑定。
    pub bindings: Vec<(String, DefId)>,
    /// Namespace aliases introduced by module imports.
    /// 模块命名空间导入引入的别名。
    pub module_aliases: Vec<String>,
}

/// Errors produced by compatibility session operations.
/// 兼容会话操作返回的错误。
#[derive(Debug, Clone)]
pub enum SessionError {
    /// Import path could not be resolved relative to the current module.
    /// 相对于当前模块无法解析导入路径。
    CannotResolveImportPath(String),
    /// Loading a dependency module failed.
    /// 加载依赖模块失败。
    ModuleLoad(ModuleLoadError),
    /// Import resolution failed after modules were loaded.
    /// 模块加载后导入解析失败。
    ImportResolution(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::CannotResolveImportPath(path) => {
                write!(
                    f,
                    "cannot resolve import path '{path}' from current REPL context"
                )
            }
            SessionError::ModuleLoad(err) => write!(f, "module load error: {err}"),
            SessionError::ImportResolution(err) => write!(f, "import resolution error: {err}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// Compatibility frontend session used by REPL/incremental consumers.
/// REPL/增量消费者使用的兼容前端会话。
#[derive(Debug, Clone)]
pub struct FrontendSession {
    loader: ModuleLoader,
    persisted_modules: Vec<Module>,
}

impl FrontendSession {
    /// Create a new session rooted at the given directory.
    /// 创建一个以给定目录为根的前端会话。
    pub fn new(root_dir: impl AsRef<Path>) -> Self {
        Self {
            loader: ModuleLoader::new(root_dir),
            persisted_modules: Vec::new(),
        }
    }

    /// Configure an explicit std source root.
    /// 配置显式的 std 源码根目录。
    pub fn with_std_path(mut self, std_path: impl AsRef<Path>) -> Self {
        self.loader = self.loader.with_std_path(std_path);
        self
    }

    /// Borrow the current root directory.
    /// 借用当前根目录。
    pub fn root_dir(&self) -> &Path {
        self.loader.root_dir()
    }

    /// Reset the session while keeping the same root directory.
    /// 在保持根目录不变的情况下重置会话。
    pub fn clear(&mut self) {
        let root_dir = self.root_dir().to_path_buf();
        *self = Self::new(root_dir);
    }

    /// Reset the session to a new root directory.
    /// 将会话重置到新的根目录。
    pub fn rebase_root(&mut self, root_dir: impl AsRef<Path>) {
        *self = Self::new(root_dir);
    }

    /// Returns whether the session is still semantically empty.
    /// 返回当前会话是否仍然是语义空状态。
    pub fn is_pristine(&self) -> bool {
        self.loader.load_order().is_empty() && self.persisted_modules.is_empty()
    }

    /// Borrow the underlying compatibility module loader.
    /// 借用底层兼容 `ModuleLoader`。
    pub fn module_loader(&self) -> &ModuleLoader {
        &self.loader
    }

    /// Borrow the underlying compatibility module loader mutably.
    /// 可变借用底层兼容 `ModuleLoader`。
    pub fn module_loader_mut(&mut self) -> &mut ModuleLoader {
        &mut self.loader
    }

    /// Borrow loaded module info by id.
    /// 按 id 借用已加载模块信息。
    pub fn module_info(&self, module_id: ModuleId) -> Option<&ModuleInfo> {
        self.loader.get_module(module_id)
    }

    /// Borrow a lowered module by id.
    /// 按 id 借用降级后的模块。
    pub fn hir_module(&self, module_id: ModuleId) -> Option<&Module> {
        self.loader.hir_module(module_id)
    }

    /// Borrow the loaded dependency-first order.
    /// 借用依赖优先的已加载顺序。
    pub fn load_order(&self) -> &[ModuleId] {
        self.loader.load_order()
    }

    /// Get the next global definition id counter tracked by the loader.
    /// 获取 loader 维护的全局定义 ID 计数器。
    pub fn next_def_id(&self) -> u32 {
        self.loader.next_def_id()
    }

    /// Update the next global definition id counter tracked by the loader.
    /// 更新 loader 维护的全局定义 ID 计数器。
    pub fn set_def_id_counter(&mut self, next: u32) {
        self.loader.set_def_id_counter(next);
    }

    /// Record a persisted in-memory module.
    /// 记录一个持久化的内存模块。
    pub fn record_module(&mut self, module: Module) {
        self.persisted_modules.push(module);
    }

    /// Borrow all persisted in-memory modules.
    /// 借用所有持久化内存模块。
    pub fn persisted_modules(&self) -> &[Module] {
        &self.persisted_modules
    }

    /// Analyze loaded modules from the compatibility loader.
    /// 分析兼容 loader 中的已加载模块。
    pub fn analyze_loaded_modules(&self) -> HashMap<ModuleId, ModuleAnalysis> {
        let (global_types, global_spans) = self.collect_loaded_global_env();
        let type_names = self.collect_type_names(None);
        let mut modules = HashMap::new();

        for module_id in self.loader.load_order() {
            let parse_diagnostics = self.loader.parsed_diagnostics(*module_id).unwrap_or(&[]);
            if !parse_diagnostics.is_empty() {
                modules.insert(
                    *module_id,
                    ModuleAnalysis {
                        diagnostics: parse_diagnostics.to_vec(),
                        semantics: ModuleSemantics::default(),
                    },
                );
                continue;
            }

            let Some(module) = self.loader.hir_module(*module_id) else {
                continue;
            };

            let mut checker =
                TypeChecker::with_global_env(global_types.clone(), global_spans.clone());
            checker.check(module);
            let semantics = collect_module_semantics(&checker);
            let diagnostics =
                rewrite_diagnostics_with_names(checker.diagnostics_ref().to_vec(), &type_names);

            modules.insert(
                *module_id,
                ModuleAnalysis {
                    diagnostics,
                    semantics,
                },
            );
        }

        modules
    }

    /// Analyze a current in-memory module against loaded + persisted session state.
    /// 基于已加载模块与持久化模块状态分析当前内存模块。
    pub fn analyze_module(&self, current_module: &Module) -> ModuleAnalysis {
        let mut checker = TypeChecker::new();

        for module_id in self.loader.load_order() {
            let Some(module) = self.loader.hir_module(*module_id) else {
                continue;
            };
            checker.check(module);
            checker.clear_diagnostics();
            checker.clear_method_resolutions();
        }

        for module in &self.persisted_modules {
            checker.check(module);
            checker.clear_diagnostics();
            checker.clear_method_resolutions();
        }

        checker.check(current_module);
        let type_names = self.collect_type_names(Some(current_module));
        let diagnostics =
            rewrite_diagnostics_with_names(checker.diagnostics_ref().to_vec(), &type_names);

        ModuleAnalysis {
            diagnostics,
            semantics: collect_module_semantics(&checker),
        }
    }

    /// Collect readable type names visible to the current session.
    /// 收集当前会话可见的可读类型名。
    pub fn type_names(&self) -> HashMap<DefId, String> {
        self.collect_type_names(None)
    }

    /// Collect readable type names visible to the current session plus one extra module.
    /// 收集当前会话加一个额外模块可见的可读类型名。
    pub fn type_names_with_current(&self, current_module: &Module) -> HashMap<DefId, String> {
        self.collect_type_names(Some(current_module))
    }

    /// Collect diagnostics for newly loaded modules using canonical frontend analysis.
    /// 使用规范 frontend 分析收集新加载模块的诊断。
    pub fn loaded_module_diagnostics(
        &self,
        newly_loaded: &[ModuleId],
    ) -> Vec<SessionLoadedDiagnostics> {
        let pending: std::collections::HashSet<_> = newly_loaded.iter().copied().collect();
        let analyses = self.analyze_loaded_modules();
        let mut entries = Vec::new();

        for module_id in self.loader.load_order() {
            if !pending.contains(module_id) {
                continue;
            }

            let Some(info) = self.loader.get_module(*module_id) else {
                continue;
            };
            let Some(analysis) = analyses.get(module_id) else {
                continue;
            };

            if analysis
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == neve_diagnostic::Severity::Error)
            {
                entries.push(SessionLoadedDiagnostics {
                    module_id: *module_id,
                    file_path: info.file_path.clone(),
                    diagnostics: analysis.diagnostics.clone(),
                });
            }
        }

        entries
    }

    /// Build a current in-memory module against the session state.
    /// 基于当前会话状态构建当前内存模块。
    pub fn build_module_from_ast(
        &mut self,
        ast: &SourceFile,
        module_name: String,
        module_path: Vec<String>,
        inputs: &SessionBuildInputs,
    ) -> Result<SessionBuildResult, SessionError> {
        self.set_def_id_counter(inputs.next_def_id.max(self.next_def_id()));
        let known_modules: HashSet<_> = self.loader.load_order().iter().copied().collect();

        for item in &ast.items {
            let ItemKind::Import(import) = &item.kind else {
                continue;
            };
            if session_std_module_prefix(import).is_some() {
                continue;
            }

            let import_path = ModulePath::from_import_def(import);
            let Some(absolute_path) = self
                .loader
                .resolve_module_path(&import_path, Some(&module_path))
            else {
                return Err(SessionError::CannotResolveImportPath(format_import_path(
                    import,
                )));
            };

            self.loader
                .load_module(&absolute_path)
                .map_err(SessionError::ModuleLoad)?;
        }

        let mut resolver = Resolver::new();
        resolver.set_def_id_counter(inputs.next_def_id.max(self.next_def_id()));
        resolver.register_existing_globals(inputs.existing_globals.clone());
        resolver.register_imports(inputs.imported_defs.clone());
        for alias in &inputs.imported_module_aliases {
            resolver.register_module_import_alias(alias.clone());
        }
        for (name, builtin_name) in &inputs.builtin_item_imports {
            resolver.register_builtin_item_import(name.clone(), builtin_name.clone());
        }
        for (alias, module_prefix) in &inputs.builtin_module_imports {
            resolver.register_builtin_module_import(alias.clone(), module_prefix.clone());
        }
        resolver.set_module_loader(self.loader.clone());
        let module = resolver.resolve_with_path(ast, module_name, module_path);
        let next_def_id = resolver.next_def_id().max(self.next_def_id());
        self.set_def_id_counter(next_def_id);

        Ok(SessionBuildResult {
            module,
            newly_loaded: self
                .loader
                .load_order()
                .iter()
                .copied()
                .filter(|module_id| !known_modules.contains(module_id))
                .collect(),
            next_def_id,
            global_defs: resolver.global_defs().clone(),
        })
    }

    /// Resolve imports for a current in-memory module against the session state.
    /// 基于当前会话状态解析当前内存模块的导入。
    pub fn resolve_ast_imports(
        &self,
        ast: &SourceFile,
        current_module_path: &[String],
    ) -> Result<SessionResolvedImports, SessionError> {
        let mut resolved = SessionResolvedImports::default();

        for item in &ast.items {
            let ItemKind::Import(import) = &item.kind else {
                continue;
            };
            if session_std_module_prefix(import).is_some() {
                continue;
            }

            let bindings = self
                .loader
                .resolve_import(&hir_import_from_ast(item.span, import), current_module_path)
                .map_err(|err| SessionError::ImportResolution(err.to_string()))?;
            resolved.bindings.extend(bindings);

            if matches!(import.items, ImportItems::Module)
                && let Some(alias) = import
                    .alias
                    .as_ref()
                    .map(|alias| alias.name.clone())
                    .or_else(|| import.path.last().map(|segment| segment.name.clone()))
            {
                resolved.module_aliases.push(alias);
            }
        }

        Ok(resolved)
    }

    fn collect_loaded_global_env(&self) -> (HashMap<DefId, Ty>, HashMap<DefId, neve_common::Span>) {
        let mut global_types = HashMap::new();
        let mut global_spans = HashMap::new();

        for module_id in self.loader.load_order() {
            let Some(module) = self.loader.hir_module(*module_id) else {
                continue;
            };
            let (types, spans) = TypeChecker::collect_signatures(module);
            global_types.extend(types);
            global_spans.extend(spans);
        }

        (global_types, global_spans)
    }

    fn collect_type_names(&self, current_module: Option<&Module>) -> HashMap<DefId, String> {
        let mut modules: Vec<&Module> = self
            .loader
            .load_order()
            .iter()
            .filter_map(|module_id| self.loader.hir_module(*module_id))
            .collect();
        modules.extend(self.persisted_modules.iter());
        if let Some(module) = current_module {
            modules.push(module);
        }
        collect_item_names_from_modules(modules)
    }
}

fn session_std_module_prefix(import: &ImportDef) -> Option<&str> {
    if import.path.len() == 2
        && import.path.first().map(|segment| segment.name.as_str()) == Some("std")
    {
        Some(import.path[1].name.as_str())
    } else {
        None
    }
}

fn hir_import_from_ast(span: Span, import: &ImportDef) -> HirImport {
    let prefix = match import.prefix {
        PathPrefix::Absolute => ImportPathPrefix::Absolute,
        PathPrefix::Self_ => ImportPathPrefix::Self_,
        PathPrefix::Super => ImportPathPrefix::Super,
        PathPrefix::Crate => ImportPathPrefix::Crate,
    };

    let kind = match &import.items {
        ImportItems::Module => HirImportKind::Module,
        ImportItems::Items(items) => {
            HirImportKind::Items(items.iter().map(|item| item.name.clone()).collect())
        }
        ImportItems::All => HirImportKind::All,
    };

    HirImport {
        prefix,
        path: import
            .path
            .iter()
            .map(|segment| segment.name.clone())
            .collect(),
        kind,
        alias: import.alias.as_ref().map(|alias| alias.name.clone()),
        is_pub: import.visibility == Visibility::Public,
        span,
    }
}

fn format_import_path(import: &ImportDef) -> String {
    let prefix = match import.prefix {
        PathPrefix::Absolute => "",
        PathPrefix::Self_ => "self.",
        PathPrefix::Super => "super.",
        PathPrefix::Crate => "crate.",
    };
    format!(
        "{}{}",
        prefix,
        import
            .path
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join(".")
    )
}
