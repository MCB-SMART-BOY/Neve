//! Store operations.
//! 存储操作。

use crate::path::store_dir;
use neve_derive::{Derivation, Hash, StorePath};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during store operations.
/// 存储操作期间可能发生的错误。
#[derive(Debug, Error)]
pub enum StoreError {
    /// I/O error. / I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Path not found. / 未找到路径。
    #[error("path not found: {0}")]
    PathNotFound(String),

    /// Path already exists. / 路径已存在。
    #[error("path already exists: {0}")]
    PathExists(String),

    /// Invalid store path. / 无效的存储路径。
    #[error("invalid store path: {0}")]
    InvalidPath(String),

    /// Hash mismatch. / 哈希不匹配。
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: Hash, actual: Hash },

    /// Serialization error. / 序列化错误。
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// The Neve store.
/// Neve 存储。
pub struct Store {
    /// The root directory of the store. / 存储的根目录。
    root: PathBuf,
    /// Cache of loaded derivations. / 已加载推导的缓存。
    derivation_cache: HashMap<StorePath, Derivation>,
}

impl Store {
    /// Open the store at the default location.
    /// 在默认位置打开存储。
    pub fn open() -> Result<Self, StoreError> {
        Self::open_at(store_dir())
    }

    /// Open the store at a specific location.
    /// 在特定位置打开存储。
    pub fn open_at(root: PathBuf) -> Result<Self, StoreError> {
        // Ensure the store directory exists
        // 确保存储目录存在
        fs::create_dir_all(&root)?;

        Ok(Self {
            root,
            derivation_cache: HashMap::new(),
        })
    }

    /// Get the store root directory.
    /// 获取存储根目录。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Check if a path exists in the store.
    /// 检查路径是否存在于存储中。
    pub fn path_exists(&self, path: &StorePath) -> bool {
        self.to_path(path).exists()
    }

    /// Convert a StorePath to an absolute filesystem path.
    /// 将 StorePath 转换为绝对文件系统路径。
    pub fn to_path(&self, store_path: &StorePath) -> PathBuf {
        store_path.path_with_prefix(&self.root.to_string_lossy())
    }

    /// Add a file to the store with a specific hash.
    /// 将文件添加到存储并使用特定哈希。
    pub fn add_file(&self, source: &Path, name: &str) -> Result<StorePath, StoreError> {
        // Read and hash the file
        // 读取并哈希文件
        let content = fs::read(source)?;
        let hash = Hash::of(&content);

        let store_path = StorePath::new(hash, name.to_string());
        let dest = self.to_path(&store_path);

        if dest.exists() {
            // Already in store, verify hash
            // 已在存储中，验证哈希
            let existing_content = fs::read(&dest)?;
            let existing_hash = Hash::of(&existing_content);
            if existing_hash != hash {
                return Err(StoreError::HashMismatch {
                    expected: hash,
                    actual: existing_hash,
                });
            }
        } else {
            // Copy to store
            // 复制到存储
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source, &dest)?;
            // Make read-only
            // 设为只读
            let mut perms = fs::metadata(&dest)?.permissions();
            perms.set_readonly(true);
            fs::set_permissions(&dest, perms)?;
        }

