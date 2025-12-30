//! Module loading and path resolution.
//! 模块加载和路径解析。
//!
//! This module provides functionality for:
//! 本模块提供以下功能：
//! - Discovering modules from file system / 从文件系统发现模块
//! - Resolving module paths (self, super, crate) / 解析模块路径（self、super、crate）
//! - Loading and caching modules / 加载和缓存模块
//! - Managing module dependencies / 管理模块依赖
//! - Incremental compilation with file timestamps / 基于文件时间戳的增量编译

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use neve_diagnostic::Diagnostic;

use crate::{DefId, Import, ImportKind, ModuleId};

/// Represents a module path in the source code.
/// 表示源代码中的模块路径。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModulePath {
    /// Path segments (e.g., ["std", "list"] for `std.list`). / 路径段（例如 `std.list` 对应 ["std", "list"]）。
    pub segments: Vec<String>,
    /// Whether this is a relative path (starts with self or super). / 是否为相对路径（以 self 或 super 开头）。
    pub kind: ModulePathKind,
}

/// Kind of module path.
/// 模块路径的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModulePathKind {
    /// Absolute path from crate root (e.g., `std.list`). / 从 crate 根开始的绝对路径（例如 `std.list`）。
    Absolute,
    /// Relative to current module (e.g., `self.utils`). / 相对于当前模块（例如 `self.utils`）。
    Self_,
    /// Relative to parent module (e.g., `super.common`). / 相对于父模块（例如 `super.common`）。
    Super,
    /// Relative to crate root (e.g., `crate.config`). / 相对于 crate 根（例如 `crate.config`）。
    Crate,
}

impl ModulePath {
    /// Create an absolute module path.
    /// 创建绝对模块路径。
    pub fn absolute(segments: Vec<String>) -> Self {
        Self {
            segments,
            kind: ModulePathKind::Absolute,
        }
    }

    /// Create a self-relative module path.
    /// 创建 self 相对模块路径。
    pub fn self_(segments: Vec<String>) -> Self {
        Self {
            segments,
            kind: ModulePathKind::Self_,
        }
    }

    /// Create a super-relative module path.
    /// 创建 super 相对模块路径。
    pub fn super_(segments: Vec<String>) -> Self {
        Self {
            segments,
            kind: ModulePathKind::Super,
        }
    }

    /// Create a crate-relative module path.
    /// 创建 crate 相对模块路径。
    pub fn crate_(segments: Vec<String>) -> Self {
        Self {
            segments,
            kind: ModulePathKind::Crate,
        }
    }

    /// Create a module path from an AST import definition.
    /// 从 AST 导入定义创建模块路径。
    pub fn from_import_def(import: &neve_syntax::ImportDef) -> Self {
        let segments: Vec<String> = import.path.iter().map(|i| i.name.clone()).collect();
        match import.prefix {
            neve_syntax::PathPrefix::Absolute => Self::absolute(segments),
            neve_syntax::PathPrefix::Self_ => Self::self_(segments),
            neve_syntax::PathPrefix::Super => Self::super_(segments),
            neve_syntax::PathPrefix::Crate => Self::crate_(segments),
        }
    }

    /// Create a module path from a HIR import.
    /// 从 HIR 导入创建模块路径。
    pub fn from_hir_import(import: &crate::Import) -> Self {
        match import.prefix {
            crate::ImportPathPrefix::Absolute => Self::absolute(import.path.clone()),
            crate::ImportPathPrefix::Self_ => Self::self_(import.path.clone()),
            crate::ImportPathPrefix::Super => Self::super_(import.path.clone()),
            crate::ImportPathPrefix::Crate => Self::crate_(import.path.clone()),
        }
    }

    /// Parse a module path from import path segments (legacy, infers prefix from first segment).
    /// 从导入路径段解析模块路径（遗留方式，从第一个段推断前缀）。
    pub fn from_import_path(segments: &[String]) -> Self {
        if segments.is_empty() {
            return Self::absolute(Vec::new());
        }

        match segments[0].as_str() {
            "self" => Self::self_(segments[1..].to_vec()),
            "super" => {
                // Handle super path - remaining segments after "super"
                // 处理 super 路径 - "super" 之后的剩余段
                Self::super_(segments[1..].to_vec())
            }
            "crate" => Self::crate_(segments[1..].to_vec()),
            _ => Self::absolute(segments.to_vec()),
        }
    }
}

