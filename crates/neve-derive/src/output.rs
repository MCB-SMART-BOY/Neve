//! Output paths for derivations.
//!
//! Output paths are computed from the derivation hash and live in the store.

use crate::Hash;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The default store path prefix.
pub const STORE_PREFIX: &str = "/neve/store";

/// A store path pointing to a derivation output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StorePath {
    /// The hash component of the path.
    hash: Hash,
    /// The name component (e.g., "hello-2.12.1").
    name: String,
}

impl StorePath {
    /// Create a new store path.
    pub fn new(hash: Hash, name: String) -> Self {
        Self { hash, name }
    }

    /// Create a store path from a derivation hash and name.
    pub fn from_derivation(drv_hash: Hash, name: &str) -> Self {
        // The output hash is derived from the derivation hash
        // In a real implementation, this would also include the output name
        let mut hasher = crate::Hasher::new();
        hasher.update(drv_hash.as_bytes());
        hasher.update_str("out");
        hasher.update_str(name);
        
        Self {
            hash: hasher.finalize(),
            name: name.to_string(),
        }
    }

    /// Get the hash component.
    pub fn hash(&self) -> &Hash {
        &self.hash
    }

    /// Get the name component.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the full path in the store.
    pub fn path(&self) -> PathBuf {
        self.path_with_prefix(STORE_PREFIX)
    }

    /// Get the full path with a custom store prefix.
    pub fn path_with_prefix(&self, prefix: &str) -> PathBuf {
        PathBuf::from(prefix).join(format!("{}-{}", self.hash.to_short_hex(), self.name))
    }

    /// Get the short display name (hash-name).
    pub fn display_name(&self) -> String {
        format!("{}-{}", self.hash.to_short_hex(), self.name)
    }

    /// Parse a store path from a path string.
    pub fn parse(path: &Path) -> Option<Self> {
        let file_name = path.file_name()?.to_str()?;
        Self::parse_name(file_name)
    }

    /// Parse from a "hash-name" string.
    pub fn parse_name(name: &str) -> Option<Self> {
        let dash_pos = name.find('-')?;
        if dash_pos != 32 {
            // Short hex is 32 characters
            return None;
        }
        let hash_str = &name[..32];
        let name_part = &name[33..];
        
        // Reconstruct full hash from short hex (pad with zeros for now)
        let mut hash_bytes = [0u8; 32];
        let short_bytes = hex_decode(hash_str)?;
        hash_bytes[..16].copy_from_slice(&short_bytes);
        
        Some(Self {
            hash: Hash::from_bytes(hash_bytes),
            name: name_part.to_string(),
        })
    }
}

/// An output specification for a derivation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    /// The output name (e.g., "out", "dev", "doc").
    pub name: String,
    /// The computed store path (set after realization).
    pub path: Option<StorePath>,
    /// Hash mode for fixed-output derivations.
    pub hash_mode: Option<HashMode>,
    /// Expected hash for fixed-output derivations.
    pub expected_hash: Option<Hash>,
}

impl Output {
    /// Create a new output with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: None,
            hash_mode: None,
            expected_hash: None,
        }
    }

    /// Create a fixed-output derivation output.
    pub fn fixed(name: impl Into<String>, hash: Hash, mode: HashMode) -> Self {
        Self {
            name: name.into(),
            path: None,
            hash_mode: Some(mode),
            expected_hash: Some(hash),
        }
    }

    /// Check if this is a fixed-output derivation.
    pub fn is_fixed(&self) -> bool {
        self.expected_hash.is_some()
    }
}

/// Hash mode for fixed-output derivations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashMode {
    /// Hash the file contents directly.
    Flat,
    /// Hash the NAR serialization of the path.
    Recursive,
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_path() {
        let hash = Hash::of(b"test derivation");
        let path = StorePath::new(hash, "hello-2.12.1".to_string());
        
        assert_eq!(path.name(), "hello-2.12.1");
        assert!(path.path().to_string_lossy().contains("hello-2.12.1"));
    }

    #[test]
    fn test_store_path_from_derivation() {
        let drv_hash = Hash::of(b"derivation content");
        let path = StorePath::from_derivation(drv_hash, "mypackage-1.0");
        
        assert_eq!(path.name(), "mypackage-1.0");
    }

    #[test]
    fn test_output() {
        let out = Output::new("out");
        assert!(!out.is_fixed());
        
        let fixed = Output::fixed("out", Hash::of(b"expected"), HashMode::Flat);
        assert!(fixed.is_fixed());
    }
}
