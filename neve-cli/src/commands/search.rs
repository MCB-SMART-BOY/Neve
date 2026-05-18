//! The `neve search` command.
//! `neve search` 命令。
//!
//! Searches for packages in the store and available package sources.
//! 在存储和可用软件包源中搜索软件包。

use crate::output;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// Search for packages matching a query.
/// 搜索匹配查询的软件包。
pub fn run(query: &str) -> Result<(), String> {
    let store_dir = get_store_dir();

    let status = output::Status::new(&format!("Searching for '{query}'"));

    let mut found = false;

    // Search in store
    // 在存储中搜索
    let store_matches = if store_dir.exists() {
        search_store(&store_dir, query)?
    } else {
        Vec::new()
    };

    // Search in package index (if available)
    // 在软件包索引中搜索（如果可用）
    let index_matches = match search_index(query) {
        Ok(matches) => matches,
        Err(e) => {
            output::warning(&format!("Package index unavailable: {}", e));
            Vec::new()
        }
    };

    status.success(Some(&format!("Search complete for '{query}'")));

    if !store_matches.is_empty() {
        output::section("Installed packages");
        let mut table = output::Table::new(vec!["Package", "Path"]);
        for (name, path) in &store_matches {
            table.add_row(vec![name, &path.display().to_string()]);
        }
        table.print();
        found = true;
    }

    if !index_matches.is_empty() {
        output::section("Available packages");
        let mut table = output::Table::new(vec!["Package", "Description"]);
        for (name, description) in &index_matches {
            table.add_row(vec![name, description]);
        }
        table.print();
        found = true;
    }

    if !found {
        output::warning(&format!("No packages found matching '{}'", query));
    }

    Ok(())
}

/// Get the store directory.
/// 获取存储目录。
fn get_store_dir() -> PathBuf {
    std::env::var("NEVE_STORE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/neve/store"))
}

/// Search for packages in the store.
/// 在存储中搜索软件包。
fn search_store(store_dir: &PathBuf, query: &str) -> Result<Vec<(String, PathBuf)>, String> {
    let mut matches = Vec::new();
    let query_lower = query.to_lowercase();

    for entry in fs::read_dir(store_dir).map_err(|e| format!("Failed to read store: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_lowercase();

        if name_str.contains(&query_lower) {
            matches.push((name.to_string_lossy().to_string(), entry.path()));
        }
    }

    // Sort by name
    // 按名称排序
    matches.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(matches)
}

/// Fetch the package index from a remote URL.
/// 从远程 URL 获取软件包索引。
fn fetch_remote_index(url: &str, query: &str) -> Result<Vec<(String, String)>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("failed to create HTTP client: {e}"))?;

    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("failed to fetch index: {e}"))?;

    let content = response
        .text()
        .map_err(|e| format!("failed to read response: {e}"))?;

    let parsed = parse_index(&content)?;
    let query_lower = query.to_lowercase();
    let mut matches: Vec<(String, String)> = parsed
        .into_iter()
        .filter(|(name, desc)| {
            name.to_lowercase().contains(&query_lower) || desc.to_lowercase().contains(&query_lower)
        })
        .collect();
    matches.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(matches)
}

/// Search the package index.
/// 搜索软件包索引。
fn search_index(query: &str) -> Result<Vec<(String, String)>, String> {
    // First try remote registry via $NEVE_REGISTRY
    // 首先尝试通过 $NEVE_REGISTRY 访问远程注册表
    if let Ok(registry_url) = std::env::var("NEVE_REGISTRY") {
        match fetch_remote_index(&registry_url, query) {
            Ok(matches) if !matches.is_empty() => return Ok(matches),
            Ok(_) => {} // empty results, fall through to local
            Err(e) => {
                output::warning(&format!("Remote registry unavailable: {e}"));
            }
        }
    }

    // Fall back to local index
    // 回退到本地索引
    let Some(index_path) = get_index_path() else {
        return Ok(Vec::new());
    };

    if !index_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&index_path)
        .map_err(|e| format!("cannot read {}: {}", index_path.display(), e))?;
    let parsed = parse_index(&content)
        .map_err(|e| format!("cannot parse {}: {}", index_path.display(), e))?;

    let query_lower = query.to_lowercase();
    let mut matches: Vec<(String, String)> = parsed
        .into_iter()
        .filter(|(name, desc)| {
            name.to_lowercase().contains(&query_lower) || desc.to_lowercase().contains(&query_lower)
        })
        .collect();
    matches.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(matches)
}

/// Package index location.
/// 软件包索引位置。
pub(crate) fn get_index_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("NEVE_PACKAGE_INDEX") {
        return Some(PathBuf::from(path));
    }

    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".neve").join("package-index.json"))
}

#[derive(Debug, Deserialize)]
struct IndexEntry {
    name: String,
    #[serde(default)]
    description: String,
}

/// Parse supported package index JSON formats.
/// 解析支持的软件包索引 JSON 格式。
pub(crate) fn parse_index(content: &str) -> Result<Vec<(String, String)>, String> {
    // Format 1: [{"name":"foo","description":"..."}]
    if let Ok(entries) = serde_json::from_str::<Vec<IndexEntry>>(content) {
        return Ok(entries
            .into_iter()
            .map(|e| (e.name, e.description))
            .collect());
    }

    // Format 2: {"foo":"desc","bar":"desc"}
    if let Ok(map) = serde_json::from_str::<std::collections::BTreeMap<String, String>>(content) {
        return Ok(map.into_iter().collect());
    }

    Err("unsupported index format (expected array or object)".to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_index;

    #[test]
    fn test_parse_index_array_format() {
        let json = r#"
        [
          {"name":"foo","description":"Foo package"},
          {"name":"bar","description":"Bar package"}
        ]
        "#;
        let parsed = parse_index(json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0], ("foo".to_string(), "Foo package".to_string()));
    }

    #[test]
    fn test_parse_index_object_format() {
        let json = r#"{"foo":"Foo package","bar":"Bar package"}"#;
        let parsed = parse_index(json).unwrap();
        assert_eq!(parsed.len(), 2);
    }
}