impl std::fmt::Display for ModulePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = match self.kind {
            ModulePathKind::Absolute => "",
            ModulePathKind::Self_ => "self.",
            ModulePathKind::Super => "super.",
            ModulePathKind::Crate => "crate.",
        };
        write!(f, "{}{}", prefix, self.segments.join("."))
    }
}

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
    /// Exported items (name -> DefId). / 导出的项（名称 -> DefId）。
    pub exports: HashMap<String, DefId>,
    /// All items with visibility. / 所有带有可见性的项。
    pub items: HashMap<String, (DefId, Visibility)>,
    /// File modification time for incremental compilation.
    /// 用于增量编译的文件修改时间。
    pub mtime: Option<SystemTime>,
}

/// Cache entry for incremental compilation.
/// 用于增量编译的缓存条目。
#[derive(Debug, Clone)]
pub struct ModuleCache {
    /// Modification time when cached. / 缓存时的修改时间。
    pub mtime: SystemTime,
    /// Cached parsed AST hash (for content-based invalidation).
    /// 缓存的已解析 AST 哈希（用于基于内容的失效）。
    pub source_hash: u64,
    /// Whether the module needs recompilation. / 模块是否需要重新编译。
    pub dirty: bool,
}

impl ModuleCache {
    /// Create a new cache entry. / 创建新的缓存条目。
    pub fn new(mtime: SystemTime, source_hash: u64) -> Self {
        Self {
            mtime,
            source_hash,
            dirty: false,
        }
    }

    /// Check if the cache is valid for the given file.
    /// 检查缓存对于给定文件是否有效。
    pub fn is_valid(&self, file_path: &Path) -> bool {
        if self.dirty {
            return false;
        }
        if let Ok(metadata) = fs::metadata(file_path)
            && let Ok(mtime) = metadata.modified()
        {
            return self.mtime == mtime;
        }
        false
    }

    /// Check if the cache is valid using content hash (for when mtime is unreliable).
    /// 使用内容哈希检查缓存是否有效（用于 mtime 不可靠时）。
    pub fn is_valid_by_hash(&self, source: &str) -> bool {
        if self.dirty {
            return false;
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        hasher.finish() == self.source_hash
    }

    /// Mark cache entry as dirty (needs recompilation).
    /// 将缓存条目标记为脏（需要重新编译）。
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Clear the dirty flag after successful recompilation.
    /// 成功重新编译后清除脏标志。
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Check if cache entry is dirty. / 检查缓存条目是否为脏。
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get the source hash (for content-based cache invalidation).
    /// 获取源哈希（用于基于内容的缓存失效）。
    pub fn source_hash(&self) -> u64 {
        self.source_hash
    }

    /// Get the modification time. / 获取修改时间。
    pub fn mtime(&self) -> SystemTime {
        self.mtime
    }

    /// Update the cache with new mtime and hash.
    /// 使用新的 mtime 和哈希更新缓存。
    pub fn update(&mut self, mtime: SystemTime, source_hash: u64) {
        self.mtime = mtime;
        self.source_hash = source_hash;
        self.dirty = false;
    }
}

/// Module loader responsible for discovering and loading modules.
/// 负责发现和加载模块的模块加载器。
pub struct ModuleLoader {
    /// Root directory for source files. / 源文件的根目录。
    root_dir: PathBuf,
    /// All loaded modules. / 所有已加载的模块。
    modules: HashMap<ModuleId, ModuleInfo>,
    /// Module lookup by path. / 按路径查找模块。
    path_to_id: HashMap<Vec<String>, ModuleId>,
    /// Module lookup by file path. / 按文件路径查找模块。
    file_to_id: HashMap<PathBuf, ModuleId>,
    /// Next module ID. / 下一个模块 ID。
    next_id: u32,
    /// Standard library path (if available). / 标准库路径（如果可用）。
    std_path: Option<PathBuf>,
    /// Diagnostics collected during loading. / 加载期间收集的诊断信息。
    diagnostics: Vec<Diagnostic>,
    /// Modules currently being loaded (for cycle detection).
    /// Maps module path to its loading stack for detailed error messages.
    /// 当前正在加载的模块（用于循环检测）。
    /// 将模块路径映射到其加载栈以获取详细的错误消息。
    loading: HashSet<Vec<String>>,
    /// Loading stack to track the import chain.
    /// 加载栈用于跟踪导入链。
    loading_stack: Vec<Vec<String>>,
    /// Cache for incremental compilation (file path -> cache entry).
    /// 用于增量编译的缓存（文件路径 -> 缓存条目）。
    file_cache: HashMap<PathBuf, ModuleCache>,
    /// Statistics for cache hits/misses. / 缓存命中/未命中统计。
    cache_stats: CacheStats,
}

/// Statistics for incremental compilation cache.
/// 增量编译缓存的统计信息。
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits. / 缓存命中次数。
    pub hits: usize,
    /// Number of cache misses. / 缓存未命中次数。
    pub misses: usize,
    /// Number of modules recompiled. / 重新编译的模块数。
    pub recompiled: usize,
}

