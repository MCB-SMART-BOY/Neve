//! Binary cache for pre-built derivations.
//!
//! The binary cache allows sharing pre-built store paths between machines,
//! avoiding the need to rebuild packages from source.

use crate::{Store, StoreError};
use neve_derive::{Derivation, Hash, StorePath};
use neve_fetch::Fetcher;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Create a placeholder derivation for cached paths.
fn placeholder_derivation(name: &str) -> Derivation {
    Derivation {
        name: name.to_string(),
        version: "0.0.0".to_string(),
        system: "unknown".to_string(),
        builder: "/bin/sh".to_string(),
        args: Vec::new(),
        env: BTreeMap::new(),
        input_drvs: BTreeMap::new(),
        input_srcs: Vec::new(),
        outputs: BTreeMap::new(),
    }
}

/// Errors that can occur during cache operations.
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("store error: {0}")]
    Store(#[from] StoreError),

    #[error("fetch error: {0}")]
    Fetch(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("compression error: {0}")]
    Compression(String),

    #[error("cache not found: {0}")]
    NotFound(String),

    #[error("invalid cache manifest: {0}")]
    InvalidManifest(String),
}

/// A cached store path with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPath {
    /// The store path
    pub path: StorePath,

    /// The derivation that produced this path
    pub derivation: Derivation,

    /// References to other store paths
    pub references: Vec<StorePath>,

    /// Size in bytes (uncompressed)
    pub size: u64,

    /// Compression format
    pub compression: CompressionFormat,

    /// Download URL (for remote caches)
    pub url: Option<String>,
}

/// Compression formats supported by the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionFormat {
    /// No compression
    None,
    /// gzip compression
    Gzip,
    /// xz compression (LZMA)
    Xz,
    /// zstd compression
    Zstd,
}

impl CompressionFormat {
    /// Get file extension for this compression format
    pub fn extension(&self) -> &'static str {
        match self {
            CompressionFormat::None => ".nar",
            CompressionFormat::Gzip => ".nar.gz",
            CompressionFormat::Xz => ".nar.xz",
            CompressionFormat::Zstd => ".nar.zst",
        }
    }
}

/// Configuration for a binary cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache name
    pub name: String,

    /// Base URL for remote cache
    pub url: Option<String>,

    /// Local directory for cache storage
    pub local_dir: Option<PathBuf>,

    /// Public key for signature verification
    pub public_key: Option<String>,

    /// Priority (higher = preferred)
    pub priority: i32,

    /// Whether to use this cache for uploads
    pub upload: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            url: None,
            local_dir: None,
            public_key: None,
            priority: 50,
            upload: false,
        }
    }
}

/// Binary cache manager.
pub struct BinaryCache {
    /// The local store
    store: Store,

    /// Configured caches (sorted by priority)
    caches: Vec<CacheConfig>,

    /// Local cache directory for downloads
    cache_dir: PathBuf,

    /// Fetcher for remote downloads
    fetcher: Fetcher,
}

impl BinaryCache {
    /// Create a new binary cache manager.
    pub fn new(store: Store) -> Result<Self, CacheError> {
        let cache_dir = store.root().join("cache");
        fs::create_dir_all(&cache_dir)?;

        let fetcher = Fetcher::new(cache_dir.clone())
            .map_err(|e: neve_fetch::FetchError| CacheError::Fetch(e.to_string()))?;

        Ok(Self {
            store,
            caches: Vec::new(),
            cache_dir,
            fetcher,
        })
    }

