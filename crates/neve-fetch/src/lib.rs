//! Source fetching for Neve.
//! Neve 的源码获取。
//!
//! This crate provides functionality for fetching sources from various locations:
//! 本 crate 提供从各种位置获取源码的功能：
//!
//! - URLs (http/https) / URL（http/https）
//! - Local files / 本地文件
//! - Git repositories / Git 仓库
//!
//! All fetched content is verified against expected hashes.
//! 所有获取的内容都会根据预期哈希进行验证。

pub mod archive;
pub mod git;
pub mod url;
pub mod verify;

use neve_derive::Hash;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during fetching.
/// 获取过程中可能发生的错误。
#[derive(Debug, Error)]
pub enum FetchError {
    /// I/O error. / I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP error. / HTTP 错误。
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Hash mismatch between expected and actual. / 预期与实际哈希不匹配。
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    /// Unsupported URL scheme. / 不支持的 URL 方案。
    #[error("unsupported URL scheme: {0}")]
    UnsupportedScheme(String),

    /// Archive extraction error. / 归档解压错误。
    #[error("archive error: {0}")]
    Archive(String),

    /// Content verification failed. / 内容验证失败。
    #[error("verification failed: {0}")]
    Verification(String),

    /// Git operation error. / Git 操作错误。
    #[error("Git error: {0}")]
    Git(String),
}

/// A source to fetch.
/// 要获取的源。
#[derive(Debug, Clone)]
pub enum Source {
    /// Fetch from a URL. / 从 URL 获取。
    Url {
        /// The URL to fetch from. / 要获取的 URL。
        url: String,
        /// Expected hash for verification. / 用于验证的预期哈希。
        hash: Option<Hash>,
        /// Optional name for the downloaded file. / 下载文件的可选名称。
        name: Option<String>,
    },
    /// Use a local file. / 使用本地文件。
    Path {
        /// Path to the local file. / 本地文件的路径。
        path: PathBuf,
        /// Expected hash for verification. / 用于验证的预期哈希。
        hash: Option<Hash>,
    },
    /// Fetch from a Git repository. / 从 Git 仓库获取。
    Git {
        /// Repository URL. / 仓库 URL。
        url: String,
        /// Revision to checkout (branch, tag, or commit). / 要检出的修订版本（分支、标签或提交）。
        rev: String,
        /// Expected hash for verification. / 用于验证的预期哈希。
        hash: Option<Hash>,
    },
}

impl Source {
    /// Create a URL source.
    /// 创建 URL 源。
    pub fn url(url: impl Into<String>) -> Self {
        Source::Url {
            url: url.into(),
            hash: None,
            name: None,
        }
    }

    /// Create a URL source with expected hash.
    /// 创建带有预期哈希的 URL 源。
    pub fn url_with_hash(url: impl Into<String>, hash: Hash) -> Self {
        Source::Url {
            url: url.into(),
            hash: Some(hash),
            name: None,
        }
    }

    /// Create a path source.
    /// 创建路径源。
    pub fn path(path: impl Into<PathBuf>) -> Self {
        Source::Path {
            path: path.into(),
            hash: None,
        }
    }

    /// Create a Git source.
    /// 创建 Git 源。
    pub fn git(url: impl Into<String>, rev: impl Into<String>) -> Self {
        Source::Git {
            url: url.into(),
            rev: rev.into(),
            hash: None,
        }
    }

    /// Set the expected hash.
    /// 设置预期哈希。
    pub fn with_hash(self, hash: Hash) -> Self {
        match self {
            Source::Url { url, name, .. } => Source::Url {
                url,
                hash: Some(hash),
                name,
            },
            Source::Path { path, .. } => Source::Path {
                path,
                hash: Some(hash),
            },
            Source::Git { url, rev, .. } => Source::Git {
                url,
                rev,
                hash: Some(hash),
            },
        }
    }

    /// Set the name (for URL sources).
    /// 设置名称（用于 URL 源）。
    pub fn with_name(self, name: impl Into<String>) -> Self {
        match self {
            Source::Url { url, hash, .. } => Source::Url {
                url,
                hash,
                name: Some(name.into()),
            },
            other => other,
        }
    }
}

/// Result of a fetch operation.
/// 获取操作的结果。
#[derive(Debug)]
pub struct FetchResult {
    /// Path to the fetched content. / 获取内容的路径。
    pub path: PathBuf,
    /// Hash of the fetched content. / 获取内容的哈希。
    pub hash: Hash,
    /// Whether this was a cache hit. / 是否命中缓存。
    pub cached: bool,
}

/// Fetcher for downloading and caching sources.
/// 用于下载和缓存源的获取器。
pub struct Fetcher {
    /// Cache directory. / 缓存目录。
    cache_dir: PathBuf,
}

impl Fetcher {
    /// Create a new fetcher with the given cache directory.
    /// 使用给定的缓存目录创建新的获取器。
    pub fn new(cache_dir: PathBuf) -> Result<Self, FetchError> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    /// Fetch a source.
    /// 获取源。
    pub fn fetch(&self, source: &Source) -> Result<FetchResult, FetchError> {
        match source {
            Source::Url { url, hash, name } => self.fetch_url(url, hash.as_ref(), name.as_deref()),
            Source::Path { path, hash } => self.fetch_path(path, hash.as_ref()),
            Source::Git { url, rev, hash } => self.fetch_git(url, rev, hash.as_ref()),
        }
    }

