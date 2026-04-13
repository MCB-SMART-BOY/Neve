//! Compatibility multi-module frontend driver.
//! 兼容式多模块前端驱动。

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use neve_common::Span;
use neve_hir::{DefId, ModuleId, ModuleInfo, ModuleLoadError, ModuleLoader};
use neve_typeck::TypeChecker;

use crate::{
    Diagnostic, Module, ModuleSemantics, SourceFile, collect_item_names_from_modules,
    collect_module_semantics, rewrite_diagnostics_with_names,
};

/// Per-module semantic analysis produced by the compatibility driver.
/// 兼容驱动产出的单模块语义分析结果。
#[derive(Debug, Clone)]
pub struct ModuleAnalysis {
    /// Diagnostics for this module after parse/type analysis.
    /// 当前模块在解析/类型检查后的诊断。
    pub diagnostics: Vec<Diagnostic>,
    /// Canonical semantic side tables for this module.
    /// 当前模块的规范语义 side tables。
    pub semantics: ModuleSemantics,
}

/// Multi-module program analysis produced by the compatibility driver.
/// 兼容驱动产出的多模块程序分析结果。
#[derive(Debug, Clone)]
pub struct ProgramAnalysis {
    root_id: ModuleId,
    loader: ModuleLoader,
    modules: HashMap<ModuleId, ModuleAnalysis>,
    type_names: HashMap<DefId, String>,
}

impl ProgramAnalysis {
    /// Entry/root module ID for this analysis.
    /// 当前分析的入口/根模块 ID。
    pub fn root_module_id(&self) -> ModuleId {
        self.root_id
    }

    /// Module load order (dependencies first).
    /// 模块加载顺序（依赖优先）。
    pub fn load_order(&self) -> &[ModuleId] {
        self.loader.load_order()
    }

    /// Borrow module info by ID.
    /// 按 ID 借用模块信息。
    pub fn module_info(&self, id: ModuleId) -> Option<&ModuleInfo> {
        self.loader.get_module(id)
    }

    /// Borrow lowered HIR by module ID.
    /// 按模块 ID 借用降级后的 HIR。
    pub fn hir_module(&self, id: ModuleId) -> Option<&Module> {
        self.loader.hir_module(id)
    }

    /// Borrow parsed AST by module ID.
    /// 按模块 ID 借用解析后的 AST。
    pub fn parsed_source(&self, id: ModuleId) -> Option<&SourceFile> {
        self.loader.parsed_source(id)
    }

    /// Borrow cached parse diagnostics by module ID.
    /// 按模块 ID 借用缓存的解析诊断。
    pub fn parsed_diagnostics(&self, id: ModuleId) -> Option<&[Diagnostic]> {
        self.loader.parsed_diagnostics(id)
    }

    /// Borrow the final diagnostics for a module.
    /// 借用模块的最终诊断。
    pub fn diagnostics(&self, id: ModuleId) -> Option<&[Diagnostic]> {
        self.modules
            .get(&id)
            .map(|module| module.diagnostics.as_slice())
    }

    /// Borrow method resolutions for a module.
    /// 借用模块的方法解析结果。
    pub fn method_resolutions(&self, id: ModuleId) -> Option<&HashMap<Span, DefId>> {
        self.modules
            .get(&id)
            .map(|module| &module.semantics.method_resolutions)
    }

    /// Borrow canonical side tables for a module.
    /// 借用模块的规范语义 side tables。
    pub fn semantics(&self, id: ModuleId) -> Option<&ModuleSemantics> {
        self.modules.get(&id).map(|module| &module.semantics)
    }

    /// Borrow the shared visible type-name map.
    /// 借用共享的可见类型名映射。
    pub fn type_names(&self) -> &HashMap<DefId, String> {
        &self.type_names
    }
}

/// Errors produced by the compatibility frontend driver.
/// 兼容前端驱动返回的错误。
#[derive(Debug, Clone)]
pub enum FrontendError {
    /// Module loading failed before semantic analysis could complete.
    /// 在语义分析开始前模块加载失败。
    ModuleLoad(ModuleLoadError),
}

impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrontendError::ModuleLoad(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for FrontendError {}

/// Compatibility driver for multi-module frontend analysis.
/// 多模块前端分析的兼容驱动。
#[derive(Debug, Clone)]
pub struct FrontendDriver {
    root_dir: PathBuf,
    std_path: Option<PathBuf>,
}

impl FrontendDriver {
    /// Create a driver rooted at a source directory.
    /// 创建一个以源码目录为根的驱动。
    pub fn new(root_dir: impl AsRef<Path>) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
            std_path: None,
        }
    }

    /// Configure an explicit std source root for module resolution.
    /// 为模块解析配置显式的 std 源码根路径。
    pub fn with_std_path(mut self, std_path: impl AsRef<Path>) -> Self {
        self.std_path = Some(std_path.as_ref().to_path_buf());
        self
    }

    /// Analyze a root module path and all transitively loaded modules.
    /// 分析一个根模块路径及其所有传递加载的模块。
    pub fn analyze_module_path(
        &self,
        module_path: &[String],
    ) -> Result<ProgramAnalysis, FrontendError> {
        let mut loader = ModuleLoader::new(&self.root_dir);
        if let Some(std_path) = &self.std_path {
            loader = loader.with_std_path(std_path);
        }

        let root_id = loader
            .load_module(module_path)
            .map_err(FrontendError::ModuleLoad)?;

        let mut global_types = HashMap::new();
        let mut global_spans = HashMap::new();
        for module_id in loader.load_order() {
            if let Some(module) = loader.hir_module(*module_id) {
                let (types, spans) = TypeChecker::collect_signatures(module);
                global_types.extend(types);
                global_spans.extend(spans);
            }
        }

        let type_names = collect_item_names_from_modules(
            loader
                .load_order()
                .iter()
                .filter_map(|module_id| loader.hir_module(*module_id)),
        );

        let mut modules = HashMap::new();
        for module_id in loader.load_order() {
            let parse_diagnostics = loader.parsed_diagnostics(*module_id).unwrap_or(&[]);
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

            let Some(module) = loader.hir_module(*module_id) else {
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

        Ok(ProgramAnalysis {
            root_id,
            loader,
            modules,
            type_names,
        })
    }
}

/// Analyze a module path rooted at the given source directory.
/// 分析指定源码目录下的模块路径。
pub fn analyze_module_path(
    root_dir: impl AsRef<Path>,
    module_path: &[String],
) -> Result<ProgramAnalysis, FrontendError> {
    FrontendDriver::new(root_dir).analyze_module_path(module_path)
}
