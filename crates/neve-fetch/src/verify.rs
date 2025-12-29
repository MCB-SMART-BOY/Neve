//! Content verification utilities.

use crate::FetchError;
use neve_derive::Hash;
use std::fs;
use std::path::Path;

/// Verify a file against an expected hash.
pub fn verify_file(path: &Path, expected: &Hash) -> Result<(), FetchError> {
    let content = fs::read(path)?;
    verify_content(&content, expected)
}

/// Verify content against an expected hash.
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
pub fn hash_dir(path: &Path) -> Result<Hash, FetchError> {
    use neve_derive::Hasher;

    let mut hasher = Hasher::new();
    hash_dir_recursive(path, &mut hasher)?;
    Ok(hasher.finalize())
}

/// Recursively hash directory contents.
fn hash_dir_recursive(path: &Path, hasher: &mut neve_derive::Hasher) -> Result<(), FetchError> {
    let mut entries: Vec<_> = fs::read_dir(path)?.filter_map(|e| e.ok()).collect();

    // Sort for deterministic hashing
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let entry_path = entry.path();
        let name = entry.file_name();

        // Hash the entry name
        hasher.update(name.as_encoded_bytes());

        if entry_path.is_dir() {
            hasher.update(b"d");
            hash_dir_recursive(&entry_path, hasher)?;
        } else if entry_path.is_file() {
            hasher.update(b"f");
            let content = fs::read(&entry_path)?;
            hasher.update(&content);
        } else if entry_path.is_symlink() {
            hasher.update(b"l");
            let target = fs::read_link(&entry_path)?;
            hasher.update(target.as_os_str().as_encoded_bytes());
        }
    }

    Ok(())
}

/// Hash prefetching result - compute hash without storing.
pub fn prefetch_hash(content: &[u8]) -> Hash {
    Hash::of(content)
}