    /// Add a cache configuration.
    pub fn add_cache(&mut self, config: CacheConfig) {
        self.caches.push(config);
        // Sort by priority (descending)
        self.caches.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Query if a path is available in any cache.
    pub fn query(&self, path: &StorePath) -> Result<Option<CachedPath>, CacheError> {
        // Check each cache in priority order
        for cache in &self.caches {
            if let Some(cached) = self.query_cache(cache, path)? {
                return Ok(Some(cached));
            }
        }

        Ok(None)
    }

    /// Query a specific cache for a path.
    fn query_cache(
        &self,
        cache: &CacheConfig,
        path: &StorePath,
    ) -> Result<Option<CachedPath>, CacheError> {
        // Try local cache first
        if let Some(local_dir) = &cache.local_dir {
            let manifest_path = local_dir.join(format!("{}.json", path.hash()));
            if manifest_path.exists() {
                let manifest = fs::read_to_string(&manifest_path)?;
                let cached: CachedPath = serde_json::from_str(&manifest)?;
                return Ok(Some(cached));
            }
        }

        // Try remote cache
        if let Some(url) = &cache.url {
            let manifest_url = format!("{}/{}.narinfo", url, path.hash());
            if let Ok(content) = self.fetcher.fetch_text(&manifest_url) {
                let cached = self.parse_narinfo(&content, path)?;
                return Ok(Some(cached));
            }
        }

        Ok(None)
    }

    /// Download and install a cached path.
    pub fn fetch(&mut self, cached: &CachedPath) -> Result<(), CacheError> {
        // Check if already in store
        if self.store.path_exists(&cached.path) {
            return Ok(());
        }

        // Download the NAR file
        let nar_file = self.download_nar(cached)?;

        // Extract to store
        self.extract_nar(&nar_file, &cached.path)?;

        // Verify hash
        let extracted_path = self.store.to_path(&cached.path);
        let actual_hash = self.compute_path_hash(&extracted_path)?;
        if actual_hash != *cached.path.hash() {
            return Err(StoreError::HashMismatch {
                expected: *cached.path.hash(),
                actual: actual_hash,
            }
            .into());
        }

        Ok(())
    }

    /// Download a NAR file from cache.
    fn download_nar(&self, cached: &CachedPath) -> Result<PathBuf, CacheError> {
        let url = cached
            .url
            .as_ref()
            .ok_or_else(|| CacheError::NotFound("No download URL".to_string()))?;

        let filename = format!("{}{}", cached.path.hash(), cached.compression.extension());
        let dest = self.cache_dir.join(&filename);

        if !dest.exists() {
            self.fetcher
                .fetch_file(url, &dest)
                .map_err(|e: neve_fetch::FetchError| CacheError::Fetch(e.to_string()))?;
        }

        Ok(dest)
    }

    /// Extract a NAR archive to the store.
    fn extract_nar(&self, _nar_file: &Path, path: &StorePath) -> Result<(), CacheError> {
        let dest = self.store.to_path(path);

        // Create parent directory
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        // For now, this is a placeholder - actual NAR extraction would use tar/compression libs
        // In a real implementation, we would:
        // 1. Decompress based on format
        // 2. Extract tar archive
        // 3. Set permissions correctly

        // Placeholder: just create the directory
        fs::create_dir_all(&dest)?;

        Ok(())
    }

    /// Compute the hash of a store path.
    fn compute_path_hash(&self, _path: &Path) -> Result<Hash, CacheError> {
        // This would recursively hash all files in the path
        // For now, return a placeholder
        Ok(Hash::of(b"placeholder"))
    }

    /// Parse a .narinfo file.
    fn parse_narinfo(&self, content: &str, path: &StorePath) -> Result<CachedPath, CacheError> {
        let mut url = None;
        let mut size = 0;
        let mut references = Vec::new();
        let mut compression = CompressionFormat::Xz;

        for line in content.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let value = value.trim();
                match key.trim() {
                    "URL" => url = Some(value.to_string()),
                    "FileSize" => size = value.parse().unwrap_or(0),
                    "References" => {
                        references = value
                            .split_whitespace()
                            .filter_map(|s| StorePath::parse(Path::new(s)))
                            .collect();
                    }
                    "Compression" => {
                        compression = match value {
                            "none" => CompressionFormat::None,
                            "gzip" => CompressionFormat::Gzip,
                            "xz" => CompressionFormat::Xz,
                            "zstd" => CompressionFormat::Zstd,
                            _ => CompressionFormat::Xz,
                        };
                    }
                    _ => {}
                }
            }
        }

