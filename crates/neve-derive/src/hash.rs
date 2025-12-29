//! Content hashing for derivations.
//! 推导的内容哈希。
//!
//! Neve uses BLAKE3 for all content hashing due to its speed and security.
//! Neve 使用 BLAKE3 进行所有内容哈希，因为其速度快且安全。

use serde::{Deserialize, Serialize};
use std::fmt;

/// A content hash using BLAKE3.
/// 使用 BLAKE3 的内容哈希。
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Hash {
    bytes: [u8; 32],
}

impl Hash {
    /// Create a hash from raw bytes.
    /// 从原始字节创建哈希。
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// Hash arbitrary data.
    /// 哈希任意数据。
    pub fn of(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self {
            bytes: *hash.as_bytes(),
        }
    }

    /// Hash a string.
    /// 哈希字符串。
    pub fn of_str(s: &str) -> Self {
        Self::of(s.as_bytes())
    }

    /// Get the raw bytes.
    /// 获取原始字节。
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    /// Convert to a hex string (first 32 characters for display).
    /// 转换为十六进制字符串（显示前 32 个字符）。
    pub fn to_short_hex(&self) -> String {
        hex::encode(&self.bytes[..16])
    }

    /// Convert to a full hex string.
    /// 转换为完整的十六进制字符串。
    pub fn to_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    /// Parse from a hex string.
    /// 从十六进制字符串解析。
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
    /// 空哈希（全零）。
    pub fn null() -> Self {
        Self { bytes: [0u8; 32] }
    }

    /// Check if this is the null hash.
    /// 检查是否为空哈希。
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
/// 与哈希操作相关的错误。
#[derive(Debug, Clone, thiserror::Error)]
pub enum HashError {
    /// Invalid hex string. / 无效的十六进制字符串。
    #[error("invalid hex string")]
    InvalidHex,
    /// Invalid hash length. / 无效的哈希长度。
    #[error("invalid hash length")]
    InvalidLength,
}

/// A hasher for incrementally building hashes.
/// 用于增量构建哈希的哈希器。
pub struct Hasher {
    inner: blake3::Hasher,
}

impl Hasher {
    /// Create a new hasher.
    /// 创建新的哈希器。
    pub fn new() -> Self {
        Self {
            inner: blake3::Hasher::new(),
        }
    }

    /// Update the hasher with data.
    /// 用数据更新哈希器。
    pub fn update(&mut self, data: &[u8]) -> &mut Self {
        self.inner.update(data);
        self
    }

    /// Update the hasher with a string.
    /// 用字符串更新哈希器。
    pub fn update_str(&mut self, s: &str) -> &mut Self {
        self.update(s.as_bytes())
    }

    /// Finalize and return the hash.
    /// 完成并返回哈希值。
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
// 简单的十六进制编码/解码（避免外部依赖）
mod hex {
    /// Encode bytes to hex string.
    /// 将字节编码为十六进制字符串。
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Decode hex string to bytes.
    /// 将十六进制字符串解码为字节。
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
