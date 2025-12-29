//! Metadata database for the store.
//! 存储的元数据数据库。
//!
//! Stores information about derivations, their outputs, and references.
//! 存储有关推导、其输出和引用的信息。

use crate::StoreError;
use neve_derive::{Hash, StorePath};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// Metadata about a store path.
/// 存储路径的元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    /// The store path. / 存储路径。
    pub path: StorePath,
    /// Hash of the path contents. / 路径内容的哈希。
    pub nar_hash: Hash,
    /// Size of the path in bytes. / 路径的大小（字节）。
    pub nar_size: u64,
    /// Paths that this path references. / 此路径引用的路径。
    pub references: HashSet<StorePath>,
    /// The derivation that produced this path (if any). / 产生此路径的推导（如有）。
    pub deriver: Option<StorePath>,
    /// Registration time (Unix timestamp). / 注册时间（Unix 时间戳）。
    pub registration_time: u64,
    /// Whether this is a valid path. / 是否为有效路径。
    pub valid: bool,
}

impl PathInfo {
    /// Create a new PathInfo.
    /// 创建新的 PathInfo。
    pub fn new(path: StorePath, nar_hash: Hash, nar_size: u64) -> Self {
        Self {
            path,
            nar_hash,
            nar_size,
            references: HashSet::new(),
            deriver: None,
            registration_time: current_time(),
            valid: true,
        }
    }

    /// Add a reference.
    /// 添加引用。
    pub fn add_reference(&mut self, path: StorePath) {
        self.references.insert(path);
    }

    /// Set the deriver.
    /// 设置推导器。
    pub fn set_deriver(&mut self, drv: StorePath) {
        self.deriver = Some(drv);
    }
}

/// The metadata database.
/// 元数据数据库。
pub struct Database {
    /// Root directory for the database. / 数据库的根目录。
    root: PathBuf,
    /// Cached path info. / 缓存的路径信息。
    cache: HashMap<StorePath, PathInfo>,
}

impl Database {
    /// Get the root directory of the database.
    /// 获取数据库的根目录。
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Open the database at the given root.
    /// 在给定的根目录打开数据库。
    pub fn open(root: PathBuf) -> Result<Self, StoreError> {
        let db_dir = root.join("db");
        fs::create_dir_all(&db_dir)?;

        Ok(Self {
            root: db_dir,
            cache: HashMap::new(),
        })
    }

    /// Get the path to the info file for a store path.
    /// 获取存储路径的信息文件路径。
    fn info_path(&self, store_path: &StorePath) -> PathBuf {
        self.root.join(format!("{}.json", store_path.hash()))
    }

    /// Register a path in the database.
    /// 在数据库中注册路径。
    pub fn register(&mut self, info: PathInfo) -> Result<(), StoreError> {
        let path = self.info_path(&info.path);
        let json = serde_json::to_string_pretty(&info)?;
        fs::write(&path, json)?;
        self.cache.insert(info.path.clone(), info);
        Ok(())
    }

    /// Query path info.
    /// 查询路径信息。
    pub fn query(&mut self, store_path: &StorePath) -> Result<Option<PathInfo>, StoreError> {
        if let Some(info) = self.cache.get(store_path) {
            return Ok(Some(info.clone()));
        }

        let path = self.info_path(store_path);
        if !path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&path)?;
        let info: PathInfo = serde_json::from_str(&json)?;
        self.cache.insert(store_path.clone(), info.clone());

        Ok(Some(info))
    }

    /// Check if a path is valid (registered and exists).
    /// 检查路径是否有效（已注册且存在）。
    pub fn is_valid(&mut self, store_path: &StorePath) -> Result<bool, StoreError> {
        Ok(self.query(store_path)?.map(|i| i.valid).unwrap_or(false))
    }

    /// Get all references of a path.
    /// 获取路径的所有引用。
    pub fn get_references(
        &mut self,
        store_path: &StorePath,
    ) -> Result<HashSet<StorePath>, StoreError> {
        Ok(self
            .query(store_path)?
            .map(|i| i.references)
            .unwrap_or_default())
    }

    /// Get paths that reference the given path (referrers).
    /// 获取引用给定路径的路径（引用者）。
    pub fn get_referrers(
        &mut self,
        store_path: &StorePath,
    ) -> Result<HashSet<StorePath>, StoreError> {
        let mut referrers = HashSet::new();

        // Scan all info files (inefficient, but simple)
        // 扫描所有信息文件（低效但简单）
        if !self.root.exists() {
            return Ok(referrers);
        }

        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|e| e == "json") {
                let json = fs::read_to_string(entry.path())?;
                if let Ok(info) = serde_json::from_str::<PathInfo>(&json)
                    && info.references.contains(store_path)
                {
                    referrers.insert(info.path);
                }
            }
        }

        Ok(referrers)
    }

    /// Delete path info from the database.
    /// 从数据库中删除路径信息。
    pub fn delete(&mut self, store_path: &StorePath) -> Result<(), StoreError> {
        let path = self.info_path(store_path);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        self.cache.remove(store_path);
        Ok(())
    }

    /// Invalidate a path (mark as not valid).
    /// 使路径无效（标记为无效）。
    pub fn invalidate(&mut self, store_path: &StorePath) -> Result<(), StoreError> {
        if let Some(mut info) = self.query(store_path)? {
            info.valid = false;
            self.register(info)?;
        }
        Ok(())
    }

    /// List all registered paths.
    /// 列出所有已注册的路径。
    pub fn list_all(&self) -> Result<Vec<StorePath>, StoreError> {
        let mut paths = Vec::new();

        if !self.root.exists() {
            return Ok(paths);
        }

        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|e| e == "json") {
                let json = fs::read_to_string(entry.path())?;
                if let Ok(info) = serde_json::from_str::<PathInfo>(&json) {
                    paths.push(info.path);
                }
            }
        }

        Ok(paths)
    }
}

/// Get current Unix timestamp.
/// 获取当前 Unix 时间戳。
fn current_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
