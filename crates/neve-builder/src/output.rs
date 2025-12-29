//! Output collection and registration.
//! 输出收集和注册。
//!
//! Handles collecting build outputs and registering them in the store.
//! 处理收集构建输出并将其注册到存储中。

use crate::BuildError;
use neve_derive::{Hash, StorePath};
use neve_store::Store;
use std::fs;
use std::path::Path;

/// Collect a build output and register it in the store.
/// 收集构建输出并将其注册到存储中。
pub fn collect_output(
    store: &Store,
    output_dir: &Path,
    name: &str,
) -> Result<StorePath, BuildError> {
    if !output_dir.exists() {
        return Err(BuildError::BuildFailed(format!(
            "output directory does not exist: {}",
            output_dir.display()
        )));
    }

    // Hash the output
    // 哈希输出
    let hash = hash_output(output_dir)?;

    // Add to store
    // 添加到存储
    let store_path = store.add_dir(output_dir, name)?;

    // Verify the hash matches
    // 验证哈希是否匹配
    let stored_hash = StorePath::new(hash, name.to_string());
    if store_path.hash() != stored_hash.hash() {
        // This shouldn't happen, but check anyway
        // 这不应该发生，但还是检查一下
        return Err(BuildError::OutputHashMismatch {
            output: name.to_string(),
            expected: stored_hash.hash().to_hex(),
            actual: store_path.hash().to_hex(),
        });
    }

    Ok(store_path)
}

/// Hash an output directory.
/// 哈希输出目录。
fn hash_output(path: &Path) -> Result<Hash, BuildError> {
    use neve_derive::Hasher;

    let mut hasher = Hasher::new();
    hash_recursive(path, &mut hasher)?;
    Ok(hasher.finalize())
}

/// Recursively hash a path.
/// 递归哈希路径。
fn hash_recursive(path: &Path, hasher: &mut neve_derive::Hasher) -> Result<(), BuildError> {
    if path.is_file() {
        let content = fs::read(path)?;
        hasher.update(&content);
    } else if path.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(path)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let name = entry.file_name();
            hasher.update(name.as_encoded_bytes());

            let entry_path = entry.path();
            if entry_path.is_dir() {
                // Directory marker
                // 目录标记
                hasher.update(b"d");
            } else if entry_path.is_symlink() {
                // Symlink marker
                // 符号链接标记
                hasher.update(b"l");
            } else {
                // File marker
                // 文件标记
                hasher.update(b"f");
            }

            hash_recursive(&entry_path, hasher)?;
        }
    } else if path.is_symlink() {
        let target = fs::read_link(path)?;
        hasher.update(target.as_os_str().as_encoded_bytes());
    }

    Ok(())
}

/// Validate an output path.
/// 验证输出路径。
pub fn validate_output(path: &Path) -> Result<(), BuildError> {
    if !path.exists() {
        return Err(BuildError::BuildFailed(format!(
            "output does not exist: {}",
            path.display()
        )));
    }

    // Check for common issues
    // 检查常见问题
    if path.is_dir() {
        validate_dir_recursive(path)?;
    }

    Ok(())
}

/// Recursively validate a directory.
/// 递归验证目录。
fn validate_dir_recursive(dir: &Path) -> Result<(), BuildError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Check for broken symlinks
        // 检查损坏的符号链接
        if path.is_symlink() {
            let target = fs::read_link(&path)?;
            if target.is_absolute() && !target.starts_with("/neve/store") {
                return Err(BuildError::BuildFailed(format!(
                    "output contains absolute symlink outside store: {} -> {}",
                    path.display(),
                    target.display()
                )));
            }
        }

        if path.is_dir() {
            validate_dir_recursive(&path)?;
        }
    }

    Ok(())
}

/// Calculate the size of an output.
/// 计算输出的大小。
pub fn output_size(path: &Path) -> Result<u64, BuildError> {
    let mut size = 0u64;

    if path.is_file() {
        size = fs::metadata(path)?.len();
    } else if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            size += output_size(&entry.path())?;
        }
    }

    Ok(size)
}

/// Format a size as a human-readable string.
/// 将大小格式化为人类可读的字符串。
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
