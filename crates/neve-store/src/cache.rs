//! Binary cache for pre-built derivations.
//! 预构建推导的二进制缓存。
//!
//! The binary cache allows sharing pre-built store paths between machines,
//! avoiding the need to rebuild packages from source.
//! 二进制缓存允许在机器之间共享预构建的存储路径，
//! 避免从源码重新构建包。

use crate::nar::{self, NarError};
use crate::{Database, PathInfo, Store, StoreError};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use neve_derive::{Derivation, Hash, StorePath};
use neve_fetch::{FetchError, Fetcher};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
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

    /// Hash mismatch during substitute verification. / 替换验证期间哈希不匹配。
    #[error("hash mismatch for {kind}: expected {expected}, got {actual}")]
    HashMismatch {
        kind: &'static str,
        expected: String,
        actual: String,
    },

    /// Signature verification failure. / 签名验证失败。
    #[error("signature verification failed: {0}")]
    Signature(String),
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

    /// Expected hash of compressed NAR payload (optional). / 压缩 NAR 载荷的预期哈希（可选）。
    pub file_hash: Option<String>,

    /// Expected hash of decompressed NAR bytes (optional). / 解压后 NAR 字节的预期哈希（可选）。
    pub nar_hash: Option<String>,
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

    /// Parse compression name from narinfo.
    /// 从 narinfo 解析压缩格式名称。
    fn from_narinfo(value: &str) -> Option<Self> {
        match value {
            "none" => Some(CompressionFormat::None),
            "gzip" => Some(CompressionFormat::Gzip),
            "xz" => Some(CompressionFormat::Xz),
            "zstd" => Some(CompressionFormat::Zstd),
            _ => None,
        }
    }

    /// Render compression name for narinfo.
    /// 渲染用于 narinfo 的压缩格式名称。
    fn as_narinfo(&self) -> &'static str {
        match self {
            CompressionFormat::None => "none",
            CompressionFormat::Gzip => "gzip",
            CompressionFormat::Xz => "xz",
            CompressionFormat::Zstd => "zstd",
        }
    }
}

/// Parsed narinfo metadata for substituter protocol.
/// substituter 协议的 narinfo 元数据。
#[derive(Debug, Clone)]
struct NarInfo {
    store_path: StorePath,
    url: String,
    compression: CompressionFormat,
    file_size: u64,
    nar_size: Option<u64>,
    references: Vec<StorePath>,
    nar_hash: Option<String>,
    file_hash: Option<String>,
    signature: Option<String>,
}

impl NarInfo {
    fn parse(content: &str) -> Result<Self, CacheError> {
        let mut store_path = None;
        let mut url = None;
        let mut compression = CompressionFormat::Xz;
        let mut file_size = None;
        let mut nar_size = None;
        let mut references = Vec::new();
        let mut nar_hash = None;
        let mut file_hash = None;
        let mut signature = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once(':') else {
                return Err(CacheError::InvalidManifest(format!(
                    "invalid narinfo line: {}",
                    line
                )));
            };
            let key = key.trim();
            let value = value.trim();

            match key {
                "StorePath" => {
                    store_path = parse_store_path_token(value, "StorePath")?;
                }
                "URL" => {
                    if value.is_empty() {
                        return Err(CacheError::InvalidManifest(
                            "narinfo URL cannot be empty".to_string(),
                        ));
                    }
                    url = Some(value.to_string());
                }
                "Compression" => {
                    compression = CompressionFormat::from_narinfo(value).ok_or_else(|| {
                        CacheError::InvalidManifest(format!(
                            "unsupported narinfo compression '{}'",
                            value
                        ))
                    })?;
                }
                "FileSize" => {
                    file_size = Some(value.parse::<u64>().map_err(|_| {
                        CacheError::InvalidManifest(format!("invalid FileSize '{}'", value))
                    })?);
                }
                "NarSize" => {
                    nar_size = Some(value.parse::<u64>().map_err(|_| {
                        CacheError::InvalidManifest(format!("invalid NarSize '{}'", value))
                    })?);
                }
                "References" => {
                    references = value
                        .split_whitespace()
                        .map(|token| {
                            parse_store_path_token(token, "References").and_then(|entry| {
                                entry.ok_or_else(|| {
                                    CacheError::InvalidManifest(format!(
                                        "invalid reference store path '{}'",
                                        token
                                    ))
                                })
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                }
                "NarHash" => {
                    if !value.is_empty() {
                        nar_hash = Some(value.to_string());
                    }
                }
                "FileHash" => {
                    if !value.is_empty() {
                        file_hash = Some(value.to_string());
                    }
                }
                "Sig" => {
                    if !value.is_empty() {
                        signature = Some(value.to_string());
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            store_path: store_path.ok_or_else(|| {
                CacheError::InvalidManifest("narinfo missing StorePath".to_string())
            })?,
            url: url
                .ok_or_else(|| CacheError::InvalidManifest("narinfo missing URL".to_string()))?,
            compression,
            file_size: file_size.ok_or_else(|| {
                CacheError::InvalidManifest("narinfo missing FileSize".to_string())
            })?,
            nar_size,
            references,
            nar_hash,
            file_hash,
            signature,
        })
    }

    fn to_unsigned_text(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("StorePath: {}", self.store_path.display_name()));
        lines.push(format!("URL: {}", self.url));
        lines.push(format!("Compression: {}", self.compression.as_narinfo()));
        lines.push(format!("FileSize: {}", self.file_size));
        if let Some(file_hash) = &self.file_hash {
            lines.push(format!("FileHash: {}", file_hash));
        }
        if let Some(nar_hash) = &self.nar_hash {
            lines.push(format!("NarHash: {}", nar_hash));
        }
        if let Some(nar_size) = self.nar_size {
            lines.push(format!("NarSize: {}", nar_size));
        }
        if !self.references.is_empty() {
            let refs = self
                .references
                .iter()
                .map(StorePath::display_name)
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(format!("References: {}", refs));
        }
        lines.push(String::new());
        lines.join("\n")
    }

    fn to_text(&self) -> String {
        let mut lines = self
            .to_unsigned_text()
            .lines()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }
        if let Some(signature) = &self.signature {
            lines.push(format!("Sig: {}", signature));
        }
        lines.push(String::new());
        lines.join("\n")
    }
}

#[derive(Debug, Clone, Copy)]
enum NarInfoSource<'a> {
    Local(&'a Path),
    Remote(&'a str),
}

fn parse_store_path_token(token: &str, field: &str) -> Result<Option<StorePath>, CacheError> {
    if token.is_empty() {
        return Ok(None);
    }

    if let Some(store_path) = StorePath::parse(Path::new(token)) {
        return Ok(Some(store_path));
    }

    if let Some(store_path) = StorePath::parse_name(token) {
        return Ok(Some(store_path));
    }

    Err(CacheError::InvalidManifest(format!(
        "invalid {} store path '{}'",
        field, token
    )))
}

fn is_absolute_cache_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("file://")
}

fn resolve_narinfo_url(url: &str, source: NarInfoSource<'_>) -> String {
    if is_absolute_cache_url(url) {
        return url.to_string();
    }

    match source {
        NarInfoSource::Local(base) => base.join(url).to_string_lossy().to_string(),
        NarInfoSource::Remote(base) => {
            format!(
                "{}/{}",
                base.trim_end_matches('/'),
                url.trim_start_matches('/')
            )
        }
    }
}

fn parse_cache_hash(input: Option<&str>) -> Result<Option<Hash>, CacheError> {
    let Some(raw) = input else {
        return Ok(None);
    };

    let normalized = raw
        .trim()
        .strip_prefix("blake3:")
        .or_else(|| raw.trim().strip_prefix("blake3-"))
        .unwrap_or(raw.trim());

    if normalized.is_empty() {
        return Ok(None);
    }

    Hash::from_hex(normalized).map(Some).map_err(|_| {
        CacheError::InvalidManifest(format!("invalid blake3 hash format '{}'", raw.trim()))
    })
}

fn format_hash(hash: &Hash) -> String {
    format!("blake3:{}", hash.to_hex())
}

const REMOTE_RETRY_ATTEMPTS: usize = 3;
const REMOTE_RETRY_BASE_DELAY_MS: u64 = 150;

fn remote_retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(REMOTE_RETRY_BASE_DELAY_MS * (attempt as u64 + 1))
}

fn should_retry_http_status(status: reqwest::StatusCode) -> bool {
    status.is_server_error() || status.as_u16() == 429
}

fn should_retry_reqwest_error(error: &reqwest::Error) -> bool {
    if let Some(status) = error.status() {
        return should_retry_http_status(status);
    }
    error.is_timeout() || error.is_connect()
}

fn should_retry_fetch_error(error: &FetchError) -> bool {
    match error {
        FetchError::Http(http_error) => should_retry_reqwest_error(http_error),
        _ => false,
    }
}

fn is_fetch_http_status(error: &FetchError, status: reqwest::StatusCode) -> bool {
    matches!(
        error,
        FetchError::Http(http_error) if http_error.status() == Some(status)
    )
}

fn decode_prefixed_base64<const N: usize>(
    input: &str,
    prefix: &str,
    field_name: &str,
) -> Result<[u8; N], CacheError> {
    let trimmed = input.trim();
    let encoded = trimmed.strip_prefix(prefix).ok_or_else(|| {
        CacheError::Signature(format!(
            "{} must start with '{}' (got '{}')",
            field_name, prefix, trimmed
        ))
    })?;

    let decoded = BASE64_STANDARD
        .decode(encoded)
        .map_err(|e| CacheError::Signature(format!("{} is not valid base64: {}", field_name, e)))?;
    decoded.try_into().map_err(|_| {
        CacheError::Signature(format!(
            "{} decoded length mismatch: expected {} bytes",
            field_name, N
        ))
    })
}

fn parse_ed25519_public_key(value: &str) -> Result<VerifyingKey, CacheError> {
    let key_bytes = decode_prefixed_base64::<32>(value, "ed25519:", "cache public key")?;
    VerifyingKey::from_bytes(&key_bytes).map_err(|e| {
        CacheError::Signature(format!("cache public key is invalid ed25519 bytes: {}", e))
    })
}

fn parse_ed25519_private_key(value: &str) -> Result<SigningKey, CacheError> {
    let key_bytes = decode_prefixed_base64::<32>(value, "ed25519:", "cache private key")?;
    Ok(SigningKey::from_bytes(&key_bytes))
}

fn parse_ed25519_signature(value: &str) -> Result<Signature, CacheError> {
    let sig_bytes = decode_prefixed_base64::<64>(value, "ed25519:", "narinfo Sig")?;
    Ok(Signature::from_bytes(&sig_bytes))
}

fn verify_narinfo_signature(narinfo: &NarInfo, public_key: &str) -> Result<(), CacheError> {
    let signature = narinfo.signature.as_deref().ok_or_else(|| {
        CacheError::Signature("narinfo is missing Sig but cache requires signatures".to_string())
    })?;
    let verifying_key = parse_ed25519_public_key(public_key)?;
    let signature = parse_ed25519_signature(signature)?;

    let payload = narinfo.to_unsigned_text();
    verifying_key
        .verify(payload.as_bytes(), &signature)
        .map_err(|_| {
            CacheError::Signature(format!(
                "narinfo Sig validation failed for {}",
                narinfo.store_path.display_name()
            ))
        })
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

    /// Private key for narinfo signing on upload. / 上传 narinfo 时使用的私钥。
    pub private_key: Option<String>,

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
            private_key: None,
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
        self.caches
            .sort_by_key(|entry| std::cmp::Reverse(entry.priority));
    }

    fn fetch_text_with_retry(&self, url: &str) -> Result<String, CacheError> {
        for attempt in 0..REMOTE_RETRY_ATTEMPTS {
            match self.fetcher.fetch_text(url) {
                Ok(content) => return Ok(content),
                Err(err) => {
                    if is_fetch_http_status(&err, reqwest::StatusCode::NOT_FOUND) {
                        return Err(CacheError::NotFound(url.to_string()));
                    }
                    if should_retry_fetch_error(&err) && attempt + 1 < REMOTE_RETRY_ATTEMPTS {
                        std::thread::sleep(remote_retry_delay(attempt));
                        continue;
                    }
                    return Err(CacheError::Fetch(format!(
                        "failed to fetch remote text {}: {}",
                        url, err
                    )));
                }
            }
        }

        Err(CacheError::Fetch(format!(
            "failed to fetch remote text after {} attempts: {}",
            REMOTE_RETRY_ATTEMPTS, url
        )))
    }

    fn fetch_file_with_retry(&self, url: &str, dest: &Path) -> Result<(), CacheError> {
        for attempt in 0..REMOTE_RETRY_ATTEMPTS {
            match self.fetcher.fetch_file(url, dest) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    if is_fetch_http_status(&err, reqwest::StatusCode::NOT_FOUND) {
                        return Err(CacheError::NotFound(url.to_string()));
                    }
                    if should_retry_fetch_error(&err) && attempt + 1 < REMOTE_RETRY_ATTEMPTS {
                        std::thread::sleep(remote_retry_delay(attempt));
                        continue;
                    }
                    return Err(CacheError::Fetch(format!(
                        "failed to fetch remote file {}: {}",
                        url, err
                    )));
                }
            }
        }

