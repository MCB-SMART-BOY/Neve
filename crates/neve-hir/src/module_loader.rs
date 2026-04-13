//! Module loading and compatibility-facing module discovery.
//! 模块加载与面向兼容层的模块发现。
//!
//! This module provides functionality for:
//! 本模块提供以下功能：
//! - Discovering modules from file system / 从文件系统发现模块
//! - Resolving module paths (self, super, crate) / 解析模块路径（self、super、crate）
//! - Loading and caching modules / 加载和缓存模块
//! - Acting as a compatibility facade over module graph state / 作为模块图状态的兼容外观
//! - Incremental compilation with file timestamps / 基于文件时间戳的增量编译

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use neve_diagnostic::Diagnostic;

use crate::incremental::{CacheStats, IncrementalCache, get_mtime};
use crate::module_diagnostics::ModuleDiagnostics;
use crate::module_graph::ModuleGraphState;
use crate::module_lowering::{LoweredModuleArtifacts, collect_imports, lower_module_with_imports};
use crate::module_paths::ModulePathResolver;
use crate::{DefId, Import, ImportKind, Module, ModuleId, ModulePath};

// Re-export Visibility from the AST
// 从 AST 重新导出 Visibility
pub use neve_syntax::Visibility;

/// Information about a loaded module.
/// 已加载模块的信息。
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module ID. / 模块 ID。
    pub id: ModuleId,
    /// Module path (e.g., ["std", "list"]). / 模块路径（例如 ["std", "list"]）。
    pub path: Vec<String>,
    /// File path on disk. / 磁盘上的文件路径。
    pub file_path: PathBuf,
    /// Parent module (None for root). / 父模块（根模块为 None）。
    pub parent: Option<ModuleId>,
    /// Child modules. / 子模块。
    pub children: Vec<ModuleId>,
    /// Imported module dependencies. / 导入的模块依赖。
    pub dependencies: Vec<ModuleId>,
    /// Exported items (name -> DefId). / 导出的项（名称 -> DefId）。
    pub exports: HashMap<String, DefId>,
    /// All items with visibility. / 所有带有可见性的项。
    pub items: HashMap<String, (DefId, Visibility)>,
    /// File modification time for incremental compilation.
    /// 用于增量编译的文件修改时间。
    pub mtime: Option<SystemTime>,
}

/// Module loader responsible for discovering and loading modules.
/// 负责发现和加载模块的模块加载器。
#[derive(Debug, Clone)]
pub struct ModuleLoader {
    /// Rooted path/file resolver. / 以根目录为基准的路径/文件解析器。
    path_resolver: ModulePathResolver,
    /// Module graph state. / 模块图状态。
    graph: ModuleGraphState,
    /// Next module ID. / 下一个模块 ID。
    next_id: u32,
    /// Diagnostics collected during loading. / 加载期间收集的诊断信息。
    diagnostics: ModuleDiagnostics,
    /// Modules currently being loaded (for cycle detection).
    /// Maps module path to its loading stack for detailed error messages.
    /// 当前正在加载的模块（用于循环检测）。
    /// 将模块路径映射到其加载栈以获取详细的错误消息。
    loading: HashSet<Vec<String>>,
    /// Loading stack to track the import chain.
    /// 加载栈用于跟踪导入链。
    loading_stack: Vec<Vec<String>>,
    /// Incremental cache and parsed-source storage.
    /// 增量缓存与解析源码存储。
    cache: IncrementalCache,
    /// Next global definition ID. / 下一个全局定义 ID。
    next_def_id: u32,
    /// Lowered HIR modules by ID. / 按 ID 存储的已降级 HIR 模块。
    hir_modules: HashMap<ModuleId, crate::Module>,
}

impl ModuleLoader {
    /// Create a new module loader with the given root directory.
    /// 使用给定的根目录创建新的模块加载器。
    pub fn new(root_dir: impl AsRef<Path>) -> Self {
        Self {
            path_resolver: ModulePathResolver::new(root_dir),
            graph: ModuleGraphState::default(),
            next_id: 0,
            diagnostics: ModuleDiagnostics::default(),
            loading: HashSet::new(),
            loading_stack: Vec::new(),
            cache: IncrementalCache::default(),
            next_def_id: 0,
            hir_modules: HashMap::new(),
        }
    }

