//! The `neve registry update` command.
//! `neve registry update` 命令。
//!
//! Fetches the package index from a remote registry and saves it locally.
//! 从远程注册表获取软件包索引并保存到本地。

use super::search;
use crate::output;
use std::fs;

/// Update the local package index from a remote registry.
/// 从远程注册表更新本地软件包索引。
pub fn update(registry_url: Option<&str>) -> Result<(), String> {
    let url = registry_url
        .map(|s| s.to_string())
        .or_else(|| std::env::var("NEVE_REGISTRY").ok())
        .unwrap_or_else(|| "https://registry.neve.dev/packages.json".to_string());

    let status = output::Status::new(&format!("Fetching package index from {url}"));

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("failed to create HTTP client: {e}"))?;

    let response = client
        .get(&url)
        .send()
        .map_err(|e| format!("failed to fetch: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("registry returned HTTP {}", response.status()));
    }

    let content = response
        .text()
        .map_err(|e| format!("failed to read response: {e}"))?;

    // Validate it's valid JSON by parsing it
    // 通过解析来验证它是有效的 JSON
    search::parse_index(&content)?;

    // Save to local index
    // 保存到本地索引
    let index_path =
        search::get_index_path().ok_or_else(|| "cannot determine index path".to_string())?;

    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create directory: {e}"))?;
    }

    fs::write(&index_path, &content).map_err(|e| format!("failed to write index: {e}"))?;

    status.success(Some(&format!("Index saved to {}", index_path.display())));
    Ok(())
}