        Err(CacheError::Fetch(format!(
            "failed to fetch remote file after {} attempts: {}",
            REMOTE_RETRY_ATTEMPTS, url
        )))
    }

    /// Query if a path is available in any cache.
    /// 查询路径是否在任何缓存中可用。
    pub fn query(&self, path: &StorePath) -> Result<Option<CachedPath>, CacheError> {
        let mut deferred_fetch_error = None;

        // Check each cache in priority order
        // 按优先级顺序检查每个缓存
        for cache in &self.caches {
            match self.query_cache(cache, path) {
                Ok(Some(cached)) => return Ok(Some(cached)),
                Ok(None) => {}
                Err(CacheError::Fetch(message)) => {
                    deferred_fetch_error = Some(CacheError::Fetch(message));
                }
                Err(err) => return Err(err),
            }
        }

        if let Some(err) = deferred_fetch_error {
            return Err(err);
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
            let narinfo_path = local_dir.join(format!("{}.narinfo", path.hash()));
            if narinfo_path.exists() {
                let narinfo = fs::read_to_string(&narinfo_path)?;
                let cached = self.parse_narinfo(
                    &narinfo,
                    path,
                    NarInfoSource::Local(local_dir),
                    cache.public_key.as_deref(),
                )?;
                return Ok(Some(cached));
            }

            let manifest_path = local_dir.join(format!("{}.json", path.hash()));
            if manifest_path.exists() {
                let manifest = fs::read_to_string(&manifest_path)?;
                let mut cached: CachedPath = serde_json::from_str(&manifest)?;
                if cached.url.is_none() {
                    let nar_path = local_dir.join(format!(
                        "{}{}",
                        path.hash(),
                        cached.compression.extension()
                    ));
                    cached.url = Some(nar_path.to_string_lossy().to_string());
                }
                return Ok(Some(cached));
            }
        }

        // Try remote cache
        // 尝试远程缓存
        if let Some(url) = &cache.url {
            let manifest_url = format!("{}/{}.narinfo", url, path.hash());
            match self.fetch_text_with_retry(&manifest_url) {
                Ok(content) => {
                    let cached = self.parse_narinfo(
                        &content,
                        path,
                        NarInfoSource::Remote(url),
                        cache.public_key.as_deref(),
                    )?;
                    return Ok(Some(cached));
                }
                Err(CacheError::NotFound(_)) => {}
                Err(err) => return Err(err),
            }
        }

        Ok(None)
    }

    /// Download and install a cached path.
    /// 下载并安装缓存的路径。
    pub fn fetch(&mut self, cached: &CachedPath) -> Result<(), CacheError> {
        let mut visiting = HashSet::new();
        self.fetch_with_references(cached, &mut visiting)
    }

    fn fetch_with_references(
        &mut self,
        cached: &CachedPath,
        visiting: &mut HashSet<StorePath>,
    ) -> Result<(), CacheError> {
        if !visiting.insert(cached.path.clone()) {
            // Cyclic references in metadata should not cause infinite recursion.
            // 元数据中的循环引用不应导致无限递归。
            return Ok(());
        }

        let result = (|| -> Result<(), CacheError> {
            // Ensure metadata references are also present in the local store.
            // 确保元数据引用的路径也存在于本地存储中。
            for reference in &cached.references {
                if self.store.path_exists(reference) {
                    continue;
                }

                let reference_cached = self.query(reference)?.ok_or_else(|| {
                    CacheError::NotFound(format!(
                        "missing referenced path {} required by {}",
                        reference.display_name(),
                        cached.path.display_name()
                    ))
                })?;

                self.fetch_with_references(&reference_cached, visiting)?;
            }

            // Fetch current path after references are present. This prevents
            // leaving a partially available closure when references are missing.
            // 在引用就绪后再拉取当前路径，避免缺失引用时留下半可用闭包。
            if !self.store.path_exists(&cached.path) {
                let nar_file = self.download_nar(cached)?;
                self.verify_downloaded_file_hash(cached, &nar_file)?;

                let extracted_nar_hash = self.extract_nar(&nar_file, &cached.path)?;
                self.verify_extracted_nar_hash(cached, &extracted_nar_hash)?;
                self.register_fetched_path_info(cached, Some(extracted_nar_hash))?;

                let extracted_path = self.store.to_path(&cached.path);
                let actual_hash = self.compute_path_hash(&extracted_path)?;
                if actual_hash != *cached.path.hash() {
                    return Err(StoreError::HashMismatch {
                        expected: *cached.path.hash(),
                        actual: actual_hash,
                    }
                    .into());
                }
            } else {
                self.register_fetched_path_info(cached, None)?;
            }

            Ok(())
        })();

        visiting.remove(&cached.path);
        result
    }

    fn register_fetched_path_info(
        &self,
        cached: &CachedPath,
        extracted_nar_hash: Option<Hash>,
    ) -> Result<(), CacheError> {
        let mut db = Database::open(self.store.root().to_path_buf())?;
        let nar_hash = if let Some(hash) = extracted_nar_hash {
            hash
        } else {
            parse_cache_hash(cached.nar_hash.as_deref())?.unwrap_or(*cached.path.hash())
        };

        let mut info = PathInfo::new(cached.path.clone(), nar_hash, cached.size);
        for reference in &cached.references {
            if reference != &cached.path {
                info.add_reference(reference.clone());
            }
        }
        db.register(info)?;
        Ok(())
    }

    /// Verify downloaded compressed NAR hash if metadata provides it.
    /// 若元数据提供压缩包哈希则校验下载结果。
    fn verify_downloaded_file_hash(
        &self,
        cached: &CachedPath,
        nar_file: &Path,
    ) -> Result<(), CacheError> {
        let Some(expected) = parse_cache_hash(cached.file_hash.as_deref())? else {
            return Ok(());
        };

        let content = fs::read(nar_file)?;
        let actual = Hash::of(&content);
        if actual != expected {
            return Err(CacheError::HashMismatch {
                kind: "nar-compressed",
                expected: format_hash(&expected),
                actual: format_hash(&actual),
            });
        }

        Ok(())
    }

    /// Verify decompressed NAR hash if metadata provides it.
    /// 若元数据提供 NAR 哈希则校验解压结果。
    fn verify_extracted_nar_hash(
        &self,
        cached: &CachedPath,
        actual_hash: &Hash,
    ) -> Result<(), CacheError> {
        let Some(expected) = parse_cache_hash(cached.nar_hash.as_deref())? else {
            return Ok(());
        };

        if *actual_hash != expected {
            return Err(CacheError::HashMismatch {
                kind: "nar",
                expected: format_hash(&expected),
                actual: format_hash(actual_hash),
            });
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
            if let Some(local_path) = url.strip_prefix("file://") {
                Self::copy_local_nar(Path::new(local_path), &dest)?;
            } else {
                let local_path = Path::new(url);
                if local_path.is_absolute() && local_path.exists() {
                    Self::copy_local_nar(local_path, &dest)?;
                } else {
                    self.fetch_file_with_retry(url, &dest)?;
                }
            }
        }

        Ok(dest)
    }

    /// Copy a local NAR file into the binary cache download directory.
    /// 将本地 NAR 文件复制到二进制缓存下载目录。
    fn copy_local_nar(source: &Path, dest: &Path) -> Result<(), CacheError> {
        if !source.exists() {
            return Err(CacheError::NotFound(source.to_string_lossy().to_string()));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, dest)?;
        Ok(())
    }

    /// Extract a NAR archive to the store.
    /// 将 NAR 归档提取到存储。
    fn extract_nar(&self, nar_file: &Path, path: &StorePath) -> Result<Hash, CacheError> {
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
        let nar_hash = Hash::of(&nar_data);

        // Extract using our NAR implementation
        // 使用我们的 NAR 实现提取
        nar::extract_nar(&nar_data, &dest)?;

        Ok(nar_hash)
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
    fn parse_narinfo(
        &self,
        content: &str,
        expected_path: &StorePath,
        source: NarInfoSource<'_>,
        required_public_key: Option<&str>,
    ) -> Result<CachedPath, CacheError> {
        let narinfo = NarInfo::parse(content)?;
        if narinfo.store_path != *expected_path {
            return Err(CacheError::InvalidManifest(format!(
                "narinfo StorePath mismatch: expected '{}', got '{}'",
                expected_path.display_name(),
                narinfo.store_path.display_name()
            )));
        }
        if let Some(public_key) = required_public_key {
            verify_narinfo_signature(&narinfo, public_key)?;
        }

        let file_hash = narinfo.file_hash.clone();
        let nar_hash = narinfo.nar_hash.clone();
        // Validate hash format eagerly to fail fast on malformed narinfo metadata.
        // 预先校验哈希格式，尽早拒绝损坏的 narinfo 元数据。
        parse_cache_hash(file_hash.as_deref())?;
        parse_cache_hash(nar_hash.as_deref())?;

        let resolved_url = resolve_narinfo_url(&narinfo.url, source);

        Ok(CachedPath {
            path: narinfo.store_path.clone(),
            derivation: placeholder_derivation(&narinfo.store_path.display_name()),
            references: narinfo.references.clone(),
            size: narinfo.nar_size.unwrap_or(narinfo.file_size),
            compression: narinfo.compression,
            url: Some(resolved_url),
            file_hash,
            nar_hash,
        })
    }

    fn maybe_sign_narinfo(
        &self,
        cache: &CacheConfig,
        narinfo: &mut NarInfo,
    ) -> Result<(), CacheError> {
        let private_key = match cache.private_key.as_deref() {
            Some(private_key) => private_key,
            None => {
                if cache.public_key.is_some() {
                    return Err(CacheError::Signature(
                        "cache has public_key but no private_key for narinfo signing".to_string(),
                    ));
                }
                return Ok(());
            }
        };

        let signing_key = parse_ed25519_private_key(private_key)?;
        let signature = signing_key.sign(narinfo.to_unsigned_text().as_bytes());
        narinfo.signature = Some(format!(
            "ed25519:{}",
            BASE64_STANDARD.encode(signature.to_bytes())
        ));
        Ok(())
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
        let (nar_file, nar_size, nar_hash) = self.create_nar(path)?;
        let file_hash = Hash::of(&fs::read(&nar_file)?);
        let file_size = fs::metadata(&nar_file)?.len();
        let references = self.discover_references(path)?;

        // Write manifest
        // 写入清单
        let cached = CachedPath {
            path: path.clone(),
            derivation: placeholder_derivation(&path.to_string()),
            references: references.clone(),
            size: nar_size,
            compression: CompressionFormat::Xz,
            url: None,
            file_hash: Some(format_hash(&file_hash)),
            nar_hash: Some(format_hash(&nar_hash)),
        };

        // Upload to local cache
        // 上传到本地缓存
        if let Some(local_dir) = &cache.local_dir {
            fs::create_dir_all(local_dir)?;

            let nar_filename = format!("{}{}", path.hash(), CompressionFormat::Xz.extension());
            let nar_dest = local_dir.join(&nar_filename);
            fs::copy(&nar_file, &nar_dest)?;

            let mut narinfo = NarInfo {
                store_path: path.clone(),
                url: nar_filename,
                compression: CompressionFormat::Xz,
                file_size,
                nar_size: Some(nar_size),
                references: references.clone(),
                nar_hash: Some(format_hash(&nar_hash)),
                file_hash: Some(format_hash(&file_hash)),
                signature: None,
            };
            self.maybe_sign_narinfo(cache, &mut narinfo)?;
            fs::write(
                local_dir.join(format!("{}.narinfo", path.hash())),
                narinfo.to_text(),
            )?;

            let manifest_path = local_dir.join(format!("{}.json", path.hash()));
            let mut cached = cached.clone();
            cached.url = Some(nar_dest.to_string_lossy().to_string());
            let manifest = serde_json::to_string_pretty(&cached)?;
            fs::write(manifest_path, manifest)?;
        }

        if let Some(remote_url) = &cache.url {
            let mut narinfo = NarInfo {
                store_path: path.clone(),
                url: format!("{}{}", path.hash(), CompressionFormat::Xz.extension()),
                compression: CompressionFormat::Xz,
                file_size,
                nar_size: Some(nar_size),
                references: references.clone(),
                nar_hash: Some(format_hash(&nar_hash)),
                file_hash: Some(format_hash(&file_hash)),
                signature: None,
            };
            self.maybe_sign_narinfo(cache, &mut narinfo)?;
            self.push_to_remote_cache(remote_url, path, &nar_file, &narinfo.to_text())?;
        }

        // Remote upload uses a minimal HTTP PUT protocol:
        //   <base>/<hash>.nar.xz and <base>/<hash>.narinfo
        // 远程上传使用最小 HTTP PUT 协议：
        //   <base>/<hash>.nar.xz 与 <base>/<hash>.narinfo

        Ok(())
    }

    fn discover_references(&self, path: &StorePath) -> Result<Vec<StorePath>, CacheError> {
        if let Some(references) = self.try_references_from_database(path) {
            return Ok(references);
        }

        let store_path = self.store.to_path(path);
        let mut references = HashSet::new();
        self.discover_references_in_path(&store_path, path, &mut references)?;
        let mut references = references.into_iter().collect::<Vec<_>>();
        references.sort();
        Ok(references)
    }

    fn try_references_from_database(&self, path: &StorePath) -> Option<Vec<StorePath>> {
        let mut db = Database::open(self.store.root().to_path_buf()).ok()?;
        let info = db.query(path).ok()??;
        if info.references.is_empty() {
            return None;
        }

        let mut references = info
            .references
            .into_iter()
            .filter(|reference| reference != path && self.store.path_exists(reference))
            .collect::<Vec<_>>();
        references.sort();
        Some(references)
    }

    fn discover_references_in_path(
        &self,
        fs_path: &Path,
        current: &StorePath,
        references: &mut HashSet<StorePath>,
    ) -> Result<(), CacheError> {
        let metadata = fs::symlink_metadata(fs_path)?;

        if metadata.file_type().is_symlink() {
            let target = fs::read_link(fs_path)?;
            self.extract_references_from_bytes(
                target.to_string_lossy().as_bytes(),
                current,
                references,
            );
            return Ok(());
        }

        if metadata.is_dir() {
            for entry in fs::read_dir(fs_path)? {
                let entry = entry?;
                self.discover_references_in_path(&entry.path(), current, references)?;
            }
            return Ok(());
        }

        if metadata.is_file() {
            let content = fs::read(fs_path)?;
            self.extract_references_from_bytes(&content, current, references);
        }

        Ok(())
    }

    fn extract_references_from_bytes(
        &self,
        content: &[u8],
        current: &StorePath,
        references: &mut HashSet<StorePath>,
    ) {
        let text = String::from_utf8_lossy(content);
        for token in text
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '+')))
        {
            if token.len() <= 65 {
                continue;
            }

            let Some(candidate) = StorePath::parse_name(token) else {
                continue;
            };
            if candidate == *current {
                continue;
            }
            if self.store.path_exists(&candidate) {
                references.insert(candidate);
            }
        }
    }

    /// Upload NAR payload and narinfo metadata to a remote cache via HTTP PUT.
    /// 通过 HTTP PUT 上传 NAR 载荷与 narinfo 元数据到远程缓存。
    fn push_to_remote_cache(
        &self,
        base_url: &str,
        path: &StorePath,
        nar_file: &Path,
        narinfo_text: &str,
    ) -> Result<(), CacheError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| CacheError::Fetch(format!("failed to build HTTP client: {}", e)))?;

        let nar_name = format!("{}{}", path.hash(), CompressionFormat::Xz.extension());
        let nar_url = format!("{}/{}", base_url.trim_end_matches('/'), nar_name);
        let nar_bytes = fs::read(nar_file)?;
        Self::put_with_retry(
            &client,
            &nar_url,
            "application/x-nix-nar",
            &nar_bytes,
            "NAR",
        )?;

        let narinfo_url = format!("{}/{}.narinfo", base_url.trim_end_matches('/'), path.hash());
        Self::put_with_retry(
            &client,
            &narinfo_url,
            "text/x-nix-narinfo",
            narinfo_text.as_bytes(),
            "narinfo",
        )?;

        Ok(())
    }

    fn put_with_retry(
        client: &Client,
        url: &str,
        content_type: &str,
        body: &[u8],
        label: &str,
    ) -> Result<(), CacheError> {
        for attempt in 0..REMOTE_RETRY_ATTEMPTS {
            match client
                .put(url)
                .header("content-type", content_type)
                .body(body.to_vec())
                .send()
            {
                Ok(resp) if resp.status().is_success() => return Ok(()),
                Ok(resp) => {
                    let status = resp.status();
                    if should_retry_http_status(status) && attempt + 1 < REMOTE_RETRY_ATTEMPTS {
                        std::thread::sleep(remote_retry_delay(attempt));
                        continue;
                    }
                    return Err(CacheError::Fetch(format!(
                        "remote cache rejected {} upload: {} {}",
                        label, url, status
                    )));
                }
                Err(err) => {
                    if should_retry_reqwest_error(&err) && attempt + 1 < REMOTE_RETRY_ATTEMPTS {
                        std::thread::sleep(remote_retry_delay(attempt));
                        continue;
                    }
                    return Err(CacheError::Fetch(format!(
                        "failed to upload {}: {}",
                        label, err
                    )));
                }
            }
        }

        Err(CacheError::Fetch(format!(
            "failed to upload {} after {} attempts",
            label, REMOTE_RETRY_ATTEMPTS
        )))
    }

    /// Create a NAR archive of a store path.
    /// 创建存储路径的 NAR 归档。
    fn create_nar(&self, path: &StorePath) -> Result<(PathBuf, u64, Hash), CacheError> {
        let store_path = self.store.to_path(path);
        let nar_file = self.cache_dir.join(format!("{}.nar.xz", path.hash()));

        // Create NAR archive using our implementation
        // 使用我们的实现创建 NAR 归档
        let nar_data = nar::create_nar(&store_path)?;
        let nar_size = nar_data.len() as u64;
        let nar_hash = Hash::of(&nar_data);

        // Compress with xz
        // 使用 xz 压缩
        let compressed = self.compress_nar(&nar_data, CompressionFormat::Xz)?;

        // Write to file
        // 写入文件
        fs::write(&nar_file, compressed)?;

        Ok((nar_file, nar_size, nar_hash))
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
    use crate::PathInfo;
    use ed25519_dalek::{Signer, SigningKey};
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    fn deterministic_signing_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn cache_public_key(signing_key: &SigningKey) -> String {
        format!(
            "ed25519:{}",
            BASE64_STANDARD.encode(signing_key.verifying_key().to_bytes())
        )
    }

    fn cache_private_key(signing_key: &SigningKey) -> String {
        format!("ed25519:{}", BASE64_STANDARD.encode(signing_key.to_bytes()))
    }

    fn unsigned_narinfo(path: &StorePath) -> NarInfo {
        NarInfo {
            store_path: path.clone(),
            url: format!("{}.nar.xz", path.hash()),
            compression: CompressionFormat::Xz,
            file_size: 42,
            nar_size: Some(100),
            references: Vec::new(),
            nar_hash: None,
            file_hash: None,
            signature: None,
        }
    }

    fn signed_narinfo_text(path: &StorePath, signing_key: &SigningKey) -> String {
        let mut narinfo = unsigned_narinfo(path);
        let payload = narinfo.to_unsigned_text();
        let signature = signing_key.sign(payload.as_bytes());
        narinfo.signature = Some(format!(
            "ed25519:{}",
            BASE64_STANDARD.encode(signature.to_bytes())
        ));
        narinfo.to_text()
    }

    fn stage_store_file(
        store: &Store,
        temp_root: &std::path::Path,
        source_name: &str,
        content: &[u8],
        store_name: &str,
    ) -> StorePath {
        let source = temp_root.join(source_name);
        fs::write(&source, content).unwrap();
        let nar_hash = nar::hash_path(&source).unwrap();
        let store_path = StorePath::new(nar_hash, store_name.to_string());
        let upload_path = store.to_path(&store_path);
        if let Some(parent) = upload_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::copy(&source, &upload_path).unwrap();
        store_path
    }

    struct TestHttpCacheServer {
        base_url: String,
        storage: Arc<Mutex<HashMap<String, Vec<u8>>>>,
        fail_puts: Arc<Mutex<HashMap<String, usize>>>,
        fail_gets: Arc<Mutex<HashMap<String, usize>>>,
        request_counts: Arc<Mutex<HashMap<String, usize>>>,
        stop: Arc<AtomicBool>,
        handle: Option<JoinHandle<()>>,
    }

    impl TestHttpCacheServer {
        fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let addr = listener.local_addr().unwrap();

            let storage = Arc::new(Mutex::new(HashMap::new()));
            let fail_puts = Arc::new(Mutex::new(HashMap::new()));
            let fail_gets = Arc::new(Mutex::new(HashMap::new()));
            let request_counts = Arc::new(Mutex::new(HashMap::new()));
            let stop = Arc::new(AtomicBool::new(false));

            let storage_worker = Arc::clone(&storage);
            let fail_puts_worker = Arc::clone(&fail_puts);
            let fail_gets_worker = Arc::clone(&fail_gets);
            let request_counts_worker = Arc::clone(&request_counts);
            let stop_worker = Arc::clone(&stop);
            let handle = thread::spawn(move || {
                while !stop_worker.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let _ = handle_http_connection(
                                &mut stream,
                                &storage_worker,
                                &fail_puts_worker,
                                &fail_gets_worker,
                                &request_counts_worker,
                            );
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(_) => break,
                    }
                }
            });

            Self {
                base_url: format!("http://{}", addr),
                storage,
                fail_puts,
                fail_gets,
                request_counts,
                stop,
                handle: Some(handle),
            }
        }

        fn read_path(&self, path: &str) -> Option<Vec<u8>> {
            self.storage.lock().unwrap().get(path).cloned()
        }

        fn fail_next_put(&self, path: &str, count: usize) {
            self.fail_puts
                .lock()
                .unwrap()
                .insert(path.to_string(), count);
        }

        fn fail_next_get(&self, path: &str, count: usize) {
            self.fail_gets
                .lock()
                .unwrap()
                .insert(path.to_string(), count);
        }

        fn request_count(&self, method: &str, path: &str) -> usize {
            let key = format!("{} {}", method, path);
            self.request_counts
                .lock()
                .unwrap()
                .get(&key)
                .copied()
                .unwrap_or(0)
        }

        fn write_path(&self, path: &str, content: &[u8]) {
            self.storage
                .lock()
                .unwrap()
                .insert(path.to_string(), content.to_vec());
        }
    }

    impl Drop for TestHttpCacheServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    fn find_header_end(buf: &[u8]) -> Option<usize> {
        buf.windows(4).position(|window| window == b"\r\n\r\n")
    }

    fn parse_content_length(headers: &str) -> usize {
        headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.trim().eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    fn write_http_response(
        stream: &mut std::net::TcpStream,
        status: &str,
        content_type: &str,
        body: &[u8],
    ) -> std::io::Result<()> {
        let headers = format!(
            "HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n",
            status,
            body.len(),
            content_type
        );
        stream.write_all(headers.as_bytes())?;
        stream.write_all(body)?;
        stream.flush()?;
        Ok(())
    }

    fn handle_http_connection(
        stream: &mut std::net::TcpStream,
        storage: &Arc<Mutex<HashMap<String, Vec<u8>>>>,
        fail_puts: &Arc<Mutex<HashMap<String, usize>>>,
        fail_gets: &Arc<Mutex<HashMap<String, usize>>>,
        request_counts: &Arc<Mutex<HashMap<String, usize>>>,
    ) -> std::io::Result<()> {
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;

        let mut request = Vec::new();
        let mut buf = [0u8; 1024];
        let mut header_end = None;

        while header_end.is_none() {
            let n = stream.read(&mut buf)?;
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);
            header_end = find_header_end(&request);
        }

        let Some(header_end_idx) = header_end else {
            return write_http_response(stream, "400 Bad Request", "text/plain", b"bad request");
        };
        let body_start = header_end_idx + 4;
        let header_text = String::from_utf8_lossy(&request[..header_end_idx]);
        let mut header_lines = header_text.lines();
        let request_line = header_lines.next().unwrap_or_default();
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or_default();
        let path = parts.next().unwrap_or("/");
        let content_length = parse_content_length(&header_text);
        let request_key = format!("{} {}", method, path);
        *request_counts
            .lock()
            .unwrap()
            .entry(request_key)
            .or_insert(0) += 1;

        let mut body = request[body_start..].to_vec();
        while body.len() < content_length {
            let n = stream.read(&mut buf)?;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&buf[..n]);
        }
        if body.len() > content_length {
            body.truncate(content_length);
        }

        match method {
            "PUT" => {
                if let Some(remaining) = fail_puts.lock().unwrap().get_mut(path)
                    && *remaining > 0
                {
                    *remaining -= 1;
                    return write_http_response(
                        stream,
                        "503 Service Unavailable",
                        "text/plain",
                        b"retry later",
                    );
                }
                storage.lock().unwrap().insert(path.to_string(), body);
                write_http_response(stream, "200 OK", "text/plain", b"ok")
            }
            "GET" => {
                if let Some(remaining) = fail_gets.lock().unwrap().get_mut(path)
                    && *remaining > 0
                {
                    *remaining -= 1;
                    return write_http_response(
                        stream,
                        "503 Service Unavailable",
                        "text/plain",
                        b"retry later",
                    );
                }
                let payload = storage.lock().unwrap().get(path).cloned();
                match payload {
                    Some(payload) => {
                        let content_type = if path.ends_with(".narinfo") {
                            "text/x-nix-narinfo"
                        } else {
                            "application/octet-stream"
                        };
                        write_http_response(stream, "200 OK", content_type, &payload)
                    }
                    None => write_http_response(stream, "404 Not Found", "text/plain", b"missing"),
                }
            }
            _ => write_http_response(
                stream,
                "405 Method Not Allowed",
                "text/plain",
                b"method not allowed",
            ),
        }
    }

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

    #[test]
    fn test_parse_narinfo_remote_resolves_relative_url() {
        let temp = tempfile::TempDir::new().unwrap();
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let cache = BinaryCache::new(store).unwrap();

        let path = StorePath::new(Hash::of(b"narinfo"), "pkg-1.0".to_string());
        let content = format!(
            "StorePath: {}\nURL: nar/{}.nar.xz\nCompression: xz\nFileSize: 42\nNarSize: 100\n",
            path.display_name(),
            path.hash()
        );

        let parsed = cache
            .parse_narinfo(
                &content,
                &path,
                NarInfoSource::Remote("https://cache.example"),
                None,
            )
            .unwrap();
        let expected = format!("https://cache.example/nar/{}.nar.xz", path.hash());

        assert_eq!(parsed.url.as_deref(), Some(expected.as_str()));
        assert_eq!(parsed.size, 100);
        assert_eq!(parsed.compression, CompressionFormat::Xz);
    }

    #[test]
    fn test_parse_cache_hash_accepts_prefixes_and_raw() {
        let hash = Hash::of(b"payload");
        let raw = hash.to_hex();

        let parsed_raw = parse_cache_hash(Some(&raw)).unwrap().unwrap();
        assert_eq!(parsed_raw, hash);

        let parsed_colon = parse_cache_hash(Some(&format!("blake3:{}", raw)))
            .unwrap()
            .unwrap();
        assert_eq!(parsed_colon, hash);

        let parsed_dash = parse_cache_hash(Some(&format!("blake3-{}", raw)))
            .unwrap()
            .unwrap();
        assert_eq!(parsed_dash, hash);
    }

    #[test]
    fn test_parse_cache_hash_rejects_invalid_format() {
        let err = parse_cache_hash(Some("blake3:not-a-hex")).unwrap_err();
        assert!(matches!(err, CacheError::InvalidManifest(_)));
    }

    #[test]
    fn test_parse_narinfo_rejects_store_path_mismatch() {
        let temp = tempfile::TempDir::new().unwrap();
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let cache = BinaryCache::new(store).unwrap();

        let expected = StorePath::new(Hash::of(b"expected"), "pkg-1.0".to_string());
        let other = StorePath::new(Hash::of(b"other"), "pkg-1.0".to_string());
        let content = format!(
            "StorePath: {}\nURL: {}.nar.xz\nCompression: xz\nFileSize: 42\n",
            other.display_name(),
            other.hash()
        );

        let err = cache
            .parse_narinfo(
                &content,
                &expected,
                NarInfoSource::Remote("https://cache.example"),
                None,
            )
            .unwrap_err();

        assert!(matches!(err, CacheError::InvalidManifest(_)));
    }

    #[test]
    fn test_parse_narinfo_accepts_valid_signature_when_key_configured() {
        let temp = tempfile::TempDir::new().unwrap();
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let cache = BinaryCache::new(store).unwrap();

        let signing_key = deterministic_signing_key(7);
        let path = StorePath::new(Hash::of(b"signed"), "pkg-1.0".to_string());
        let content = signed_narinfo_text(&path, &signing_key);
        let public_key = cache_public_key(&signing_key);

        let parsed = cache
            .parse_narinfo(
                &content,
                &path,
                NarInfoSource::Remote("https://cache.example"),
                Some(public_key.as_str()),
            )
            .unwrap();

        assert_eq!(parsed.path, path);
    }

    #[test]
    fn test_parse_narinfo_rejects_invalid_signature_when_key_configured() {
        let temp = tempfile::TempDir::new().unwrap();
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let cache = BinaryCache::new(store).unwrap();

        let signing_key = deterministic_signing_key(7);
        let wrong_key = deterministic_signing_key(9);
        let path = StorePath::new(Hash::of(b"signed-invalid"), "pkg-1.0".to_string());
        let content = signed_narinfo_text(&path, &signing_key);
        let public_key = cache_public_key(&wrong_key);

        let err = cache
            .parse_narinfo(
                &content,
                &path,
                NarInfoSource::Remote("https://cache.example"),
                Some(public_key.as_str()),
            )
            .unwrap_err();

        assert!(matches!(err, CacheError::Signature(_)));
    }

    #[test]
    fn test_parse_narinfo_rejects_missing_signature_when_key_configured() {
        let temp = tempfile::TempDir::new().unwrap();
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let cache = BinaryCache::new(store).unwrap();

        let signing_key = deterministic_signing_key(7);
        let public_key = cache_public_key(&signing_key);
        let path = StorePath::new(Hash::of(b"signed-missing"), "pkg-1.0".to_string());
        let content = unsigned_narinfo(&path).to_unsigned_text();

        let err = cache
            .parse_narinfo(
                &content,
                &path,
                NarInfoSource::Remote("https://cache.example"),
                Some(public_key.as_str()),
            )
            .unwrap_err();

        assert!(matches!(err, CacheError::Signature(_)));
    }

    #[test]
    fn test_push_local_cache_signs_narinfo_and_query_verifies_signature() {
        let temp = tempfile::TempDir::new().unwrap();
        let local_cache = temp.path().join("signed-cache");
        let store_root = temp.path().join("store");
        let store = Store::open_at(store_root.clone()).unwrap();
        let store_path = store.add_content(b"payload-signed", "pkg-1.0").unwrap();

        let signing_key = deterministic_signing_key(11);
        let private_key = cache_private_key(&signing_key);
        let public_key = cache_public_key(&signing_key);

        let mut upload_cache = BinaryCache::new(store).unwrap();
        upload_cache.add_cache(CacheConfig {
            name: "signed-local-upload".to_string(),
            local_dir: Some(local_cache.clone()),
            private_key: Some(private_key),
            upload: true,
            ..Default::default()
        });
        upload_cache.push(&store_path).unwrap();

        let narinfo_path = local_cache.join(format!("{}.narinfo", store_path.hash()));
        let narinfo_content = fs::read_to_string(&narinfo_path).unwrap();
        assert!(narinfo_content.contains("Sig: ed25519:"));

        let verify_store = Store::open_at(store_root).unwrap();
        let mut verify_cache = BinaryCache::new(verify_store).unwrap();
        verify_cache.add_cache(CacheConfig {
            name: "signed-local-verify".to_string(),
            local_dir: Some(local_cache),
            public_key: Some(public_key),
            ..Default::default()
        });

        let queried = verify_cache
            .query(&store_path)
            .unwrap()
            .expect("signed cache query should succeed");
        assert_eq!(queried.path, store_path);
    }

    #[test]
    fn test_push_rejects_public_key_without_private_key() {
        let temp = tempfile::TempDir::new().unwrap();
        let local_cache = temp.path().join("signed-cache");
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let store_path = store.add_content(b"payload-signed", "pkg-1.0").unwrap();

        let signing_key = deterministic_signing_key(11);
        let mut cache = BinaryCache::new(store).unwrap();
        cache.add_cache(CacheConfig {
            name: "signed-local-upload".to_string(),
            local_dir: Some(local_cache),
            public_key: Some(cache_public_key(&signing_key)),
            upload: true,
            ..Default::default()
        });

        let err = cache.push(&store_path).unwrap_err();
        assert!(matches!(err, CacheError::Signature(_)));
    }

    #[test]
    fn test_remote_cache_signed_roundtrip_query_and_fetch() {
        let temp = tempfile::TempDir::new().unwrap();
        let server = TestHttpCacheServer::start();
        let signing_key = deterministic_signing_key(21);
        let private_key = cache_private_key(&signing_key);
        let public_key = cache_public_key(&signing_key);

        let upload_store = Store::open_at(temp.path().join("upload-store")).unwrap();
        let source = temp.path().join("remote-source.txt");
        fs::write(&source, b"remote-cache-payload").unwrap();
        let nar_hash = nar::hash_path(&source).unwrap();
        let store_path = StorePath::new(nar_hash, "pkg-1.0".to_string());
        let upload_path = upload_store.to_path(&store_path);
        if let Some(parent) = upload_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::copy(&source, &upload_path).unwrap();

        let mut upload_cache = BinaryCache::new(upload_store).unwrap();
        upload_cache.add_cache(CacheConfig {
            name: "remote-upload".to_string(),
            url: Some(server.base_url.clone()),
            private_key: Some(private_key),
            upload: true,
            ..Default::default()
        });
        upload_cache.push(&store_path).unwrap();

        let narinfo_path = format!("/{}.narinfo", store_path.hash());
        let uploaded_narinfo = String::from_utf8(server.read_path(&narinfo_path).unwrap()).unwrap();
        assert!(uploaded_narinfo.contains("Sig: ed25519:"));

        let fetch_root = temp.path().join("fetch-store");
        let mut fetch_cache =
            BinaryCache::new(Store::open_at(fetch_root.clone()).unwrap()).unwrap();
        fetch_cache.add_cache(CacheConfig {
            name: "remote-read".to_string(),
            url: Some(server.base_url.clone()),
            public_key: Some(public_key),
            ..Default::default()
        });

        let cached = fetch_cache
            .query(&store_path)
            .unwrap()
            .expect("remote cache query should return narinfo");
        fetch_cache.fetch(&cached).unwrap();

        let fetched_store = Store::open_at(fetch_root).unwrap();
        assert!(fetched_store.path_exists(&store_path));
        let fetched_path = fetched_store.to_path(&store_path);
        let content = fs::read(fetched_path).unwrap();
        assert_eq!(content, b"remote-cache-payload");
    }

    #[test]
    fn test_fetch_remote_cache_recursively_fetches_references() {
        let temp = tempfile::TempDir::new().unwrap();
        let server = TestHttpCacheServer::start();

        let upload_store = Store::open_at(temp.path().join("upload-store")).unwrap();
        let dependency = stage_store_file(
            &upload_store,
            temp.path(),
            "dep-source.txt",
            b"dependency-payload",
            "dep-1.0",
        );
        let root_payload = format!("root-uses-{}", dependency.display_name());
        let root = stage_store_file(
            &upload_store,
            temp.path(),
            "root-source.txt",
            root_payload.as_bytes(),
            "root-1.0",
        );

        let mut upload_cache = BinaryCache::new(upload_store).unwrap();
        upload_cache.add_cache(CacheConfig {
            name: "remote-upload".to_string(),
            url: Some(server.base_url.clone()),
            upload: true,
            ..Default::default()
        });
        upload_cache.push(&dependency).unwrap();
        upload_cache.push(&root).unwrap();

        let root_narinfo_path = format!("/{}.narinfo", root.hash());
        let root_narinfo =
            String::from_utf8(server.read_path(&root_narinfo_path).unwrap()).unwrap();
        let root_narinfo = format!(
            "{}References: {}\n",
            root_narinfo,
            dependency.display_name()
        );
        server.write_path(&root_narinfo_path, root_narinfo.as_bytes());

        let fetch_root = temp.path().join("fetch-store");
        let mut fetch_cache =
            BinaryCache::new(Store::open_at(fetch_root.clone()).unwrap()).unwrap();
        fetch_cache.add_cache(CacheConfig {
            name: "remote-read".to_string(),
            url: Some(server.base_url.clone()),
            ..Default::default()
        });

        let cached = fetch_cache
            .query(&root)
            .unwrap()
            .expect("root path should be available");
        fetch_cache.fetch(&cached).unwrap();

        let fetched_store = Store::open_at(fetch_root).unwrap();
        assert!(fetched_store.path_exists(&root));
        assert!(fetched_store.path_exists(&dependency));

        let mut db = Database::open(fetched_store.root().to_path_buf()).unwrap();
        let root_info = db
            .query(&root)
            .unwrap()
            .expect("root metadata should be registered");
        let dep_info = db
            .query(&dependency)
            .unwrap()
            .expect("dependency metadata should be registered");
        assert!(root_info.references.contains(&dependency));
        assert!(dep_info.references.is_empty());
    }

    #[test]
    fn test_fetch_existing_path_registers_metadata_without_download() {
        let temp = tempfile::TempDir::new().unwrap();
        let store_root = temp.path().join("store");
        let store = Store::open_at(store_root.clone()).unwrap();
        let dependency = store
            .add_content(b"dependency-existing", "dep-1.0")
            .unwrap();
        let root = store.add_content(b"root-existing", "root-1.0").unwrap();

        let cached = CachedPath {
            path: root.clone(),
            derivation: placeholder_derivation("root-1.0"),
            references: vec![dependency.clone()],
            size: 777,
            compression: CompressionFormat::Xz,
            url: None,
            file_hash: None,
            nar_hash: None,
        };

        let mut cache = BinaryCache::new(Store::open_at(store_root.clone()).unwrap()).unwrap();
        cache.fetch(&cached).unwrap();

        let mut db = Database::open(store_root).unwrap();
        let info = db
            .query(&root)
            .unwrap()
            .expect("existing path metadata should be registered");
        assert_eq!(info.nar_hash, *root.hash());
        assert_eq!(info.nar_size, 777);
        assert!(info.references.contains(&dependency));
    }

    #[test]
    fn test_fetch_remote_cache_fails_when_reference_is_missing() {
        let temp = tempfile::TempDir::new().unwrap();
        let server = TestHttpCacheServer::start();

        let upload_store = Store::open_at(temp.path().join("upload-store")).unwrap();
        let root = stage_store_file(
            &upload_store,
            temp.path(),
            "root-missing-ref-source.txt",
            b"root-with-missing-ref",
            "root-1.0",
        );
        let missing_reference = StorePath::new(Hash::of(b"missing-reference"), "dep-1.0".into());

        let mut upload_cache = BinaryCache::new(upload_store).unwrap();
        upload_cache.add_cache(CacheConfig {
            name: "remote-upload".to_string(),
            url: Some(server.base_url.clone()),
            upload: true,
            ..Default::default()
        });
        upload_cache.push(&root).unwrap();

        let root_narinfo_path = format!("/{}.narinfo", root.hash());
        let root_narinfo =
            String::from_utf8(server.read_path(&root_narinfo_path).unwrap()).unwrap();
        let root_narinfo = format!(
            "{}References: {}\n",
            root_narinfo,
            missing_reference.display_name()
        );
        server.write_path(&root_narinfo_path, root_narinfo.as_bytes());

        let fetch_root = temp.path().join("fetch-store");
        let mut fetch_cache =
            BinaryCache::new(Store::open_at(fetch_root.clone()).unwrap()).unwrap();
        fetch_cache.add_cache(CacheConfig {
            name: "remote-read".to_string(),
            url: Some(server.base_url.clone()),
            ..Default::default()
        });

        let cached = fetch_cache
            .query(&root)
            .unwrap()
            .expect("root path should be available");
        let err = fetch_cache.fetch(&cached).unwrap_err();
        assert!(matches!(err, CacheError::NotFound(_)));
        let fetched_store = Store::open_at(fetch_root).unwrap();
        assert!(!fetched_store.path_exists(&root));
    }

    #[test]
    fn test_push_remote_cache_retries_transient_put_failure() {
        let temp = tempfile::TempDir::new().unwrap();
        let server = TestHttpCacheServer::start();

        let upload_store = Store::open_at(temp.path().join("upload-store")).unwrap();
        let source = temp.path().join("remote-retry-source.txt");
        fs::write(&source, b"remote-cache-retry").unwrap();
        let nar_hash = nar::hash_path(&source).unwrap();
        let store_path = StorePath::new(nar_hash, "pkg-1.0".to_string());
        let upload_path = upload_store.to_path(&store_path);
        if let Some(parent) = upload_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::copy(&source, &upload_path).unwrap();

        let narinfo_path = format!("/{}.narinfo", store_path.hash());
        server.fail_next_put(&narinfo_path, 1);

        let mut upload_cache = BinaryCache::new(upload_store).unwrap();
        upload_cache.add_cache(CacheConfig {
            name: "remote-upload-retry".to_string(),
            url: Some(server.base_url.clone()),
            upload: true,
            ..Default::default()
        });

        upload_cache.push(&store_path).unwrap();
        assert!(server.read_path(&narinfo_path).is_some());
    }

    #[test]
    fn test_query_remote_cache_retries_transient_get_failure() {
        let temp = tempfile::TempDir::new().unwrap();
        let server = TestHttpCacheServer::start();
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let mut cache = BinaryCache::new(store).unwrap();

        let store_path = StorePath::new(Hash::of(b"remote-get-retry"), "pkg-1.0".to_string());
        let narinfo_path = format!("/{}.narinfo", store_path.hash());
        let narinfo = unsigned_narinfo(&store_path).to_text();
        server.write_path(&narinfo_path, narinfo.as_bytes());
        server.fail_next_get(&narinfo_path, 1);

        cache.add_cache(CacheConfig {
            name: "remote-query-retry".to_string(),
            url: Some(server.base_url.clone()),
            ..Default::default()
        });

        let cached = cache
            .query(&store_path)
            .unwrap()
            .expect("query should recover after transient GET failure");
        assert_eq!(cached.path, store_path);
        assert_eq!(server.request_count("GET", &narinfo_path), 2);
    }

    #[test]
    fn test_query_remote_cache_does_not_retry_not_found() {
        let temp = tempfile::TempDir::new().unwrap();
        let server = TestHttpCacheServer::start();
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let mut cache = BinaryCache::new(store).unwrap();

        let store_path = StorePath::new(Hash::of(b"remote-not-found"), "pkg-1.0".to_string());
        let narinfo_path = format!("/{}.narinfo", store_path.hash());

        cache.add_cache(CacheConfig {
            name: "remote-query-404".to_string(),
            url: Some(server.base_url.clone()),
            ..Default::default()
        });

        assert!(cache.query(&store_path).unwrap().is_none());
        assert_eq!(server.request_count("GET", &narinfo_path), 1);
    }

    #[test]
    fn test_query_uses_lower_priority_cache_when_higher_remote_unavailable() {
        let temp = tempfile::TempDir::new().unwrap();
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let mut cache = BinaryCache::new(store).unwrap();
        let local_cache = temp.path().join("local-cache");
        fs::create_dir_all(&local_cache).unwrap();

        let down_server = TestHttpCacheServer::start();
        let down_url = down_server.base_url.clone();
        drop(down_server);

        let store_path = StorePath::new(Hash::of(b"fallback-query"), "pkg-1.0".to_string());
        let narinfo_path = local_cache.join(format!("{}.narinfo", store_path.hash()));
        fs::write(&narinfo_path, unsigned_narinfo(&store_path).to_text()).unwrap();

        cache.add_cache(CacheConfig {
            name: "high-remote-down".to_string(),
            url: Some(down_url),
            priority: 100,
            ..Default::default()
        });
        cache.add_cache(CacheConfig {
            name: "low-local".to_string(),
            local_dir: Some(local_cache),
            priority: 10,
            ..Default::default()
        });

        let cached = cache
            .query(&store_path)
            .unwrap()
            .expect("lower-priority cache should still satisfy query");
        assert_eq!(cached.path, store_path);
    }

    #[test]
    fn test_query_returns_fetch_error_when_only_remote_cache_is_unavailable() {
        let temp = tempfile::TempDir::new().unwrap();
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let mut cache = BinaryCache::new(store).unwrap();

        let down_server = TestHttpCacheServer::start();
        let down_url = down_server.base_url.clone();
        drop(down_server);

        cache.add_cache(CacheConfig {
            name: "remote-down".to_string(),
            url: Some(down_url),
            ..Default::default()
        });

        let store_path = StorePath::new(Hash::of(b"remote-down"), "pkg-1.0".to_string());
        let err = cache.query(&store_path).unwrap_err();
        assert!(matches!(err, CacheError::Fetch(_)));
    }

    #[test]
    fn test_push_local_cache_records_references_in_narinfo() {
        let temp = tempfile::TempDir::new().unwrap();
        let local_cache = temp.path().join("local-cache");
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let dependency = stage_store_file(
            &store,
            temp.path(),
            "dep-reference-source.txt",
            b"dependency-payload",
            "dep-1.0",
        );
        let root_payload = format!("depends-on:{}\n", dependency.display_name());
        let root = stage_store_file(
            &store,
            temp.path(),
            "root-reference-source.txt",
            root_payload.as_bytes(),
            "root-1.0",
        );

        let mut cache = BinaryCache::new(store).unwrap();
        cache.add_cache(CacheConfig {
            name: "local".to_string(),
            local_dir: Some(local_cache.clone()),
            upload: true,
            ..Default::default()
        });

        cache.push(&root).unwrap();

        let narinfo_path = local_cache.join(format!("{}.narinfo", root.hash()));
        let narinfo = fs::read_to_string(&narinfo_path).unwrap();
        assert!(narinfo.contains(&format!("References: {}", dependency.display_name())));

        let queried = cache.query(&root).unwrap().expect("query should succeed");
        assert_eq!(queried.references, vec![dependency]);
    }

    #[test]
    fn test_push_local_cache_prefers_database_references_when_present() {
        let temp = tempfile::TempDir::new().unwrap();
        let local_cache = temp.path().join("local-cache");
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let dependency = stage_store_file(
            &store,
            temp.path(),
            "dep-db-reference-source.txt",
            b"dependency-payload",
            "dep-1.0",
        );
        let root = stage_store_file(
            &store,
            temp.path(),
            "root-db-reference-source.txt",
            b"root-without-inline-reference",
            "root-1.0",
        );

        let mut db = Database::open(store.root().to_path_buf()).unwrap();
        let mut info = PathInfo::new(root.clone(), *root.hash(), 0);
        info.add_reference(dependency.clone());
        db.register(info).unwrap();

        let mut cache = BinaryCache::new(store).unwrap();
        cache.add_cache(CacheConfig {
            name: "local".to_string(),
            local_dir: Some(local_cache.clone()),
            upload: true,
            ..Default::default()
        });

        cache.push(&root).unwrap();

        let narinfo_path = local_cache.join(format!("{}.narinfo", root.hash()));
        let narinfo = fs::read_to_string(&narinfo_path).unwrap();
        assert!(narinfo.contains(&format!("References: {}", dependency.display_name())));

        let queried = cache.query(&root).unwrap().expect("query should succeed");
        assert_eq!(queried.references, vec![dependency]);
    }

    #[test]
    fn test_push_local_cache_ignores_nonexistent_reference_tokens() {
        let temp = tempfile::TempDir::new().unwrap();
        let local_cache = temp.path().join("local-cache");
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let fake_reference = format!("{}-ghost-1.0", "a".repeat(64));
        let root_payload = format!("not-a-real-ref:{}\n", fake_reference);
        let root = stage_store_file(
            &store,
            temp.path(),
            "root-fake-reference-source.txt",
            root_payload.as_bytes(),
            "root-1.0",
        );

        let mut cache = BinaryCache::new(store).unwrap();
        cache.add_cache(CacheConfig {
            name: "local".to_string(),
            local_dir: Some(local_cache.clone()),
            upload: true,
            ..Default::default()
        });

        cache.push(&root).unwrap();

        let narinfo_path = local_cache.join(format!("{}.narinfo", root.hash()));
        let narinfo = fs::read_to_string(&narinfo_path).unwrap();
        assert!(!narinfo.contains("References:"));

        let queried = cache.query(&root).unwrap().expect("query should succeed");
        assert!(queried.references.is_empty());
    }

    #[test]
    fn test_push_local_cache_writes_narinfo_and_query_uses_it() {
        let temp = tempfile::TempDir::new().unwrap();
        let local_cache = temp.path().join("local-cache");
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let store_path = store.add_content(b"payload", "pkg-1.0").unwrap();

        let mut cache = BinaryCache::new(store).unwrap();
        cache.add_cache(CacheConfig {
            name: "local".to_string(),
            local_dir: Some(local_cache.clone()),
            upload: true,
            ..Default::default()
        });

        cache.push(&store_path).unwrap();

        let narinfo_path = local_cache.join(format!("{}.narinfo", store_path.hash()));
        assert!(narinfo_path.exists());

        let narinfo = std::fs::read_to_string(&narinfo_path).unwrap();
        assert!(narinfo.contains(&format!("StorePath: {}", store_path.display_name())));
        assert!(narinfo.contains("Compression: xz"));

        let queried = cache
            .query(&store_path)
            .unwrap()
            .expect("query should succeed");
        assert_eq!(queried.path, store_path);
        assert!(queried.url.is_some());
    }

    #[test]
    fn test_fetch_rejects_file_hash_mismatch() {
        let temp = tempfile::TempDir::new().unwrap();
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let mut cache = BinaryCache::new(store).unwrap();

        let source = temp.path().join("source.txt");
        fs::write(&source, b"hello-cache").unwrap();

        let nar_data = nar::create_nar(&source).unwrap();
        let nar_hash = Hash::of(&nar_data);
        let mut compressed = Vec::new();
        lzma_rs::xz_compress(&mut std::io::Cursor::new(&nar_data), &mut compressed).unwrap();
        let nar_file = temp.path().join("payload.nar.xz");
        fs::write(&nar_file, &compressed).unwrap();

        let cached = CachedPath {
            path: StorePath::new(nar_hash, "pkg-1.0".to_string()),
            derivation: placeholder_derivation("pkg-1.0"),
            references: Vec::new(),
            size: nar_data.len() as u64,
            compression: CompressionFormat::Xz,
            url: Some(nar_file.to_string_lossy().to_string()),
            file_hash: Some(format_hash(&Hash::of(b"wrong-file-hash"))),
            nar_hash: Some(format_hash(&nar_hash)),
        };

        let err = cache.fetch(&cached).unwrap_err();
        assert!(matches!(
            err,
            CacheError::HashMismatch {
                kind: "nar-compressed",
                ..
            }
        ));
    }

    #[test]
    fn test_fetch_rejects_nar_hash_mismatch() {
        let temp = tempfile::TempDir::new().unwrap();
        let store = Store::open_at(temp.path().join("store")).unwrap();
        let mut cache = BinaryCache::new(store).unwrap();

        let source = temp.path().join("source.txt");
        fs::write(&source, b"hello-nar").unwrap();

        let nar_data = nar::create_nar(&source).unwrap();
        let nar_hash = Hash::of(&nar_data);
        let mut compressed = Vec::new();
        lzma_rs::xz_compress(&mut std::io::Cursor::new(&nar_data), &mut compressed).unwrap();
        let nar_file = temp.path().join("payload.nar.xz");
        fs::write(&nar_file, &compressed).unwrap();

        let cached = CachedPath {
            path: StorePath::new(nar_hash, "pkg-1.0".to_string()),
            derivation: placeholder_derivation("pkg-1.0"),
            references: Vec::new(),
            size: nar_data.len() as u64,
            compression: CompressionFormat::Xz,
            url: Some(nar_file.to_string_lossy().to_string()),
            file_hash: Some(format_hash(&Hash::of(&compressed))),
            nar_hash: Some(format_hash(&Hash::of(b"wrong-nar-hash"))),
        };

        let err = cache.fetch(&cached).unwrap_err();
        assert!(matches!(err, CacheError::HashMismatch { kind: "nar", .. }));
    }
}
