//! Binary cache for pre-built derivations.
//! 预构建推导的二进制缓存。
//!
//! The binary cache allows sharing pre-built store paths between machines,
//! avoiding the need to rebuild packages from source.
//! 二进制缓存允许在机器之间共享预构建的存储路径，
//! 避免从源码重新构建包。

use crate::nar::{self, NarError};
use crate::{Store, StoreError};
use neve_derive::{Derivation, Hash, StorePath};
use neve_fetch::Fetcher;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Create a placeholder derivation for cached paths.
/// 为缓存路径创建占位推导。
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
/// 缓存操作期间可能发生的错误。
#[derive(Debug, Error)]
pub enum CacheError {
    /// Store error. / 存储错误。
    #[error("store error: {0}")]
    Store(#[from] StoreError),

    /// Fetch error. / 获取错误。
    #[error("fetch error: {0}")]
    Fetch(String),

    /// I/O error. / I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error. / 序列化错误。
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Compression error. / 压缩错误。
    #[error("compression error: {0}")]
    Compression(String),

    /// NAR error. / NAR 错误。
    #[error("NAR error: {0}")]
    Nar(#[from] NarError),

    /// Cache not found. / 未找到缓存。
    #[error("cache not found: {0}")]
    NotFound(String),

    /// Invalid cache manifest. / 无效的缓存清单。
    #[error("invalid cache manifest: {0}")]
    InvalidManifest(String),
}

/// A cached store path with metadata.
/// 带有元数据的缓存存储路径。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPath {
    /// The store path. / 存储路径。
    pub path: StorePath,

    /// The derivation that produced this path. / 产生此路径的推导。
    pub derivation: Derivation,

    /// References to other store paths. / 对其他存储路径的引用。
    pub references: Vec<StorePath>,

    /// Size in bytes (uncompressed). / 大小（字节，未压缩）。
    pub size: u64,

    /// Compression format. / 压缩格式。
    pub compression: CompressionFormat,

    /// Download URL (for remote caches). / 下载 URL（用于远程缓存）。
    pub url: Option<String>,
}

/// Compression formats supported by the cache.
/// 缓存支持的压缩格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionFormat {
    /// No compression. / 无压缩。
    None,
    /// gzip compression. / gzip 压缩。
    Gzip,
    /// xz compression (LZMA). / xz 压缩 (LZMA)。
    Xz,
    /// zstd compression. / zstd 压缩。
    Zstd,
}

impl CompressionFormat {
    /// Get file extension for this compression format.
    /// 获取此压缩格式的文件扩展名。
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
/// 二进制缓存的配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache name. / 缓存名称。
    pub name: String,

    /// Base URL for remote cache. / 远程缓存的基础 URL。
    pub url: Option<String>,

    /// Local directory for cache storage. / 缓存存储的本地目录。
    pub local_dir: Option<PathBuf>,

    /// Public key for signature verification. / 用于签名验证的公钥。
    pub public_key: Option<String>,

    /// Priority (higher = preferred). / 优先级（越高越优先）。
    pub priority: i32,

    /// Whether to use this cache for uploads. / 是否使用此缓存进行上传。
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
/// 二进制缓存管理器。
pub struct BinaryCache {
    /// The local store. / 本地存储。
    store: Store,

    /// Configured caches (sorted by priority). / 配置的缓存（按优先级排序）。
    caches: Vec<CacheConfig>,

    /// Local cache directory for downloads. / 下载的本地缓存目录。
    cache_dir: PathBuf,

