//! Store path utilities.

use neve_derive::StorePath;
use std::path::{Path, PathBuf};

/// The default store directory.
pub const DEFAULT_STORE_DIR: &str = "/neve/store";

/// Get the store directory from environment or use default.
pub fn store_dir() -> PathBuf {
    std::env::var("NEVE_STORE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_STORE_DIR))
}

/// Check if a path is inside the store.
pub fn is_in_store(path: &Path) -> bool {
    let store = store_dir();
    path.starts_with(&store)
}

/// Get the relative path within the store.
pub fn relative_store_path(path: &Path) -> Option<&Path> {
    let store = store_dir();
    path.strip_prefix(&store).ok()
}

/// Convert a StorePath to an absolute filesystem path.
pub fn to_absolute(store_path: &StorePath) -> PathBuf {
    store_path.path_with_prefix(&store_dir().to_string_lossy())
}

