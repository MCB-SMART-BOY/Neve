//! Incremental cache and parsed-source storage for module loading.
//! 模块加载使用的增量缓存与解析源码存储。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use neve_diagnostic::Diagnostic;
use neve_syntax::SourceFile;

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
        hash_source(source) == self.source_hash
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

/// Cached parsed source by content hash.
/// 按内容哈希缓存的已解析源文件。
#[derive(Debug, Clone)]
struct ParsedSource {
    /// Cached parsed AST hash. / 缓存的 AST 哈希。
    source_hash: u64,
    /// Parsed source file. / 已解析的源文件。
    file: SourceFile,
    /// Parse diagnostics. / 解析诊断信息。
    diagnostics: Vec<Diagnostic>,
}

/// Incremental cache and parsed-source store used by module loading.
/// 模块加载使用的增量缓存与解析源码存储。
#[derive(Debug, Clone, Default)]
pub(crate) struct IncrementalCache {
    file_cache: HashMap<PathBuf, ModuleCache>,
    parsed_sources: HashMap<PathBuf, ParsedSource>,
    stats: CacheStats,
}

impl IncrementalCache {
    /// Record whether a file needs recompilation and update hit/miss stats.
    /// 记录文件是否需要重新编译并更新命中/未命中统计。
    pub(crate) fn record_recompile_check(&mut self, file_path: &Path) -> bool {
        let needs_recompile = match self.file_cache.get(file_path) {
            Some(cache) => cache.is_dirty() || !cache.is_valid(file_path),
            None => true,
        };

        if needs_recompile {
            self.stats.misses += 1;
        } else {
            self.stats.hits += 1;
        }

        needs_recompile
    }

    /// Parse source text using a content-addressed parsed-source cache.
    /// 使用基于内容寻址的解析缓存解析源码。
    pub(crate) fn parse_source(
        &mut self,
        file_path: &Path,
        source: &str,
    ) -> (SourceFile, Vec<Diagnostic>) {
        let source_hash = hash_source(source);

        if let Some(cached) = self.parsed_sources.get(file_path)
            && cached.source_hash == source_hash
        {
            return (cached.file.clone(), cached.diagnostics.clone());
        }

        let (file, diagnostics) = neve_parser::parse(source);
        self.parsed_sources.insert(
            file_path.to_path_buf(),
            ParsedSource {
                source_hash,
                file: file.clone(),
                diagnostics: diagnostics.clone(),
            },
        );
        (file, diagnostics)
    }

    /// Finish a successful load by updating cache entries and stats.
    /// 成功加载后更新缓存条目和统计。
    pub(crate) fn finish_load(&mut self, file_path: &Path, source: &str, needs_recompile: bool) {
        self.update_cache(file_path, source);
        if needs_recompile {
            self.stats.recompiled += 1;
        }
    }

    /// Get cache statistics. / 获取缓存统计信息。
    pub(crate) fn cache_stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Get the parsed source for a file path.
    /// 获取文件路径对应的解析源码。
    pub(crate) fn parsed_source(&self, file_path: &Path) -> Option<&SourceFile> {
        self.parsed_sources
            .get(file_path)
            .map(|parsed| &parsed.file)
    }

    /// Get parse diagnostics for a file path.
    /// 获取文件路径对应的解析诊断。
    pub(crate) fn parsed_diagnostics(&self, file_path: &Path) -> Option<&[Diagnostic]> {
        self.parsed_sources
            .get(file_path)
            .map(|parsed| parsed.diagnostics.as_slice())
    }

    /// Invalidate cache for a file. / 使文件缓存失效。
    pub(crate) fn invalidate_cache(&mut self, file_path: &Path) {
        self.file_cache.remove(file_path);
        self.parsed_sources.remove(file_path);
    }

    /// Clear all cache entries. / 清除所有缓存条目。
    pub(crate) fn clear(&mut self) {
        self.file_cache.clear();
        self.parsed_sources.clear();
        self.stats = CacheStats::default();
    }