    /// Set the standard library path.
    /// 设置标准库路径。
    pub fn with_std_path(mut self, path: impl AsRef<Path>) -> Self {
        self.path_resolver.set_std_path(path);
        self
    }

    /// Get the root directory.
    /// 获取根目录。
    pub fn root_dir(&self) -> &Path {
        self.path_resolver.root_dir()
    }

    /// Set the next global definition ID counter used when lowering modules.
    /// 设置模块降级时使用的下一个全局定义 ID 计数器。
    pub fn set_def_id_counter(&mut self, next: u32) {
        self.next_def_id = next;
    }

    /// Get the next global definition ID counter.
    /// 获取当前的全局定义 ID 计数器。
    pub fn next_def_id(&self) -> u32 {
        self.next_def_id
    }

    /// Get collected diagnostics.
    /// 获取收集的诊断信息。
    pub fn diagnostics(&self) -> &[Diagnostic] {
        self.diagnostics.as_slice()
    }

    /// Take collected diagnostics.
    /// 取出收集的诊断信息。
    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        self.diagnostics.take()
    }

    /// Get cache statistics. / 获取缓存统计信息。
    pub fn cache_stats(&self) -> &CacheStats {
        self.cache.cache_stats()
    }

    /// Get the parsed source for a loaded module.
    /// 获取已加载模块的解析源文件。
    pub fn parsed_source(&self, module_id: ModuleId) -> Option<&neve_syntax::SourceFile> {
        let info = self.graph.get_module(module_id)?;
        self.cache.parsed_source(&info.file_path)
    }

    /// Get parse diagnostics for a loaded module.
    /// 获取已加载模块的解析诊断。
    pub fn parsed_diagnostics(&self, module_id: ModuleId) -> Option<&[Diagnostic]> {
        let info = self.graph.get_module(module_id)?;
        self.cache.parsed_diagnostics(&info.file_path)
    }

    /// Invalidate cache for a file. / 使文件的缓存失效。
    pub fn invalidate_cache(&mut self, file_path: &Path) {
        self.cache.invalidate_cache(file_path);
    }

    /// Clear all cache entries. / 清除所有缓存条目。
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get list of files that need recompilation.
    /// 获取需要重新编译的文件列表。
    pub fn get_dirty_files(&self) -> Vec<PathBuf> {
        self.cache.get_dirty_files()
    }

    /// Check if a file's content has changed using hash comparison.
    /// Returns true if content has changed or file cannot be read.
    /// 使用哈希比较检查文件内容是否已更改。
    /// 如果内容已更改或文件无法读取，则返回 true。
    pub fn has_content_changed(&self, file_path: &Path) -> bool {
        self.cache.has_content_changed(file_path)
    }

    /// Get cached modification time for a file.
    /// 获取文件的缓存修改时间。
    pub fn get_cached_mtime(&self, file_path: &Path) -> Option<SystemTime> {
        self.cache.get_cached_mtime(file_path)
    }

    /// Get cached source hash for a file.
    /// 获取文件的缓存源哈希。
    pub fn get_cached_hash(&self, file_path: &Path) -> Option<u64> {
        self.cache.get_cached_hash(file_path)
    }

    /// Mark a file as dirty (needs recompilation).
    /// 将文件标记为脏（需要重新编译）。
    pub fn mark_file_dirty(&mut self, file_path: &Path) {
        self.cache.mark_file_dirty(file_path);
    }

    /// Mark a file as clean after successful recompilation.
    /// 成功重新编译后将文件标记为干净。
    pub fn mark_file_clean(&mut self, file_path: &Path) {
        self.cache.mark_file_clean(file_path);
    }

    /// Mark all dependents of a file as dirty (for incremental recompilation).
    /// This marks the file itself and all modules that import it as needing recompilation.
    /// 将文件的所有依赖项标记为脏（用于增量重新编译）。
    /// 这会将文件本身及所有导入它的模块标记为需要重新编译。
    pub fn invalidate_dependents(&mut self, file_path: &Path) {
        if let Some(module_id) = self.graph.module_id_for_file(file_path) {
            for current in self.graph.dependent_closure(module_id) {
                if let Some(file_path) = self
                    .graph
                    .get_module(current)
                    .map(|info| info.file_path.clone())
                {
                    self.mark_file_dirty(&file_path);
                }
            }
        }
    }

    /// Allocate a new module ID.
    /// 分配新的模块 ID。
    fn fresh_module_id(&mut self) -> ModuleId {
        let id = ModuleId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Resolve a module path to a file path.
    /// 将模块路径解析为文件路径。
    pub fn resolve_path(
        &self,
        path: &ModulePath,
        from_module: Option<&[String]>,
    ) -> Option<PathBuf> {
        self.path_resolver.resolve_path(path, from_module)
    }

    /// Resolve an import path to its absolute module path segments.
    /// 将导入路径解析为绝对模块路径段。
    pub fn resolve_module_path(
        &self,
        path: &ModulePath,
        from_module: Option<&[String]>,
    ) -> Option<Vec<String>> {
        self.path_resolver.resolve_module_path(path, from_module)
    }

    /// Load a module by path.
    /// 按路径加载模块。
    pub fn load_module(&mut self, path: &[String]) -> Result<ModuleId, ModuleLoadError> {
        // Check if already loaded
        // 检查是否已加载
        if let Some(id) = self.graph.lookup_module(path) {
            return Ok(id);
        }

        // Check for circular dependency
        // 检查循环依赖
        if self.loading.contains(path) {
            // Build the circular dependency chain
            // 构建循环依赖链
            let mut chain = self.loading_stack.clone();
            chain.push(path.to_vec());
            return Err(ModuleLoadError::CircularDependency {
                module: path.to_vec(),
                chain,
            });
        }

        // Find the file
        // 查找文件
        let file_path = self
            .path_resolver
            .find_module_file(path)
            .ok_or_else(|| ModuleLoadError::NotFound(path.to_vec()))?;

        // Mark as loading and add to stack
        // 标记为正在加载并添加到栈
        self.loading.insert(path.to_vec());
        self.loading_stack.push(path.to_vec());

        // Check if file needs recompilation (incremental compilation)
        // 检查文件是否需要重新编译（增量编译）
        let needs_recompile = self.cache.record_recompile_check(&file_path);

        // Read and parse the file
        // 读取并解析文件
        let source = fs::read_to_string(&file_path)
            .map_err(|e| ModuleLoadError::IoError(file_path.clone(), e.to_string()))?;

        let (source_file, parse_errors) = self.cache.parse_source(&file_path, &source);

        // Update cache with new mtime and source hash
        // 使用新的 mtime 和源哈希更新缓存
        self.cache.finish_load(&file_path, &source, needs_recompile);

        // Collect parse errors
        // 收集解析错误
        self.diagnostics.extend(parse_errors);

        // Allocate module ID
        // 分配模块 ID
        let module_id = self.fresh_module_id();

        // Load dependencies (imports) BEFORE registering the module as loaded
        // This allows circular dependency detection to work correctly
        // 在将模块注册为已加载之前加载依赖（导入）
        // 这使得循环依赖检测能够正常工作
        //
        // IMPORTANT: For `pub import` (re-exports), we need special handling to avoid
        // infinite loops when modules re-export each other's symbols.
        // 重要：对于 `pub import`（重导出），我们需要特殊处理以避免
        // 模块相互重导出符号时的无限循环。
        for item in &source_file.items {
            if let neve_syntax::ItemKind::Import(import_def) = &item.kind {
                let import_path = ModulePath::from_import_def(import_def);

                // Check if this is a re-export (pub import)
                // 检查是否为重导出（pub import）
                let is_reexport = import_def.visibility != neve_syntax::Visibility::Private;

                #[allow(clippy::collapsible_if)]
                if let Some(abs_path) = self
                    .path_resolver
                    .resolve_module_path(&import_path, Some(path))
                    && abs_path != path
                // Only load if not a self-reference / 仅在不是自引用时加载
                {
                    // For re-exports, check if the target module is already being loaded
                    // in our dependency chain. If so, we can safely skip loading it now
                    // and defer symbol resolution to later.
                    // 对于重导出，检查目标模块是否已在我们的依赖链中加载。
                    // 如果是，我们可以安全地跳过现在加载它，并将符号解析推迟到以后。
                    if is_reexport && self.loading.contains(&abs_path) {
                        // This is a re-export of a module that's currently being loaded.
                        // This is safe - we'll resolve the symbols later after all modules
                        // are loaded. This breaks the infinite loop.
                        // 这是当前正在加载的模块的重导出。
                        // 这是安全的 - 我们将在所有模块加载后解析符号。这打破了无限循环。
                        continue;
                    }

                    // Propagate circular dependency errors immediately
                    // 立即传播循环依赖错误
                    if let Err(e) = self.load_module(&abs_path) {
                        match &e {
                            ModuleLoadError::NotFound(_)
                                if ModulePathResolver::is_std_path(&abs_path) =>
                            {
                                continue;
                            }
                            // Circular dependencies and module not found should fail immediately
                            // 循环依赖和模块未找到应立即失败
                            ModuleLoadError::CircularDependency { .. }
                            | ModuleLoadError::NotFound(_) => {
                                // Remove from loading set and stack before returning error
                                // 在返回错误之前从加载集和栈中移除
                                self.loading.remove(path);
                                self.loading_stack.pop();
                                return Err(e);
                            }
                            // Other errors get logged but don't block loading
                            // 其他错误被记录但不阻止加载
                            _ => {
                                self.diagnostics.push(Diagnostic::error(
                                    neve_diagnostic::DiagnosticKind::Module,
                                    item.span,
                                    format!(
                                        "Failed to load module '{}': {}",
                                        abs_path.join("."),
                                        e
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }

        let imports = collect_imports(&source_file);
        let mut resolved_imports = Vec::new();
        let mut reexports: Vec<(String, DefId)> = Vec::new();
        for import in &imports {
            match self.resolve_import(import, path) {
                Ok(resolved) => {
                    if import.is_pub {
                        reexports.extend(resolved.iter().cloned());
                    }
                    resolved_imports.extend(resolved);
                }
                Err(err) => {
                    if matches!(err, ImportResolveError::ModuleNotFound(_)) {
                        let import_path = ModulePath::from_hir_import(import);
                        if let Some(abs_path) = self
                            .path_resolver
                            .resolve_module_path(&import_path, Some(path))
                            && ModulePathResolver::is_std_path(&abs_path)
                        {
                            continue;
                        }
                    }
                    self.diagnostics.push(Diagnostic::error(
                        neve_diagnostic::DiagnosticKind::Module,
                        import.span,
                        format!(
                            "Failed to resolve import '{}': {}",
                            ModulePath::from_hir_import(import),
                            err
                        ),
                    ));
                }
            }
        }

        let LoweredModuleArtifacts {
            module,
            items,
            exports,
            next_def_id,
        } = lower_module_with_imports(
            &source_file,
            path,
            module_id,
            self.next_def_id,
            resolved_imports,
            reexports,
        );
        self.next_def_id = next_def_id;

        let dependencies = self.collect_dependencies(&imports, path);

        // Create module info
        // 创建模块信息
        let mtime = get_mtime(&file_path);
        let info = ModuleInfo {
            id: module_id,
            path: path.to_vec(),
            file_path: file_path.clone(),
            parent: self.graph.find_parent_module(path),
            children: Vec::new(),
            dependencies: dependencies.clone(),
            exports,
            items,
            mtime,
        };

        // Register the module as loaded (only after dependencies are loaded)
        // 将模块注册为已加载（仅在依赖加载后）
        let parent_id = info.parent;
        self.graph.register_module(info);
        self.hir_modules.insert(module_id, module);
        self.graph
            .register_dependency_edges(module_id, &dependencies);

        // Update parent's children list
        // 更新父模块的子模块列表
        if let Some(parent_id) = parent_id {
            self.graph.register_child(parent_id, module_id);
        }

        // Remove from loading set and stack
        // 从加载集和栈中移除
        self.loading.remove(path);
        self.loading_stack.pop();

        Ok(module_id)
    }

    /// Collect imported module dependencies for a module path.
    /// 收集指定模块路径的导入模块依赖。
    fn collect_dependencies(&self, imports: &[Import], from_module: &[String]) -> Vec<ModuleId> {
        self.graph
            .collect_dependencies(imports, from_module, &self.path_resolver)
    }
    /// Get module info by ID.
    /// 按 ID 获取模块信息。
    pub fn get_module(&self, id: ModuleId) -> Option<&ModuleInfo> {
        self.graph.get_module(id)
    }

    /// Get mutable module info by ID.
    /// 按 ID 获取可变模块信息。
    pub fn get_module_mut(&mut self, id: ModuleId) -> Option<&mut ModuleInfo> {
        self.graph.get_module_mut(id)
    }

    /// Look up a module by path.
    /// 按路径查找模块。
    pub fn lookup_module(&self, path: &[String]) -> Option<ModuleId> {
        self.graph.lookup_module(path)
    }

    /// Get all loaded modules.
    /// 获取所有已加载的模块。
    pub fn all_modules(&self) -> impl Iterator<Item = (&Vec<String>, &ModuleInfo)> {
        self.graph.all_modules()
    }

    /// Get lowered HIR module by ID.
    /// 按 ID 获取降级后的 HIR 模块。
    pub fn hir_module(&self, id: ModuleId) -> Option<&Module> {
        self.hir_modules.get(&id)
    }

    /// Iterate all lowered HIR modules.
    /// 迭代所有已降级的 HIR 模块。
    pub fn hir_modules(&self) -> impl Iterator<Item = (&ModuleId, &Module)> {
        self.hir_modules.iter()
    }

    /// Get module load order (dependencies first).
    /// 获取模块加载顺序（依赖优先）。
    pub fn load_order(&self) -> &[ModuleId] {
        self.graph.load_order()
    }

    /// Register an exported item for a module.
    /// 为模块注册导出的项。
    pub fn register_export(
        &mut self,
        module_id: ModuleId,
        name: String,
        def_id: DefId,
        visibility: Visibility,
    ) {
        if let Some(info) = self.graph.get_module_mut(module_id) {
            info.items.insert(name.clone(), (def_id, visibility));
            if visibility == Visibility::Public {
                info.exports.insert(name, def_id);
            }
        }
    }

    /// Resolve an import from one module to another.
    /// 解析从一个模块到另一个模块的导入。
    pub fn resolve_import(
        &self,
        import: &Import,
        from_module: &[String],
    ) -> Result<Vec<(String, DefId)>, ImportResolveError> {
        let import_path = ModulePath::from_hir_import(import);

        let target_path = self
            .path_resolver
            .resolve_module_path(&import_path, Some(from_module))
            .ok_or_else(|| ImportResolveError::InvalidPath(import.path.clone()))?;

        let target_id = self
            .graph
            .lookup_module(&target_path)
            .ok_or_else(|| ImportResolveError::ModuleNotFound(target_path.clone()))?;

        let target_info = self
            .graph
            .get_module(target_id)
            .ok_or_else(|| ImportResolveError::ModuleNotFound(target_path.clone()))?;

        // Check visibility based on module relationship
        // 根据模块关系检查可见性
        let can_access = |visibility: Visibility| -> bool {
            match visibility {
                Visibility::Public => true,
                Visibility::Crate => true, // Within same crate / 在同一 crate 内
                Visibility::Super => {
                    // Check if from_module is a child of target's parent
                    // 检查 from_module 是否是目标父模块的子模块
                    if let Some(parent) = &target_info.parent
                        && let Some(parent_info) = self.graph.get_module(*parent)
                    {
                        return from_module.starts_with(&parent_info.path);
                    }
                    false
                }
                Visibility::Private => from_module == target_path.as_slice(),
            }
        };

        match &import.kind {
            ImportKind::Module => {
                // Import the module as a namespace
                // 将模块作为命名空间导入
                let alias = import
                    .alias
                    .as_ref()
                    .or_else(|| target_path.last())
                    .cloned()
                    .ok_or_else(|| ImportResolveError::InvalidPath(import.path.clone()))?;

                // Return all accessible exports with the namespace prefix
                // 返回带有命名空间前缀的所有可访问导出
                let exports: Vec<_> = target_info
                    .exports
                    .iter()
                    .filter(|&(name, _)| {
                        target_info
                            .items
                            .get(name)
                            .map(|(_, vis)| can_access(*vis))
                            .unwrap_or(false)
                    })
                    .map(|(name, def_id)| (format!("{}.{}", alias, name), *def_id))
                    .collect();

                Ok(exports)
            }
            ImportKind::Items(names) => {
                let mut result = Vec::new();
                for name in names {
                    if let Some(&def_id) = target_info.exports.get(name) {
                        if let Some((_, visibility)) = target_info.items.get(name) {
                            if can_access(*visibility) {
                                result.push((name.clone(), def_id));
                            } else {
                                return Err(ImportResolveError::PrivateItem(name.clone()));
                            }
                        }
                    } else {
                        return Err(ImportResolveError::ItemNotFound(name.clone()));
                    }
                }
                Ok(result)
            }
            ImportKind::All => {
                let exports: Vec<_> = target_info
                    .exports
                    .iter()
                    .filter(|(name, _)| {
                        target_info
                            .items
                            .get(*name)
                            .map(|(_, vis)| can_access(*vis))
                            .unwrap_or(false)
                    })
                    .map(|(name, &def_id)| (name.clone(), def_id))
                    .collect();
                Ok(exports)
            }
        }
    }

    /// Discover all modules in the project.
    /// 发现项目中的所有模块。
    pub fn discover_modules(&mut self) -> Result<Vec<ModuleId>, ModuleLoadError> {
        let mut discovered = Vec::new();

        // Start with lib.neve or main.neve
        // 从 lib.neve 或 main.neve 开始
        let root_dir = self.root_dir().to_path_buf();
        let _root_file = if root_dir.join("lib.neve").exists() {
            root_dir.join("lib.neve")
        } else if root_dir.join("main.neve").exists() {
            root_dir.join("main.neve")
        } else if root_dir.join("src/lib.neve").exists() {
            root_dir.join("src/lib.neve")
        } else if root_dir.join("src/main.neve").exists() {
            root_dir.join("src/main.neve")
        } else {
            return Err(ModuleLoadError::NoRootModule);
        };

        // Load the root module
        // 加载根模块
        let root_id = self.load_module(&[])?;
        discovered.push(root_id);

        // Recursively discover submodules
        // 递归发现子模块
        self.discover_submodules(&root_dir, &[], &mut discovered)?;

        Ok(discovered)
    }

    /// Recursively discover submodules in a directory.
    /// 递归发现目录中的子模块。
    fn discover_submodules(
        &mut self,
        dir: &Path,
        parent_path: &[String],
        discovered: &mut Vec<ModuleId>,
    ) -> Result<(), ModuleLoadError> {
        let entries = fs::read_dir(dir)
            .map_err(|e| ModuleLoadError::IoError(dir.to_path_buf(), e.to_string()))?;

        for entry in entries {
            let entry =
                entry.map_err(|e| ModuleLoadError::IoError(dir.to_path_buf(), e.to_string()))?;
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if path.is_file() && file_name.ends_with(".neve") && file_name != "mod.neve" {
                let module_name = file_name.trim_end_matches(".neve");
                let mut module_path = parent_path.to_vec();
                module_path.push(module_name.to_string());

                if let Ok(id) = self.load_module(&module_path) {
                    discovered.push(id);
                }
            } else if path.is_dir() && !file_name.starts_with('.') {
                // Check for mod.neve in subdirectory
                // 检查子目录中的 mod.neve
                let mod_file = path.join("mod.neve");
                if mod_file.exists() {
                    let mut module_path = parent_path.to_vec();
                    module_path.push(file_name.to_string());

                    if let Ok(id) = self.load_module(&module_path) {
                        discovered.push(id);
                    }

                    // Recurse into subdirectory
                    // 递归进入子目录
                    self.discover_submodules(&path, &module_path, discovered)?;
                }
            }
        }

        Ok(())
    }
}

/// Errors that can occur during module loading.
/// 模块加载期间可能发生的错误。
#[derive(Debug, Clone)]
pub enum ModuleLoadError {
    /// Module file not found. / 未找到模块文件。
    NotFound(Vec<String>),
    /// Circular dependency detected.
    /// 检测到循环依赖。
    CircularDependency {
        /// The module that caused the cycle. / 导致循环的模块。
        module: Vec<String>,
        /// The full import chain showing the cycle. / 显示循环的完整导入链。
        chain: Vec<Vec<String>>,
    },
    /// IO error reading file. / 读取文件时的 IO 错误。
    IoError(PathBuf, String),
    /// No root module found. / 未找到根模块。
    NoRootModule,
    /// Parse error in module. / 模块中的解析错误。
    ParseError(Vec<String>),
}

impl std::fmt::Display for ModuleLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleLoadError::NotFound(path) => {
                write!(f, "module not found: {}", path.join("."))
            }
            ModuleLoadError::CircularDependency { module, chain } => {
                writeln!(
                    f,
                    "circular dependency detected when importing module: {}",
                    module.join(".")
                )?;
                writeln!(f, "\nImport chain:")?;
                for (i, m) in chain.iter().enumerate() {
                    if i == chain.len() - 1 {
                        writeln!(f, "  {} -> {} (cycle!)", m.join("."), module.join("."))?;
                    } else {
                        writeln!(f, "  {}", m.join("."))?;
                    }
                }
                Ok(())
            }
            ModuleLoadError::IoError(path, msg) => {
                write!(f, "error reading {}: {}", path.display(), msg)
            }
            ModuleLoadError::NoRootModule => {
                write!(f, "no root module found (lib.neve or main.neve)")
            }
            ModuleLoadError::ParseError(path) => {
                write!(f, "parse error in module: {}", path.join("."))
            }
        }
    }
}

/// Errors that can occur during import resolution.
/// 导入解析期间可能发生的错误。
#[derive(Debug, Clone)]
pub enum ImportResolveError {
    /// Invalid import path. / 无效的导入路径。
    InvalidPath(Vec<String>),
    /// Module not found. / 未找到模块。
    ModuleNotFound(Vec<String>),
    /// Item not found in module. / 在模块中未找到项。
    ItemNotFound(String),
    /// Item is private. / 项是私有的。
    PrivateItem(String),
}

impl std::fmt::Display for ImportResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportResolveError::InvalidPath(path) => {
                write!(f, "invalid import path: {}", path.join("."))
            }
            ImportResolveError::ModuleNotFound(path) => {
                write!(f, "module not found: {}", path.join("."))
            }
            ImportResolveError::ItemNotFound(name) => {
                write!(f, "item not found: {}", name)
            }
            ImportResolveError::PrivateItem(name) => {
                write!(f, "item is private: {}", name)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circular_dependency_error_message() {
        // Test that circular dependency error includes the full chain
        // 测试循环依赖错误包含完整链
        let error = ModuleLoadError::CircularDependency {
            module: vec!["a".into()],
            chain: vec![vec!["a".into()], vec!["b".into()], vec!["c".into()]],
        };

        let message = format!("{}", error);
        assert!(message.contains("circular dependency"));
        assert!(message.contains("Import chain"));
        assert!(message.contains("(cycle!)"));
    }

    #[test]
    fn test_loading_stack_management() {
        let mut loader = ModuleLoader::new("/tmp");

        // Initially empty
        // 初始为空
        assert!(loader.loading.is_empty());
        assert!(loader.loading_stack.is_empty());

        // Simulate loading a module
        // 模拟加载模块
        let path = vec!["test".into()];
        loader.loading.insert(path.clone());
        loader.loading_stack.push(path.clone());

        assert!(loader.loading.contains(&path));
        assert_eq!(loader.loading_stack.len(), 1);

        // Detect cycle if trying to load the same module
        // 如果尝试加载同一模块则检测循环
        assert!(loader.loading.contains(&path));

        // Cleanup
        // 清理
        loader.loading.remove(&path);
        loader.loading_stack.pop();

        assert!(loader.loading.is_empty());
        assert!(loader.loading_stack.is_empty());
    }
}