        Ok(CachedPath {
            path: path.clone(),
            derivation: placeholder_derivation(&path.to_string()),
            references,
            size,
            compression,
            url,
        })
    }

    /// Upload a store path to all writable caches.
    pub fn push(&self, path: &StorePath) -> Result<(), CacheError> {
        let store_path = self.store.to_path(path);
        if !store_path.exists() {
            return Err(CacheError::NotFound(path.to_string()));
        }

        for cache in &self.caches {
            if cache.upload {
                self.push_to_cache(cache, path)?;
            }
        }

        Ok(())
    }

    /// Upload a path to a specific cache.
    fn push_to_cache(&self, cache: &CacheConfig, path: &StorePath) -> Result<(), CacheError> {
        // Create NAR archive
        let nar_file = self.create_nar(path)?;

        // Write manifest
        let cached = CachedPath {
            path: path.clone(),
            derivation: placeholder_derivation(&path.to_string()),
            references: Vec::new(),
            size: fs::metadata(&nar_file)?.len(),
            compression: CompressionFormat::Xz,
            url: None,
        };

        // Upload to local cache
        if let Some(local_dir) = &cache.local_dir {
            let manifest_path = local_dir.join(format!("{}.json", path.hash()));
            let manifest = serde_json::to_string_pretty(&cached)?;
            fs::write(manifest_path, manifest)?;

            let nar_dest = local_dir.join(format!("{}.nar.xz", path.hash()));
            fs::copy(&nar_file, nar_dest)?;
        }

        // TODO: Upload to remote cache via HTTP PUT
        // if let Some(url) = &cache.url { ... }

        Ok(())
    }

    /// Create a NAR archive of a store path.
    fn create_nar(&self, path: &StorePath) -> Result<PathBuf, CacheError> {
        let _store_path = self.store.to_path(path);
        let nar_file = self.cache_dir.join(format!("{}.nar.xz", path.hash()));

        // In a real implementation, this would:
        // 1. Create a tar archive of the store path
        // 2. Compress with xz
        // 3. Write to nar_file

        // Placeholder: create empty file
        fs::write(&nar_file, b"")?;

        Ok(nar_file)
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_caches: self.caches.len(),
            cache_dir_size: Self::dir_size(&self.cache_dir).unwrap_or(0),
        }
    }

    fn dir_size(path: &Path) -> Result<u64, std::io::Error> {
        let mut total = 0;
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                if metadata.is_file() {
                    total += metadata.len();
                } else if metadata.is_dir() {
                    total += Self::dir_size(&entry.path())?;
                }
            }
        }
        Ok(total)
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of configured caches
    pub total_caches: usize,

    /// Size of local cache directory in bytes
    pub cache_dir_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_format_extension() {
        assert_eq!(CompressionFormat::None.extension(), ".nar");
        assert_eq!(CompressionFormat::Gzip.extension(), ".nar.gz");
        assert_eq!(CompressionFormat::Xz.extension(), ".nar.xz");
        assert_eq!(CompressionFormat::Zstd.extension(), ".nar.zst");
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.name, "default");
        assert_eq!(config.priority, 50);
        assert!(!config.upload);
    }

    #[test]
    fn test_cache_priority_sorting() {
        let store = Store::open().unwrap();
        let mut cache = BinaryCache::new(store).unwrap();

        cache.add_cache(CacheConfig {
            name: "low".to_string(),
            priority: 10,
            ..Default::default()
        });

        cache.add_cache(CacheConfig {
            name: "high".to_string(),
            priority: 100,
            ..Default::default()
        });

        cache.add_cache(CacheConfig {
            name: "medium".to_string(),
            priority: 50,
            ..Default::default()
        });

        // Should be sorted by descending priority
        assert_eq!(cache.caches[0].name, "high");
        assert_eq!(cache.caches[1].name, "medium");
        assert_eq!(cache.caches[2].name, "low");
    }
}
