//! Garbage collection for the store.
//! 存储的垃圾回收。
//!
//! Garbage collection removes paths that are no longer reachable from
//! any GC root.
//! 垃圾回收移除从任何 GC 根不再可达的路径。

use crate::{Store, StoreError};
use neve_derive::StorePath;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// GC roots directory.
/// GC 根目录。
const GC_ROOTS_DIR: &str = "gcroots";

/// Garbage collector for the store.
/// 存储的垃圾回收器。
pub struct GarbageCollector<'a> {
    store: &'a mut Store,
}

impl<'a> GarbageCollector<'a> {
    /// Create a new garbage collector.
    /// 创建新的垃圾回收器。
    pub fn new(store: &'a mut Store) -> Self {
        Self { store }
    }

    /// Get the GC roots directory.
    /// 获取 GC 根目录。
    fn roots_dir(&self) -> PathBuf {
        self.store.root().join(GC_ROOTS_DIR)
    }

    /// Add a GC root.
    /// 添加 GC 根。
    pub fn add_root(&self, name: &str, path: &StorePath) -> Result<(), StoreError> {
        let roots_dir = self.roots_dir();
        fs::create_dir_all(&roots_dir)?;

        let link_path = roots_dir.join(name);
        let target = self.store.to_path(path);

        // Remove existing link if present
        // 如果存在则移除现有链接
        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path)?;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link_path)?;

        #[cfg(not(unix))]
        fs::write(&link_path, target.to_string_lossy().as_bytes())?;

        Ok(())
    }

    /// Remove a GC root.
    /// 移除 GC 根。
    pub fn remove_root(&self, name: &str) -> Result<(), StoreError> {
        let link_path = self.roots_dir().join(name);
        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path)?;
        }
        Ok(())
    }

    /// List all GC roots.
    /// 列出所有 GC 根。
    pub fn list_roots(&self) -> Result<Vec<(String, StorePath)>, StoreError> {
        let roots_dir = self.roots_dir();
        if !roots_dir.exists() {
            return Ok(Vec::new());
        }

        let mut roots = Vec::new();
        for entry in fs::read_dir(&roots_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();

            let target = if path.is_symlink() {
                fs::read_link(&path)?
            } else {
                PathBuf::from(String::from_utf8_lossy(&fs::read(&path)?).into_owned())
            };

            if let Some(store_path) = StorePath::parse(&target) {
                roots.push((name, store_path));
            }
        }

        Ok(roots)
    }

    /// Find all paths reachable from the GC roots.
    /// 查找从 GC 根可达的所有路径。
    pub fn find_live_paths(&mut self) -> Result<HashSet<StorePath>, StoreError> {
        let roots = self.list_roots()?;
        let mut live = HashSet::new();

        for (_, root_path) in roots {
            self.add_reachable(&root_path, &mut live)?;
        }

        Ok(live)
    }

    /// Add a path and all its references to the live set.
    /// 将路径及其所有引用添加到存活集合。
    fn add_reachable(
        &mut self,
        path: &StorePath,
        live: &mut HashSet<StorePath>,
    ) -> Result<(), StoreError> {
        if live.contains(path) {
            return Ok(());
        }

        if !self.store.path_exists(path) {
            return Ok(());
        }

        live.insert(path.clone());

        // If it's a derivation, add its inputs
        // 如果是推导，添加其输入
        if path.name().ends_with(".drv")
            && let Ok(drv) = self.store.read_derivation(path)
        {
            // Collect the paths first to avoid borrow issues
            // 首先收集路径以避免借用问题
            let input_drvs: Vec<_> = drv.input_drvs.keys().cloned().collect();
            let input_srcs: Vec<_> = drv.input_srcs.clone();

            for input_drv in input_drvs {
                self.add_reachable(&input_drv, live)?;
            }
            for input_src in input_srcs {
                self.add_reachable(&input_src, live)?;
            }
        }

        Ok(())
    }

    /// Collect garbage and return the number of paths deleted.
    /// 收集垃圾并返回删除的路径数量。
    pub fn collect(&mut self) -> Result<GcResult, StoreError> {
        let live = self.find_live_paths()?;
        let all_paths = self.store.list_paths()?;

        let mut deleted = 0;
        let mut freed_bytes = 0u64;

        for path in all_paths {
            if !live.contains(&path) {
                let fs_path = self.store.to_path(&path);
                if let Ok(size) = dir_size(&fs_path) {
                    freed_bytes += size;
                }
                self.store.delete(&path)?;
                deleted += 1;
            }
        }

        Ok(GcResult {
            deleted,
            freed_bytes,
        })
    }

    /// Dry-run garbage collection and return what would be deleted.
    /// 干运行垃圾回收并返回将被删除的内容。
    pub fn dry_run(&mut self) -> Result<Vec<StorePath>, StoreError> {
        let live = self.find_live_paths()?;
        let all_paths = self.store.list_paths()?;

        Ok(all_paths
            .into_iter()
            .filter(|p| !live.contains(p))
            .collect())
    }
}

/// Result of garbage collection.
/// 垃圾回收的结果。
#[derive(Debug, Clone)]
pub struct GcResult {
    /// Number of paths deleted. / 删除的路径数量。
    pub deleted: usize,
    /// Total bytes freed. / 释放的总字节数。
    pub freed_bytes: u64,
}

impl GcResult {
    /// Format freed bytes as a human-readable string.
    /// 将释放的字节格式化为人类可读的字符串。
    pub fn freed_human(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.freed_bytes >= GB {
            format!("{:.2} GiB", self.freed_bytes as f64 / GB as f64)
        } else if self.freed_bytes >= MB {
            format!("{:.2} MiB", self.freed_bytes as f64 / MB as f64)
        } else if self.freed_bytes >= KB {
            format!("{:.2} KiB", self.freed_bytes as f64 / KB as f64)
        } else {
            format!("{} B", self.freed_bytes)
        }
    }
}

/// Calculate directory size.
/// 计算目录大小。
fn dir_size(path: &Path) -> Result<u64, StoreError> {
    let mut size = 0;

    if path.is_file() {
        return Ok(fs::metadata(path)?.len());
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            size += dir_size(&entry.path())?;
        }
    }

    Ok(size)
}