    /// Get list of files that need recompilation.
    /// 获取需要重新编译的文件列表。
    pub(crate) fn get_dirty_files(&self) -> Vec<PathBuf> {
        self.file_cache
            .iter()
            .filter(|(path, cache)| cache.is_dirty() || !cache.is_valid(path))
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// Check if a file's content has changed using hash comparison.
    /// 使用哈希比较检查文件内容是否已更改。
    pub(crate) fn has_content_changed(&self, file_path: &Path) -> bool {
        if let Some(cache) = self.file_cache.get(file_path) {
            if let Ok(source) = fs::read_to_string(file_path) {
                !cache.is_valid_by_hash(&source)
            } else {
                true
            }
        } else {
            true
        }
    }

    /// Get cached modification time for a file.
    /// 获取文件缓存的修改时间。
    pub(crate) fn get_cached_mtime(&self, file_path: &Path) -> Option<SystemTime> {
        self.file_cache.get(file_path).map(|cache| cache.mtime())
    }

    /// Get cached source hash for a file.
    /// 获取文件缓存的源哈希。
    pub(crate) fn get_cached_hash(&self, file_path: &Path) -> Option<u64> {
        self.file_cache
            .get(file_path)
            .map(|cache| cache.source_hash())
    }

    /// Mark a file as dirty (needs recompilation).
    /// 将文件标记为脏（需要重新编译）。
    pub(crate) fn mark_file_dirty(&mut self, file_path: &Path) {
        if let Some(cache) = self.file_cache.get_mut(file_path) {
            cache.mark_dirty();
        }
    }

    /// Mark a file as clean after successful recompilation.
    /// 成功重新编译后将文件标记为干净。
    pub(crate) fn mark_file_clean(&mut self, file_path: &Path) {
        if let Some(cache) = self.file_cache.get_mut(file_path) {
            cache.mark_clean();
        }
    }

    fn update_cache(&mut self, file_path: &Path, source: &str) {
        if let Some(mtime) = get_mtime(file_path) {
            let hash = hash_source(source);
            if let Some(cache) = self.file_cache.get_mut(file_path) {
                cache.update(mtime, hash);
            } else {
                self.file_cache
                    .insert(file_path.to_path_buf(), ModuleCache::new(mtime, hash));
            }
        }
    }
}

/// Get file modification time. / 获取文件修改时间。
pub(crate) fn get_mtime(file_path: &Path) -> Option<SystemTime> {
    fs::metadata(file_path).ok().and_then(|m| m.modified().ok())
}

/// Simple hash of source content for cache validation.
/// 用于缓存验证的源内容简单哈希。
pub(crate) fn hash_source(source: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::IncrementalCache;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("neve_hir_incremental_{unique}_{name}"))
    }

    #[test]
    fn cache_stats_track_recompile_checks() {
        let file_path = temp_file_path("main.neve");
        fs::write(&file_path, "let x = 1;").unwrap();

        let mut cache = IncrementalCache::default();
        let needs_recompile = cache.record_recompile_check(&file_path);
        assert!(needs_recompile);
        cache.finish_load(&file_path, "let x = 1;", needs_recompile);

        let needs_recompile = cache.record_recompile_check(&file_path);
        assert!(!needs_recompile);
        assert_eq!(cache.cache_stats().misses, 1);
        assert_eq!(cache.cache_stats().hits, 1);
        assert_eq!(cache.cache_stats().recompiled, 1);

        let _ = fs::remove_file(&file_path);
    }

    #[test]
    fn parse_source_cache_retains_parsed_ast_and_diagnostics() {
        let file_path = temp_file_path("parse.neve");
        let mut cache = IncrementalCache::default();

        let (file, diagnostics) = cache.parse_source(&file_path, "let x = 1;");
        assert!(diagnostics.is_empty());
        assert_eq!(file.items.len(), 1);
        assert!(cache.parsed_source(&file_path).is_some());
        assert!(cache.parsed_diagnostics(&file_path).is_some());

        let (cached_file, cached_diagnostics) = cache.parse_source(&file_path, "let x = 1;");
        assert_eq!(cached_file.items.len(), 1);
        assert!(cached_diagnostics.is_empty());
    }
}
