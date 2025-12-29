//! Content verification utilities.
//! 内容验证工具。
//!
//! Provides functionality for verifying content against expected hashes.
//! 提供根据预期哈希验证内容的功能。

use crate::FetchError;
use neve_derive::Hash;
use std::fs;
use std::path::Path;

/// Verify a file against an expected hash.
/// 根据预期哈希验证文件。
pub fn verify_file(path: &Path, expected: &Hash) -> Result<(), FetchError> {
    let content = fs::read(path)?;
    verify_content(&content, expected)
}

/// Verify content against an expected hash.
/// 根据预期哈希验证内容。
pub fn verify_content(content: &[u8], expected: &Hash) -> Result<(), FetchError> {
    let actual = Hash::of(content);

    if actual != *expected {
        return Err(FetchError::HashMismatch {
            expected: expected.to_hex(),
            actual: actual.to_hex(),
        });
    }

    Ok(())
}

/// Verify a directory by hashing all its contents.
/// 通过哈希所有内容来验证目录。
pub fn verify_dir(path: &Path, expected: &Hash) -> Result<(), FetchError> {
    let actual = hash_dir(path)?;

    if actual != *expected {
        return Err(FetchError::HashMismatch {
            expected: expected.to_hex(),
            actual: actual.to_hex(),
        });
    }

    Ok(())
}

/// Hash a directory's contents deterministically.
/// 确定性地哈希目录内容。
pub fn hash_dir(path: &Path) -> Result<Hash, FetchError> {
    use neve_derive::Hasher;

    let mut hasher = Hasher::new();
    hash_dir_recursive(path, &mut hasher)?;
    Ok(hasher.finalize())
}

/// Recursively hash directory contents.
/// 递归哈希目录内容。
fn hash_dir_recursive(path: &Path, hasher: &mut neve_derive::Hasher) -> Result<(), FetchError> {
    let mut entries: Vec<_> = fs::read_dir(path)?.filter_map(|e| e.ok()).collect();

    // Sort for deterministic hashing
    // 排序以实现确定性哈希
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let entry_path = entry.path();
        let name = entry.file_name();

        // Hash the entry name
        // 哈希条目名称
        hasher.update(name.as_encoded_bytes());

        if entry_path.is_dir() {
            // Directory marker
            // 目录标记
            hasher.update(b"d");
            hash_dir_recursive(&entry_path, hasher)?;
        } else if entry_path.is_file() {
            // File marker and content
            // 文件标记和内容
            hasher.update(b"f");
            let content = fs::read(&entry_path)?;
            hasher.update(&content);
        } else if entry_path.is_symlink() {
            // Symlink marker and target
            // 符号链接标记和目标
            hasher.update(b"l");
            let target = fs::read_link(&entry_path)?;
            hasher.update(target.as_os_str().as_encoded_bytes());
        }
    }

    Ok(())
}

/// Hash prefetching result - compute hash without storing.
/// 预取哈希结果 - 计算哈希而不存储。
pub fn prefetch_hash(content: &[u8]) -> Hash {
    Hash::of(content)
}