    /// Fetcher for remote downloads. / 用于远程下载的获取器。
    fetcher: Fetcher,
}

impl BinaryCache {
    /// Create a new binary cache manager.
    /// 创建新的二进制缓存管理器。
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
    /// 添加缓存配置。
    pub fn add_cache(&mut self, config: CacheConfig) {
        self.caches.push(config);
        // Sort by priority (descending)
        // 按优先级排序（降序）
        self.caches.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Query if a path is available in any cache.
    /// 查询路径是否在任何缓存中可用。
    pub fn query(&self, path: &StorePath) -> Result<Option<CachedPath>, CacheError> {
        // Check each cache in priority order
        // 按优先级顺序检查每个缓存
        for cache in &self.caches {
            if let Some(cached) = self.query_cache(cache, path)? {
                return Ok(Some(cached));
            }
        }

        Ok(None)
    }

    /// Query a specific cache for a path.
    /// 在特定缓存中查询路径。
    fn query_cache(
        &self,
        cache: &CacheConfig,
        path: &StorePath,
    ) -> Result<Option<CachedPath>, CacheError> {
        // Try local cache first
        // 首先尝试本地缓存
        if let Some(local_dir) = &cache.local_dir {
            let manifest_path = local_dir.join(format!("{}.json", path.hash()));
            if manifest_path.exists() {
                let manifest = fs::read_to_string(&manifest_path)?;
                let cached: CachedPath = serde_json::from_str(&manifest)?;
                return Ok(Some(cached));
            }
        }

        // Try remote cache
        // 尝试远程缓存
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
    /// 下载并安装缓存的路径。
    pub fn fetch(&mut self, cached: &CachedPath) -> Result<(), CacheError> {
        // Check if already in store
        // 检查是否已在存储中
        if self.store.path_exists(&cached.path) {
            return Ok(());
        }

        // Download the NAR file
        // 下载 NAR 文件
        let nar_file = self.download_nar(cached)?;

        // Extract to store
        // 提取到存储
        self.extract_nar(&nar_file, &cached.path)?;

        // Verify hash
        // 验证哈希
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
    /// 从缓存下载 NAR 文件。
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
    /// 将 NAR 归档提取到存储。
    fn extract_nar(&self, nar_file: &Path, path: &StorePath) -> Result<(), CacheError> {
        let dest = self.store.to_path(path);

        // Create parent directory
        // 创建父目录
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        // Read the compressed NAR file
        // 读取压缩的 NAR 文件
        let compressed_data = fs::read(nar_file)?;

        // Decompress based on file extension
        // 根据文件扩展名解压
        let nar_data = self.decompress_nar(&compressed_data, nar_file)?;

        // Extract using our NAR implementation
        // 使用我们的 NAR 实现提取
        nar::extract_nar(&nar_data, &dest)?;

        Ok(())
    }

    /// Decompress NAR data based on file extension.
    /// 根据文件扩展名解压 NAR 数据。
    fn decompress_nar(&self, data: &[u8], path: &Path) -> Result<Vec<u8>, CacheError> {
        let path_str = path.to_string_lossy();

        if path_str.ends_with(".nar") {
            // No compression
            // 无压缩
            Ok(data.to_vec())
        } else if path_str.ends_with(".nar.gz") {
            // gzip decompression
            // gzip 解压
            let mut decoder = flate2::read::GzDecoder::new(data);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).map_err(|e| {
                CacheError::Compression(format!("gzip decompression failed: {}", e))
            })?;
            Ok(decompressed)
        } else if path_str.ends_with(".nar.xz") {
            // xz decompression
            // xz 解压
            let mut decompressed = Vec::new();
            lzma_rs::xz_decompress(&mut std::io::Cursor::new(data), &mut decompressed)
                .map_err(|e| CacheError::Compression(format!("xz decompression failed: {}", e)))?;
            Ok(decompressed)
        } else if path_str.ends_with(".nar.zst") {
            // zstd decompression
            // zstd 解压
            zstd::decode_all(std::io::Cursor::new(data))
                .map_err(|e| CacheError::Compression(format!("zstd decompression failed: {}", e)))
        } else {
            // Assume uncompressed
            // 假设未压缩
            Ok(data.to_vec())
        }
    }

    /// Compute the hash of a store path using NAR format.
    /// 使用 NAR 格式计算存储路径的哈希。
    fn compute_path_hash(&self, path: &Path) -> Result<Hash, CacheError> {
        // Hash the path using NAR format for deterministic results
        // 使用 NAR 格式哈希路径以获得确定性结果
        let hash = nar::hash_path(path)?;
        Ok(hash)
    }

    /// Parse a .narinfo file.
    /// 解析 .narinfo 文件。
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
    /// 将存储路径上传到所有可写缓存。
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
    /// 将路径上传到特定缓存。
    fn push_to_cache(&self, cache: &CacheConfig, path: &StorePath) -> Result<(), CacheError> {
        // Create NAR archive
        // 创建 NAR 归档
        let nar_file = self.create_nar(path)?;

        // Write manifest
        // 写入清单
        let cached = CachedPath {
            path: path.clone(),
            derivation: placeholder_derivation(&path.to_string()),
            references: Vec::new(),
            size: fs::metadata(&nar_file)?.len(),
            compression: CompressionFormat::Xz,
            url: None,
        };

        // Upload to local cache
        // 上传到本地缓存
        if let Some(local_dir) = &cache.local_dir {
            let manifest_path = local_dir.join(format!("{}.json", path.hash()));
            let manifest = serde_json::to_string_pretty(&cached)?;
            fs::write(manifest_path, manifest)?;

            let nar_dest = local_dir.join(format!("{}.nar.xz", path.hash()));
            fs::copy(&nar_file, nar_dest)?;
        }

        // TODO: Upload to remote cache via HTTP PUT
        // 待办：通过 HTTP PUT 上传到远程缓存
        // if let Some(url) = &cache.url { ... }

        Ok(())
    }

    /// Create a NAR archive of a store path.
    /// 创建存储路径的 NAR 归档。
    fn create_nar(&self, path: &StorePath) -> Result<PathBuf, CacheError> {
        let store_path = self.store.to_path(path);
        let nar_file = self.cache_dir.join(format!("{}.nar.xz", path.hash()));

        // Create NAR archive using our implementation
        // 使用我们的实现创建 NAR 归档
        let nar_data = nar::create_nar(&store_path)?;

        // Compress with xz
        // 使用 xz 压缩
        let compressed = self.compress_nar(&nar_data, CompressionFormat::Xz)?;

        // Write to file
        // 写入文件
        fs::write(&nar_file, compressed)?;

        Ok(nar_file)
    }

    /// Compress NAR data with the specified format.
    /// 使用指定格式压缩 NAR 数据。
    fn compress_nar(&self, data: &[u8], format: CompressionFormat) -> Result<Vec<u8>, CacheError> {
        match format {
            CompressionFormat::None => Ok(data.to_vec()),
            CompressionFormat::Gzip => {
                let mut encoder =
                    flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(data).map_err(|e| {
                    CacheError::Compression(format!("gzip compression failed: {}", e))
                })?;
                encoder
                    .finish()
                    .map_err(|e| CacheError::Compression(format!("gzip finish failed: {}", e)))
            }
            CompressionFormat::Xz => {
                let mut compressed = Vec::new();
                lzma_rs::xz_compress(&mut std::io::Cursor::new(data), &mut compressed).map_err(
                    |e| CacheError::Compression(format!("xz compression failed: {}", e)),
                )?;
                Ok(compressed)
            }
            CompressionFormat::Zstd => zstd::encode_all(std::io::Cursor::new(data), 3)
                .map_err(|e| CacheError::Compression(format!("zstd compression failed: {}", e))),
        }
    }

    /// Get cache statistics.
    /// 获取缓存统计信息。
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_caches: self.caches.len(),
            cache_dir_size: Self::dir_size(&self.cache_dir).unwrap_or(0),
        }
    }

    /// Calculate directory size.
    /// 计算目录大小。
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
/// 缓存统计信息。
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of configured caches. / 配置的缓存数量。
    pub total_caches: usize,

    /// Size of local cache directory in bytes. / 本地缓存目录的大小（字节）。
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
        let temp = tempfile::TempDir::new().unwrap();
        let store = Store::open_at(temp.path().to_path_buf()).unwrap();
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
        // 应按优先级降序排序
        assert_eq!(cache.caches[0].name, "high");
        assert_eq!(cache.caches[1].name, "medium");
        assert_eq!(cache.caches[2].name, "low");
    }
}
