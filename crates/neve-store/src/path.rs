//! Store path utilities.
//! 存储路径工具。

use neve_derive::StorePath;
use std::path::{Path, PathBuf};

/// The default store directory.
/// 默认存储目录。
pub const DEFAULT_STORE_DIR: &str = "/neve/store";

/// Get the store directory from environment or use default.
/// 从环境变量获取存储目录或使用默认值。
pub fn store_dir() -> PathBuf {
    std::env::var("NEVE_STORE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_STORE_DIR))
}

/// Check if a path is inside the store.
/// 检查路径是否在存储内。
pub fn is_in_store(path: &Path) -> bool {
    let store = store_dir();
    path.starts_with(&store)
}

/// Get the relative path within the store.
/// 获取存储内的相对路径。
pub fn relative_store_path(path: &Path) -> Option<&Path> {
    let store = store_dir();
    path.strip_prefix(&store).ok()
}

/// Convert a StorePath to an absolute filesystem path.
/// 将 StorePath 转换为绝对文件系统路径。
pub fn to_absolute(store_path: &StorePath) -> PathBuf {
    store_path.path_with_prefix(&store_dir().to_string_lossy())
}
