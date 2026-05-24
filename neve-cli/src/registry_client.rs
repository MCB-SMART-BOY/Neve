//! Registry client for Neve v1 package registry API.
//! Neve v1 包注册表 API 客户端。

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A version entry from the registry.
/// 来自注册表的版本条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryVersion {
    pub version: String,
    #[serde(default)]
    pub nar_hash: Option<String>,
    #[serde(default)]
    pub file_hash: Option<String>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub published_at: Option<String>,
}

/// Package metadata from the registry.
/// 来自注册表的包元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPackage {
    pub name: String,
    pub versions: Vec<RegistryVersion>,
}

/// Index entry from the registry.
/// 来自注册表的索引条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndexEntry {
    pub name: String,
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(default)]
    pub description: String,
}

/// Search result from the registry.
/// 来自注册表的搜索结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySearchResponse {
    pub results: Vec<RegistryIndexEntry>,
    #[serde(default)]
    pub total: usize,
}

/// Client for the Neve v1 package registry.
/// Neve v1 包注册表客户端。
pub struct RegistryClient {
    base_url: String,
}

impl RegistryClient {
    /// Create a new registry client.
    /// 创建新的注册表客户端。
    pub fn new(base_url: String) -> Self {
        let base_url = base_url.trim_end_matches('/').to_string();
        Self { base_url }
    }

    /// Create a client from the NEVE_REGISTRY environment variable.
    /// 从 NEVE_REGISTRY 环境变量创建客户端。
    pub fn from_env() -> Option<Self> {
        std::env::var("NEVE_REGISTRY").ok().map(Self::new)
    }

    /// Get the package index (all packages with latest versions).
    /// 获取包索引（所有包及其最新版本）。
    pub fn get_index(&self) -> Result<Vec<RegistryIndexEntry>, String> {
        let url = format!("{}/v1/index.json", self.base_url);
        let body = self.get(&url)?;
        serde_json::from_str(&body).map_err(|e| format!("parse index: {e}"))
    }

    /// Search for packages by name.
    /// 按名称搜索包。
    pub fn search(&self, query: &str) -> Result<RegistrySearchResponse, String> {
        let url = format!("{}/v1/search?q={}", self.base_url, query);
        let body = self.get(&url)?;
        serde_json::from_str(&body).map_err(|e| format!("parse search: {e}"))
    }

    /// Get all versions of a package.
    /// 获取包的所有版本。
    pub fn get_package(&self, name: &str) -> Result<RegistryPackage, String> {
        let url = format!("{}/v1/packages/{name}", self.base_url);
        let body = self.get(&url)?;
        serde_json::from_str(&body).map_err(|e| format!("parse package: {e}"))
    }

    /// Get metadata for a specific package version.
    /// 获取特定包版本的元数据。
    pub fn get_version(&self, name: &str, version: &str) -> Result<RegistryVersion, String> {
        let url = format!("{}/v1/packages/{name}/{version}", self.base_url);
        let body = self.get(&url)?;
        serde_json::from_str(&body).map_err(|e| format!("parse version: {e}"))
    }

    /// Check if the registry is reachable.
    /// 检查注册表是否可达。
    pub fn ping(&self) -> Result<(), String> {
        let url = format!("{}/health", self.base_url);
        self.get(&url)?;
        Ok(())
    }

    /// Perform an HTTP GET request.
    fn get(&self, url: &str) -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("http client: {e}"))?;

        let response = client
            .get(url)
            .send()
            .map_err(|e| format!("fetch {url}: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "registry returned HTTP {} for {url}",
                response.status()
            ));
        }

        response.text().map_err(|e| format!("read response: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_client_url_trim() {
        let client = RegistryClient::new("https://example.com/".to_string());
        assert_eq!(client.base_url, "https://example.com");
    }

    #[test]
    fn test_registry_client_url_no_trim() {
        let client = RegistryClient::new("https://example.com".to_string());
        assert_eq!(client.base_url, "https://example.com");
    }
}
