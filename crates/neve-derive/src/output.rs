//! Output paths for derivations.
//! 推导的输出路径。
//!
//! Output paths are computed from the derivation hash and live in the store.
//! 输出路径从推导哈希计算得出，存储在 store 中。

use crate::Hash;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The default store path prefix.
/// 默认存储路径前缀。
pub const STORE_PREFIX: &str = "/neve/store";

/// A store path pointing to a derivation output.
/// 指向推导输出的存储路径。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StorePath {
    /// The hash component of the path. / 路径的哈希组件。
    hash: Hash,
    /// The name component (e.g., "hello-2.12.1"). / 名称组件（例如 "hello-2.12.1"）。
    name: String,
}

impl StorePath {
    /// Create a new store path.
    /// 创建新的存储路径。
    pub fn new(hash: Hash, name: String) -> Self {
        Self { hash, name }
    }

    /// Create a store path from a derivation hash and name.
    /// 从推导哈希和名称创建存储路径。
    pub fn from_derivation(drv_hash: Hash, name: &str) -> Self {
        // The output hash is derived from the derivation hash
        // In a real implementation, this would also include the output name
        // 输出哈希从推导哈希派生
        // 在实际实现中，这还会包括输出名称
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
    /// 获取哈希组件。
    pub fn hash(&self) -> &Hash {
        &self.hash
    }

    /// Get the name component.
    /// 获取名称组件。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the full path in the store.
    /// 获取存储中的完整路径。
    pub fn path(&self) -> PathBuf {
        self.path_with_prefix(STORE_PREFIX)
    }

    /// Get the full path with a custom store prefix.
    /// 使用自定义存储前缀获取完整路径。
    pub fn path_with_prefix(&self, prefix: &str) -> PathBuf {
        PathBuf::from(prefix).join(format!("{}-{}", self.hash.to_short_hex(), self.name))
    }

    /// Get the short display name (hash-name).
    /// 获取短显示名称（哈希-名称）。
    pub fn display_name(&self) -> String {
        format!("{}-{}", self.hash.to_short_hex(), self.name)
    }

    /// Parse a store path from a path string.
    /// 从路径字符串解析存储路径。
    pub fn parse(path: &Path) -> Option<Self> {
        let file_name = path.file_name()?.to_str()?;
        Self::parse_name(file_name)
    }

    /// Parse from a "hash-name" string.
    /// 从 "哈希-名称" 字符串解析。
    pub fn parse_name(name: &str) -> Option<Self> {
        let dash_pos = name.find('-')?;
        if dash_pos != 32 {
            // Short hex is 32 characters
            // 短十六进制为 32 个字符
            return None;
        }
        let hash_str = &name[..32];
        let name_part = &name[33..];

        // Reconstruct full hash from short hex (pad with zeros for now)
        // 从短十六进制重建完整哈希（目前用零填充）
        let mut hash_bytes = [0u8; 32];
        let short_bytes = hex_decode(hash_str)?;
        hash_bytes[..16].copy_from_slice(&short_bytes);

        Some(Self {
            hash: Hash::from_bytes(hash_bytes),
            name: name_part.to_string(),
        })
    }
}

impl std::fmt::Display for StorePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path().display())
    }
}

/// An output specification for a derivation.
/// 推导的输出规格。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    /// The output name (e.g., "out", "dev", "doc"). / 输出名称（例如 "out"、"dev"、"doc"）。
    pub name: String,
    /// The computed store path (set after realization). / 计算的存储路径（实现后设置）。
    pub path: Option<StorePath>,
    /// Hash mode for fixed-output derivations. / 固定输出推导的哈希模式。
    pub hash_mode: Option<HashMode>,
    /// Expected hash for fixed-output derivations. / 固定输出推导的预期哈希。
    pub expected_hash: Option<Hash>,
}

impl Output {
    /// Create a new output with the given name.
    /// 使用给定名称创建新输出。
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: None,
            hash_mode: None,
            expected_hash: None,
        }
    }

    /// Create a fixed-output derivation output.
    /// 创建固定输出推导的输出。
    pub fn fixed(name: impl Into<String>, hash: Hash, mode: HashMode) -> Self {
        Self {
            name: name.into(),
            path: None,
            hash_mode: Some(mode),
            expected_hash: Some(hash),
        }
    }

    /// Check if this is a fixed-output derivation.
    /// 检查是否为固定输出推导。
    pub fn is_fixed(&self) -> bool {
        self.expected_hash.is_some()
    }
}

/// Hash mode for fixed-output derivations.
/// 固定输出推导的哈希模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashMode {
    /// Hash the file contents directly. / 直接哈希文件内容。
    Flat,
    /// Hash the NAR serialization of the path. / 哈希路径的 NAR 序列化。
    Recursive,
}

/// Decode hex string to bytes.
/// 将十六进制字符串解码为字节。
fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}
