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
    let mut entries: Vec<_> = fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .collect();
    
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn test_verify_content() {
        let content = b"hello world";
        let hash = Hash::of(content);
        
        assert!(verify_content(content, &hash).is_ok());
        
        let wrong_hash = Hash::of(b"different content");
        assert!(verify_content(content, &wrong_hash).is_err());
    }

    #[test]
    fn test_verify_file() {
        let dir = env::temp_dir().join(format!("neve-verify-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        
        let file_path = dir.join("test.txt");
        let content = b"test content";
        fs::write(&file_path, content).unwrap();
        
        let hash = Hash::of(content);
        assert!(verify_file(&file_path, &hash).is_ok());
        
        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_hash_dir() {
        let dir = env::temp_dir().join(format!("neve-hash-dir-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        
        fs::write(dir.join("a.txt"), b"aaa").unwrap();
        fs::write(dir.join("b.txt"), b"bbb").unwrap();
        
        let hash1 = hash_dir(&dir).unwrap();
        let hash2 = hash_dir(&dir).unwrap();
        
        // Same content should produce same hash
        assert_eq!(hash1, hash2);
        
        // Different content should produce different hash
        fs::write(dir.join("c.txt"), b"ccc").unwrap();
        let hash3 = hash_dir(&dir).unwrap();
        assert_ne!(hash1, hash3);
        
        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }
}