        Ok(store_path)
    }

    /// Add a directory to the store.
    /// 将目录添加到存储。
    pub fn add_dir(&self, source: &Path, name: &str) -> Result<StorePath, StoreError> {
        // Hash the directory contents (simplified: just hash file names and contents)
        // 哈希目录内容（简化：只哈希文件名和内容）
        let hash = hash_dir(source)?;

        let store_path = StorePath::new(hash, name.to_string());
        let dest = self.to_path(&store_path);

        if !dest.exists() {
            copy_dir_recursive(source, &dest)?;
            make_readonly_recursive(&dest)?;
        }

        Ok(store_path)
    }

    /// Add content directly to the store.
    /// 将内容直接添加到存储。
    pub fn add_content(&self, content: &[u8], name: &str) -> Result<StorePath, StoreError> {
        let hash = Hash::of(content);
        let store_path = StorePath::new(hash, name.to_string());
        let dest = self.to_path(&store_path);

        if !dest.exists() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&dest, content)?;
            let mut perms = fs::metadata(&dest)?.permissions();
            perms.set_readonly(true);
            fs::set_permissions(&dest, perms)?;
        }

        Ok(store_path)
    }

    /// Add a derivation to the store.
    /// 将推导添加到存储。
    pub fn add_derivation(&mut self, drv: &Derivation) -> Result<StorePath, StoreError> {
        let drv_path = drv.drv_path();
        let dest = self.to_path(&drv_path);

        if !dest.exists() {
            let json = drv.to_json()?;
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&dest, &json)?;
        }

        self.derivation_cache.insert(drv_path.clone(), drv.clone());
        Ok(drv_path)
    }

    /// Read a derivation from the store.
    /// 从存储读取推导。
    pub fn read_derivation(&mut self, path: &StorePath) -> Result<Derivation, StoreError> {
        if let Some(drv) = self.derivation_cache.get(path) {
            return Ok(drv.clone());
        }

        let fs_path = self.to_path(path);
        if !fs_path.exists() {
            return Err(StoreError::PathNotFound(path.display_name()));
        }

        let content = fs::read_to_string(&fs_path)?;
        let drv = Derivation::from_json(&content)?;
        self.derivation_cache.insert(path.clone(), drv.clone());

        Ok(drv)
    }

    /// Delete a path from the store (for garbage collection).
    /// 从存储删除路径（用于垃圾回收）。
    pub fn delete(&self, path: &StorePath) -> Result<(), StoreError> {
        let fs_path = self.to_path(path);
        if !fs_path.exists() {
            return Ok(());
        }

        // Make writable first
        // 首先设为可写
        make_writable_recursive(&fs_path)?;

        if fs_path.is_dir() {
            fs::remove_dir_all(&fs_path)?;
        } else {
            fs::remove_file(&fs_path)?;
        }

        Ok(())
    }

    /// List all paths in the store.
    /// 列出存储中的所有路径。
    pub fn list_paths(&self) -> Result<Vec<StorePath>, StoreError> {
        let mut paths = Vec::new();

        if !self.root.exists() {
            return Ok(paths);
        }

        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(store_path) = StorePath::parse(&path) {
                paths.push(store_path);
            }
        }

        Ok(paths)
    }

    /// Get the total size of the store in bytes.
    /// 获取存储的总大小（字节）。
    pub fn size(&self) -> Result<u64, StoreError> {
        dir_size(&self.root)
    }
}

/// Hash a directory's contents.
/// 哈希目录的内容。
fn hash_dir(path: &Path) -> Result<Hash, StoreError> {
    let mut hasher = neve_derive::Hasher::new();
    hash_dir_recursive(path, &mut hasher)?;
    Ok(hasher.finalize())
}

/// Recursively hash directory contents.
/// 递归哈希目录内容。
fn hash_dir_recursive(path: &Path, hasher: &mut neve_derive::Hasher) -> Result<(), StoreError> {
    let mut entries: Vec<_> = fs::read_dir(path)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        hasher.update(name.as_encoded_bytes());

        if path.is_dir() {
            hasher.update(b"d");
            hash_dir_recursive(&path, hasher)?;
        } else {
            hasher.update(b"f");
            let content = fs::read(&path)?;
            hasher.update(&content);
        }
    }

    Ok(())
}

/// Recursively copy a directory.
/// 递归复制目录。
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), StoreError> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Recursively make a path read-only.
/// 递归地将路径设为只读。
fn make_readonly_recursive(path: &Path) -> Result<(), StoreError> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            make_readonly_recursive(&entry?.path())?;
        }
    }

    let mut perms = fs::metadata(path)?.permissions();
    perms.set_readonly(true);
    fs::set_permissions(path, perms)?;

    Ok(())
}

/// Recursively make a path writable.
/// 递归地将路径设为可写。
#[cfg(unix)]
fn make_writable_recursive(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt;

    let perms = fs::metadata(path)?.permissions();
    // Set user read/write permissions (0o644 for files, 0o755 for dirs)
    // 设置用户读/写权限（文件为 0o644，目录为 0o755）
    let mode = if path.is_dir() { 0o755 } else { 0o644 };
    let new_perms = fs::Permissions::from_mode(perms.mode() | mode);
    fs::set_permissions(path, new_perms)?;

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            make_writable_recursive(&entry?.path())?;
        }
    }

    Ok(())
}

#[cfg(not(unix))]
fn make_writable_recursive(path: &Path) -> Result<(), StoreError> {
    let mut perms = fs::metadata(path)?.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    fs::set_permissions(path, perms)?;

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            make_writable_recursive(&entry?.path())?;
        }
    }

    Ok(())
}

/// Calculate the size of a directory.
/// 计算目录的大小。
fn dir_size(path: &Path) -> Result<u64, StoreError> {
    let mut size = 0;

    if !path.exists() {
        return Ok(0);
    }

    if path.is_file() {
        return Ok(fs::metadata(path)?.len());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            size += dir_size(&path)?;
        } else {
            size += fs::metadata(&path)?.len();
        }
    }

    Ok(size)
}