    /// Fetch from a URL.
    /// 从 URL 获取。
    fn fetch_url(
        &self,
        url: &str,
        expected_hash: Option<&Hash>,
        name: Option<&str>,
    ) -> Result<FetchResult, FetchError> {
        // Derive name from URL if not provided
        // 如果未提供，从 URL 推导名称
        let file_name = name
            .map(String::from)
            .unwrap_or_else(|| url.rsplit('/').next().unwrap_or("download").to_string());

        // Check cache first if we have an expected hash
        // 如果有预期哈希，先检查缓存
        if let Some(hash) = expected_hash {
            let cached_path = self.cache_path(hash, &file_name);
            if cached_path.exists() {
                return Ok(FetchResult {
                    path: cached_path,
                    hash: *hash,
                    cached: true,
                });
            }
        }

        // Download to temp file
        // 下载到临时文件
        let content = url::fetch_url(url)?;
        let actual_hash = Hash::of(&content);

        // Verify hash if expected
        // 如果有预期哈希则验证
        if let Some(expected) = expected_hash
            && actual_hash != *expected
        {
            return Err(FetchError::HashMismatch {
                expected: expected.to_hex(),
                actual: actual_hash.to_hex(),
            });
        }

        // Store in cache
        // 存储到缓存
        let cache_path = self.cache_path(&actual_hash, &file_name);
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&cache_path, &content)?;

        Ok(FetchResult {
            path: cache_path,
            hash: actual_hash,
            cached: false,
        })
    }

    /// Fetch from a local path.
    /// 从本地路径获取。
    fn fetch_path(
        &self,
        path: &PathBuf,
        expected_hash: Option<&Hash>,
    ) -> Result<FetchResult, FetchError> {
        let content = std::fs::read(path)?;
        let actual_hash = Hash::of(&content);

        if let Some(expected) = expected_hash
            && actual_hash != *expected
        {
            return Err(FetchError::HashMismatch {
                expected: expected.to_hex(),
                actual: actual_hash.to_hex(),
            });
        }

        Ok(FetchResult {
            path: path.clone(),
            hash: actual_hash,
            cached: true,
        })
    }

    /// Fetch from a Git repository.
    /// 从 Git 仓库获取。
    fn fetch_git(
        &self,
        url: &str,
        rev: &str,
        expected_hash: Option<&Hash>,
    ) -> Result<FetchResult, FetchError> {
        // Derive a name from the URL
        // 从 URL 推导名称
        let repo_name = url
            .rsplit('/')
            .next()
            .unwrap_or("repo")
            .trim_end_matches(".git");

        // Check cache first if we have an expected hash
        // 如果有预期哈希，先检查缓存
        if let Some(hash) = expected_hash {
            let cached_path = self.git_cache_path(hash, repo_name);
            if cached_path.exists() {
                return Ok(FetchResult {
                    path: cached_path,
                    hash: *hash,
                    cached: true,
                });
            }
        }

        // Clone to a temporary location first
        // 先克隆到临时位置
        let temp_dir = tempfile::tempdir()?;
        let clone_path = temp_dir.path().join("repo");

        // Clone the repository
        // 克隆仓库
        let repo = git::clone_repo(url, &clone_path)?;

        // Checkout the specified revision
        // 检出指定的修订版本
        let _oid = git::checkout_rev(&repo, rev)?;

        // Remove .git directory to make it a pure source tree
        // 删除 .git 目录使其成为纯源码树
        let git_dir = clone_path.join(".git");
        if git_dir.exists() {
            std::fs::remove_dir_all(&git_dir)?;
        }

        // Hash the directory contents
        // 哈希目录内容
        let actual_hash = git::hash_directory(&clone_path)?;

        // Verify hash if expected
        // 如果有预期哈希则验证
        if let Some(expected) = expected_hash
            && actual_hash != *expected
        {
            return Err(FetchError::HashMismatch {
                expected: expected.to_hex(),
                actual: actual_hash.to_hex(),
            });
        }

        // Move to cache
        // 移动到缓存
        let cache_path = self.git_cache_path(&actual_hash, repo_name);
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Copy the repo to cache (can't move across filesystems)
        // 复制仓库到缓存（不能跨文件系统移动）
        copy_dir_all(&clone_path, &cache_path)?;

        Ok(FetchResult {
            path: cache_path,
            hash: actual_hash,
            cached: false,
        })
    }

    /// Get the cache path for a Git repository.
    /// 获取 Git 仓库的缓存路径。
    fn git_cache_path(&self, hash: &Hash, name: &str) -> PathBuf {
        let hash_prefix = &hash.to_hex()[..2];
        self.cache_dir
            .join("git")
            .join(hash_prefix)
            .join(format!("{}-{}", hash.to_hex(), name))
    }

    /// Get the cache path for a hash.
    /// 获取哈希的缓存路径。
    fn cache_path(&self, hash: &Hash, name: &str) -> PathBuf {
        let hash_prefix = &hash.to_hex()[..2];
        self.cache_dir
            .join(hash_prefix)
            .join(format!("{}-{}", hash.to_hex(), name))
    }

    /// Fetch text content from a URL.
    /// 从 URL 获取文本内容。
    pub fn fetch_text(&self, url: &str) -> Result<String, FetchError> {
        let content = url::fetch_url(url)?;
        String::from_utf8(content)
            .map_err(|e| FetchError::Verification(format!("Invalid UTF-8: {}", e)))
    }

    /// Fetch a file from a URL and save to destination.
    /// 从 URL 获取文件并保存到目标位置。
    pub fn fetch_file(&self, url: &str, dest: &std::path::Path) -> Result<(), FetchError> {
        let content = url::fetch_url(url)?;

        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(dest, &content)?;
        Ok(())
    }
}

/// Recursively copy a directory.
/// 递归复制目录。
fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<(), FetchError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
