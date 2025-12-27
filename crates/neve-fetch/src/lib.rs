//! Source fetching for Neve.
//!
//! This crate provides functionality for fetching sources from various locations:
//! - URLs (http/https)
//! - Local files
//! - Git repositories
//!
//! All fetched content is verified against expected hashes.

pub mod url;
pub mod verify;
pub mod archive;
pub mod git;

use neve_derive::Hash;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during fetching.
#[derive(Debug, Error)]
pub enum FetchError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    
    #[error("unsupported URL scheme: {0}")]
    UnsupportedScheme(String),
    
    #[error("archive error: {0}")]
    Archive(String),
    
    #[error("verification failed: {0}")]
    Verification(String),
    
    #[error("Git error: {0}")]
    Git(String),
}

/// A source to fetch.
#[derive(Debug, Clone)]
pub enum Source {
    /// Fetch from a URL.
    Url {
        url: String,
        hash: Option<Hash>,
        name: Option<String>,
    },
    /// Use a local file.
    Path {
        path: PathBuf,
        hash: Option<Hash>,
    },
    /// Fetch from a Git repository.
    Git {
        url: String,
        rev: String,
        hash: Option<Hash>,
    },
}

impl Source {
    /// Create a URL source.
    pub fn url(url: impl Into<String>) -> Self {
        Source::Url {
            url: url.into(),
            hash: None,
            name: None,
        }
    }

    /// Create a URL source with expected hash.
    pub fn url_with_hash(url: impl Into<String>, hash: Hash) -> Self {
        Source::Url {
            url: url.into(),
            hash: Some(hash),
            name: None,
        }
    }

    /// Create a path source.
    pub fn path(path: impl Into<PathBuf>) -> Self {
        Source::Path {
            path: path.into(),
            hash: None,
        }
    }

    /// Create a Git source.
    pub fn git(url: impl Into<String>, rev: impl Into<String>) -> Self {
        Source::Git {
            url: url.into(),
            rev: rev.into(),
            hash: None,
        }
    }

    /// Set the expected hash.
    pub fn with_hash(self, hash: Hash) -> Self {
        match self {
            Source::Url { url, name, .. } => Source::Url { url, hash: Some(hash), name },
            Source::Path { path, .. } => Source::Path { path, hash: Some(hash) },
            Source::Git { url, rev, .. } => Source::Git { url, rev, hash: Some(hash) },
        }
    }

    /// Set the name (for URL sources).
    pub fn with_name(self, name: impl Into<String>) -> Self {
        match self {
            Source::Url { url, hash, .. } => Source::Url { url, hash, name: Some(name.into()) },
            other => other,
        }
    }
}

/// Result of a fetch operation.
#[derive(Debug)]
pub struct FetchResult {
    /// Path to the fetched content.
    pub path: PathBuf,
    /// Hash of the fetched content.
    pub hash: Hash,
    /// Whether this was a cache hit.
    pub cached: bool,
}

/// Fetcher for downloading and caching sources.
pub struct Fetcher {
    /// Cache directory.
    cache_dir: PathBuf,
}

impl Fetcher {
    /// Create a new fetcher with the given cache directory.
    pub fn new(cache_dir: PathBuf) -> Result<Self, FetchError> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    /// Fetch a source.
    pub fn fetch(&self, source: &Source) -> Result<FetchResult, FetchError> {
        match source {
            Source::Url { url, hash, name } => {
                self.fetch_url(url, hash.as_ref(), name.as_deref())
            }
            Source::Path { path, hash } => {
                self.fetch_path(path, hash.as_ref())
            }
            Source::Git { url, rev, hash } => {
                self.fetch_git(url, rev, hash.as_ref())
            }
        }
    }

    /// Fetch from a URL.
    fn fetch_url(&self, url: &str, expected_hash: Option<&Hash>, name: Option<&str>) -> Result<FetchResult, FetchError> {
        // Derive name from URL if not provided
        let file_name = name.map(String::from).unwrap_or_else(|| {
            url.rsplit('/').next().unwrap_or("download").to_string()
        });

        // Check cache first if we have an expected hash
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
        let content = url::fetch_url(url)?;
        let actual_hash = Hash::of(&content);

        // Verify hash if expected
        if let Some(expected) = expected_hash
            && actual_hash != *expected {
                return Err(FetchError::HashMismatch {
                    expected: expected.to_hex(),
                    actual: actual_hash.to_hex(),
                });
            }

        // Store in cache
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
    fn fetch_path(&self, path: &PathBuf, expected_hash: Option<&Hash>) -> Result<FetchResult, FetchError> {
        let content = std::fs::read(path)?;
        let actual_hash = Hash::of(&content);

        if let Some(expected) = expected_hash
            && actual_hash != *expected {
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
    fn fetch_git(&self, url: &str, rev: &str, expected_hash: Option<&Hash>) -> Result<FetchResult, FetchError> {
        // Derive a name from the URL
        let repo_name = url
            .rsplit('/')
            .next()
            .unwrap_or("repo")
            .trim_end_matches(".git");
        
        // Check cache first if we have an expected hash
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
        let temp_dir = tempfile::tempdir()?;
        let clone_path = temp_dir.path().join("repo");
        
        // Clone the repository
        let repo = git::clone_repo(url, &clone_path)?;
        
        // Checkout the specified revision
        let _oid = git::checkout_rev(&repo, rev)?;
        
        // Remove .git directory to make it a pure source tree
        let git_dir = clone_path.join(".git");
        if git_dir.exists() {
            std::fs::remove_dir_all(&git_dir)?;
        }
        
        // Hash the directory contents
        let actual_hash = git::hash_directory(&clone_path)?;
        
        // Verify hash if expected
        if let Some(expected) = expected_hash
            && actual_hash != *expected {
                return Err(FetchError::HashMismatch {
                    expected: expected.to_hex(),
                    actual: actual_hash.to_hex(),
                });
            }
        
        // Move to cache
        let cache_path = self.git_cache_path(&actual_hash, repo_name);
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Copy the repo to cache (can't move across filesystems)
        copy_dir_all(&clone_path, &cache_path)?;
        
        Ok(FetchResult {
            path: cache_path,
            hash: actual_hash,
            cached: false,
        })
    }
    
    /// Get the cache path for a Git repository.
    fn git_cache_path(&self, hash: &Hash, name: &str) -> PathBuf {
        let hash_prefix = &hash.to_hex()[..2];
        self.cache_dir.join("git").join(hash_prefix).join(format!("{}-{}", hash.to_hex(), name))
    }

    /// Get the cache path for a hash.
    fn cache_path(&self, hash: &Hash, name: &str) -> PathBuf {
        let hash_prefix = &hash.to_hex()[..2];
        self.cache_dir.join(hash_prefix).join(format!("{}-{}", hash.to_hex(), name))
    }
}

/// Recursively copy a directory.
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

