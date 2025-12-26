//! Content hashing for derivations.
//!
//! Neve uses BLAKE3 for all content hashing due to its speed and security.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A content hash using BLAKE3.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Hash {
    bytes: [u8; 32],
}

impl Hash {
    /// Create a hash from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// Hash arbitrary data.
    pub fn of(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self {
            bytes: *hash.as_bytes(),
        }
    }

    /// Hash a string.
    pub fn of_str(s: &str) -> Self {
        Self::of(s.as_bytes())
    }

    /// Get the raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    /// Convert to a hex string (first 32 characters for display).
    pub fn to_short_hex(&self) -> String {
        hex::encode(&self.bytes[..16])
    }

    /// Convert to a full hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    /// Parse from a hex string.
    pub fn from_hex(s: &str) -> Result<Self, HashError> {
        let bytes = hex::decode(s).map_err(|_| HashError::InvalidHex)?;
        if bytes.len() != 32 {
            return Err(HashError::InvalidLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self { bytes: arr })
    }

    /// The null hash (all zeros).
    pub fn null() -> Self {
        Self { bytes: [0u8; 32] }
    }

    /// Check if this is the null hash.
    pub fn is_null(&self) -> bool {
        self.bytes == [0u8; 32]
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", self.to_short_hex())
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_short_hex())
    }
}

/// Errors related to hash operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum HashError {
    #[error("invalid hex string")]
    InvalidHex,
    #[error("invalid hash length")]
    InvalidLength,
}

/// A hasher for incrementally building hashes.
pub struct Hasher {
    inner: blake3::Hasher,
}

impl Hasher {
    /// Create a new hasher.
    pub fn new() -> Self {
        Self {
            inner: blake3::Hasher::new(),
        }
    }

    /// Update the hasher with data.
    pub fn update(&mut self, data: &[u8]) -> &mut Self {
        self.inner.update(data);
        self
    }

    /// Update the hasher with a string.
    pub fn update_str(&mut self, s: &str) -> &mut Self {
        self.update(s.as_bytes())
    }

    /// Finalize and return the hash.
    pub fn finalize(&self) -> Hash {
        let hash = self.inner.finalize();
        Hash {
            bytes: *hash.as_bytes(),
        }
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

// Simple hex encoding/decoding (to avoid external dependency)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if !s.len().is_multiple_of(2) {
            return Err(());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_data() {
        let hash = Hash::of(b"hello world");
        assert!(!hash.is_null());
        assert_eq!(hash.to_hex().len(), 64);
    }

    #[test]
    fn test_hash_roundtrip() {
        let hash = Hash::of(b"test data");
        let hex = hash.to_hex();
        let parsed = Hash::from_hex(&hex).unwrap();
        assert_eq!(hash, parsed);
    }

    #[test]
    fn test_hasher_incremental() {
        let mut hasher = Hasher::new();
        hasher.update(b"hello ");
        hasher.update(b"world");
        let hash1 = hasher.finalize();

        let hash2 = Hash::of(b"hello world");
        assert_eq!(hash1, hash2);
    }
}