impl ModuleLoader {
    /// Create a new module loader with the given root directory.
    /// 使用给定的根目录创建新的模块加载器。
    pub fn new(root_dir: impl AsRef<Path>) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
            modules: HashMap::new(),
            path_to_id: HashMap::new(),
            file_to_id: HashMap::new(),
            next_id: 0,
            std_path: None,
            diagnostics: Vec::new(),
            loading: HashSet::new(),
            loading_stack: Vec::new(),
            file_cache: HashMap::new(),
            cache_stats: CacheStats::default(),
        }
    }

    /// Set the standard library path.
    /// 设置标准库路径。
    pub fn with_std_path(mut self, path: impl AsRef<Path>) -> Self {
        self.std_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Get the root directory.
    /// 获取根目录。
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Get collected diagnostics.
    /// 获取收集的诊断信息。
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Take collected diagnostics.
    /// 取出收集的诊断信息。
    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Get cache statistics. / 获取缓存统计信息。
    pub fn cache_stats(&self) -> &CacheStats {
        &self.cache_stats
    }

    /// Check if a file needs recompilation based on modification time or dirty flag.
    /// 根据修改时间或脏标志检查文件是否需要重新编译。
    fn needs_recompile(&self, file_path: &Path) -> bool {
        if let Some(cache) = self.file_cache.get(file_path) {
            // Check dirty flag first, then mtime-based validation
            // 首先检查脏标志，然后检查基于 mtime 的验证
            cache.is_dirty() || !cache.is_valid(file_path)
        } else {
            true // No cache entry, needs compilation / 没有缓存条目，需要编译
        }
    }

    /// Get file modification time. / 获取文件修改时间。
    fn get_mtime(file_path: &Path) -> Option<SystemTime> {
        fs::metadata(file_path)
            .ok()
            .and_then(|m| m.modified().ok())
    }

    /// Simple hash of source content for cache validation.
    /// 用于缓存验证的源内容简单哈希。
    fn hash_source(source: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        hasher.finish()
    }

    /// Update cache entry for a file. / 更新文件的缓存条目。
    fn update_cache(&mut self, file_path: &Path, source: &str) {
        if let Some(mtime) = Self::get_mtime(file_path) {
            let hash = Self::hash_source(source);
            // Use update() if cache entry exists, otherwise create new
            // 如果缓存条目存在则使用 update()，否则创建新条目
            if let Some(cache) = self.file_cache.get_mut(file_path) {
                cache.update(mtime, hash);
            } else {
                self.file_cache
                    .insert(file_path.to_path_buf(), ModuleCache::new(mtime, hash));
            }
        }
    }

    /// Invalidate cache for a file. / 使文件的缓存失效。
    pub fn invalidate_cache(&mut self, file_path: &Path) {
        self.file_cache.remove(file_path);
    }

    /// Clear all cache entries. / 清除所有缓存条目。
    pub fn clear_cache(&mut self) {
        self.file_cache.clear();
        self.cache_stats = CacheStats::default();
    }

    /// Get list of files that need recompilation.
    /// 获取需要重新编译的文件列表。
    pub fn get_dirty_files(&self) -> Vec<PathBuf> {
        self.file_cache
            .iter()
            .filter(|(path, cache)| cache.is_dirty() || !cache.is_valid(path))
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// Check if a file's content has changed using hash comparison.
    /// Returns true if content has changed or file cannot be read.
    /// 使用哈希比较检查文件内容是否已更改。
    /// 如果内容已更改或文件无法读取，则返回 true。
    pub fn has_content_changed(&self, file_path: &Path) -> bool {
        if let Some(cache) = self.file_cache.get(file_path) {
            if let Ok(source) = fs::read_to_string(file_path) {
                !cache.is_valid_by_hash(&source)
            } else {
                true // Cannot read file, assume changed / 无法读取文件，假定已更改
            }
        } else {
            true // No cache entry / 没有缓存条目
        }
    }

    /// Get cached modification time for a file.
    /// 获取文件的缓存修改时间。
    pub fn get_cached_mtime(&self, file_path: &Path) -> Option<SystemTime> {
        self.file_cache.get(file_path).map(|cache| cache.mtime())
    }

    /// Get cached source hash for a file.
    /// 获取文件的缓存源哈希。
    pub fn get_cached_hash(&self, file_path: &Path) -> Option<u64> {
        self.file_cache
            .get(file_path)
            .map(|cache| cache.source_hash())
    }

    /// Mark a file as dirty (needs recompilation).
    /// 将文件标记为脏（需要重新编译）。
    pub fn mark_file_dirty(&mut self, file_path: &Path) {
        if let Some(cache) = self.file_cache.get_mut(file_path) {
            cache.mark_dirty();
        }
    }

    /// Mark a file as clean after successful recompilation.
    /// 成功重新编译后将文件标记为干净。
    pub fn mark_file_clean(&mut self, file_path: &Path) {
        if let Some(cache) = self.file_cache.get_mut(file_path) {
            cache.mark_clean();
        }
    }

    /// Mark all dependents of a file as dirty (for incremental recompilation).
    /// This marks the file itself and all files in the same module tree as needing recompilation.
    /// 将文件的所有依赖项标记为脏（用于增量重新编译）。
    /// 这会将文件本身及同一模块树中的所有文件标记为需要重新编译。
    pub fn invalidate_dependents(&mut self, file_path: &Path) {
        // Find the module for this file path
        // 查找此文件路径对应的模块
        let module_id = self
            .modules
            .iter()
            .find(|(_, info)| info.file_path == file_path)
            .map(|(id, _)| *id);

        if let Some(module_id) = module_id {
            // Mark all child modules as dirty (they may depend on parent)
            // 将所有子模块标记为脏（它们可能依赖于父模块）
            let dependents: Vec<PathBuf> = self
                .modules
                .iter()
                .filter(|(_, info)| info.parent == Some(module_id))
                .map(|(_, info)| info.file_path.clone())
                .collect();

            for dependent_path in dependents {
                self.mark_file_dirty(&dependent_path);
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
        let absolute_path = self.make_absolute(path, from_module)?;
        self.find_module_file(&absolute_path)
    }

    /// Convert a relative path to an absolute path.
    /// 将相对路径转换为绝对路径。
    fn make_absolute(
        &self,
        path: &ModulePath,
        from_module: Option<&[String]>,
    ) -> Option<Vec<String>> {
        match path.kind {
            ModulePathKind::Absolute => Some(path.segments.clone()),
            ModulePathKind::Crate => Some(path.segments.clone()),
            ModulePathKind::Self_ => {
                let mut result = from_module?.to_vec();
                result.extend(path.segments.iter().cloned());
                Some(result)
            }
            ModulePathKind::Super => {
                let from = from_module?;
                if from.len() < 2 {
                    return None; // Can't go above root or single-level module / 无法超出根目录或单级模块
                }
                // Go up two levels: remove current file and then go to parent directory
                // 上溯两级：移除当前文件然后转到父目录
                // E.g., from ["mylib", "submod", "worker"] -> ["mylib"]
                // 例如，从 ["mylib", "submod", "worker"] -> ["mylib"]
                let mut result = from[..from.len() - 2].to_vec();

                // Handle multiple super or additional path segments
                // 处理多个 super 或附加路径段
                for seg in &path.segments {
                    if seg == "super" {
                        if result.is_empty() {
                            return None;
                        }
                        result.pop();
                    } else {
                        result.push(seg.clone());
                    }
                }
                Some(result)
            }
        }
    }

    /// Find the file path for a module path.
    /// 查找模块路径对应的文件路径。
    fn find_module_file(&self, module_path: &[String]) -> Option<PathBuf> {
        if module_path.is_empty() {
            return Some(self.root_dir.join("lib.neve"));
        }

        // Check if it's a standard library module
        // 检查是否为标准库模块
        if module_path.first().map(|s| s.as_str()) == Some("std")
            && let Some(std_path) = &self.std_path
        {
            let relative: PathBuf = module_path[1..].iter().collect();

            // Try module_name.neve
            // 尝试 module_name.neve
            let file_path = std_path.join(&relative).with_extension("neve");
            if file_path.exists() {
                return Some(file_path);
            }

            // Try module_name/mod.neve
            // 尝试 module_name/mod.neve
            let mod_path = std_path.join(&relative).join("mod.neve");
            if mod_path.exists() {
                return Some(mod_path);
            }
        }

        // Build relative path
        // 构建相对路径
        let relative: PathBuf = module_path.iter().collect();

        // Try module_name.neve
        // 尝试 module_name.neve
        let file_path = self.root_dir.join(&relative).with_extension("neve");
        if file_path.exists() {
            return Some(file_path);
        }

        // Try module_name/mod.neve
        // 尝试 module_name/mod.neve
        let mod_path = self.root_dir.join(&relative).join("mod.neve");
        if mod_path.exists() {
            return Some(mod_path);
        }

        // Try src/module_name.neve
        // 尝试 src/module_name.neve
        let src_path = self
            .root_dir
            .join("src")
            .join(&relative)
            .with_extension("neve");
        if src_path.exists() {
            return Some(src_path);
        }

        None
    }

    /// Load a module by path.
    /// 按路径加载模块。
    pub fn load_module(&mut self, path: &[String]) -> Result<ModuleId, ModuleLoadError> {
        // Check if already loaded
        // 检查是否已加载
        if let Some(&id) = self.path_to_id.get(path) {
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
        let _module_path = ModulePath::absolute(path.to_vec());
        let file_path = self
            .find_module_file(path)
            .ok_or_else(|| ModuleLoadError::NotFound(path.to_vec()))?;

        // Mark as loading and add to stack
        // 标记为正在加载并添加到栈
        self.loading.insert(path.to_vec());
        self.loading_stack.push(path.to_vec());

        // Check if file needs recompilation (incremental compilation)
        // 检查文件是否需要重新编译（增量编译）
        let needs_recompile = self.needs_recompile(&file_path);
        if needs_recompile {
            self.cache_stats.misses += 1;
        } else {
            self.cache_stats.hits += 1;
        }

        // Read and parse the file
        // 读取并解析文件
        let source = fs::read_to_string(&file_path)
            .map_err(|e| ModuleLoadError::IoError(file_path.clone(), e.to_string()))?;

        // Parse the source
        // 解析源代码
        let (source_file, parse_errors) = neve_parser::parse(&source);

        // Update cache with new mtime and source hash
        // 使用新的 mtime 和源哈希更新缓存
        self.update_cache(&file_path, &source);

        // Track recompilation stats / 跟踪重新编译统计
        if needs_recompile {
            self.cache_stats.recompiled += 1;
        }

        // Collect parse errors
        // 收集解析错误
        for error in parse_errors {
            self.diagnostics.push(error);
        }

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
                if let Some(abs_path) = self.make_absolute(&import_path, Some(path))
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

        // Create module info
        // 创建模块信息
        let mtime = Self::get_mtime(&file_path);
        let info = ModuleInfo {
            id: module_id,
            path: path.to_vec(),
            file_path: file_path.clone(),
            parent: self.find_parent_module(path),
            children: Vec::new(),
            exports: HashMap::new(),
            items: HashMap::new(),
            mtime,
        };

        // Register the module as loaded (only after dependencies are loaded)
        // 将模块注册为已加载（仅在依赖加载后）
        self.modules.insert(module_id, info);
        self.path_to_id.insert(path.to_vec(), module_id);
        self.file_to_id.insert(file_path, module_id);

        // Update parent's children list
        // 更新父模块的子模块列表
        if let Some(parent_id) = self.find_parent_module(path)
            && let Some(parent_info) = self.modules.get_mut(&parent_id)
        {
            parent_info.children.push(module_id);
        }

        // Remove from loading set and stack
        // 从加载集和栈中移除
        self.loading.remove(path);
        self.loading_stack.pop();

        Ok(module_id)
    }

    /// Find the parent module for a given path.
    /// 查找给定路径的父模块。
    fn find_parent_module(&self, path: &[String]) -> Option<ModuleId> {
        if path.len() <= 1 {
            return None;
        }
        self.path_to_id.get(&path[..path.len() - 1]).copied()
    }

    /// Get module info by ID.
    /// 按 ID 获取模块信息。
    pub fn get_module(&self, id: ModuleId) -> Option<&ModuleInfo> {
        self.modules.get(&id)
    }

    /// Get mutable module info by ID.
    /// 按 ID 获取可变模块信息。
    pub fn get_module_mut(&mut self, id: ModuleId) -> Option<&mut ModuleInfo> {
        self.modules.get_mut(&id)
    }

    /// Look up a module by path.
    /// 按路径查找模块。
    pub fn lookup_module(&self, path: &[String]) -> Option<ModuleId> {
        self.path_to_id.get(path).copied()
    }

    /// Get all loaded modules.
    /// 获取所有已加载的模块。
    pub fn all_modules(&self) -> impl Iterator<Item = (&Vec<String>, &ModuleInfo)> {
        self.path_to_id
            .iter()
            .filter_map(|(path, &id)| self.modules.get(&id).map(|info| (path, info)))
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
        if let Some(info) = self.modules.get_mut(&module_id) {
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
            .make_absolute(&import_path, Some(from_module))
            .ok_or_else(|| ImportResolveError::InvalidPath(import.path.clone()))?;

        let target_id = self
            .path_to_id
            .get(&target_path)
            .ok_or_else(|| ImportResolveError::ModuleNotFound(target_path.clone()))?;

        let target_info = self
            .modules
            .get(target_id)
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
                        && let Some(parent_info) = self.modules.get(parent)
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
        let _root_file = if self.root_dir.join("lib.neve").exists() {
            self.root_dir.join("lib.neve")
        } else if self.root_dir.join("main.neve").exists() {
            self.root_dir.join("main.neve")
        } else if self.root_dir.join("src/lib.neve").exists() {
            self.root_dir.join("src/lib.neve")
        } else if self.root_dir.join("src/main.neve").exists() {
            self.root_dir.join("src/main.neve")
        } else {
            return Err(ModuleLoadError::NoRootModule);
        };

        // Load the root module
        // 加载根模块
        let root_id = self.load_module(&[])?;
        discovered.push(root_id);

        // Recursively discover submodules
        // 递归发现子模块
        self.discover_submodules(&self.root_dir.clone(), &[], &mut discovered)?;

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
    fn test_module_path_parsing() {
        let path = ModulePath::from_import_path(&["std".into(), "list".into()]);
        assert_eq!(path.kind, ModulePathKind::Absolute);
        assert_eq!(path.segments, vec!["std", "list"]);

        let path = ModulePath::from_import_path(&["self".into(), "utils".into()]);
        assert_eq!(path.kind, ModulePathKind::Self_);
        assert_eq!(path.segments, vec!["utils"]);

        let path = ModulePath::from_import_path(&["super".into(), "common".into()]);
        assert_eq!(path.kind, ModulePathKind::Super);
        assert_eq!(path.segments, vec!["common"]);

        let path = ModulePath::from_import_path(&["crate".into(), "config".into()]);
        assert_eq!(path.kind, ModulePathKind::Crate);
        assert_eq!(path.segments, vec!["config"]);
    }

    #[test]
    fn test_make_absolute() {
        let loader = ModuleLoader::new("/tmp");

        // Absolute path stays the same
        // 绝对路径保持不变
        let path = ModulePath::absolute(vec!["std".into(), "list".into()]);
        let result = loader.make_absolute(&path, Some(&["mymod".into()]));
        assert_eq!(result, Some(vec!["std".into(), "list".into()]));

        // Self-relative path
        // self 相对路径
        let path = ModulePath::self_(vec!["utils".into()]);
        let result = loader.make_absolute(&path, Some(&["mymod".into()]));
        assert_eq!(result, Some(vec!["mymod".into(), "utils".into()]));

        // Super-relative path (goes up two levels: current file and parent directory)
        // super 相对路径（向上两级：当前文件和父目录）
        // From ["mylib", "submod", "worker"], super.config resolves to ["mylib", "config"]
        // 从 ["mylib", "submod", "worker"]，super.config 解析为 ["mylib", "config"]
        let path = ModulePath::super_(vec!["common".into()]);
        let result = loader.make_absolute(
            &path,
            Some(&["parent".into(), "child".into(), "file".into()]),
        );
        assert_eq!(result, Some(vec!["parent".into(), "common".into()]));
    }

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
