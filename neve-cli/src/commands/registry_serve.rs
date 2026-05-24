//! Registry server for Neve packages (v1 API).
//! Neve 软件包注册服务器 (v1 API)。

use crate::output;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

// =============================================================================
// Registry v1 data model / 注册表 v1 数据模型
// =============================================================================

/// Index entry in the package index.
/// 包索引中的索引条目。
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct IndexEntry {
    name: String,
    versions: Vec<String>,
    description: String,
}

/// Per-version metadata for a package.
/// 包的每版本元数据。
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct VersionMetadata {
    version: String,
    #[serde(default)]
    nar_hash: Option<String>,
    #[serde(default)]
    file_hash: Option<String>,
    #[serde(default)]
    dependencies: HashMap<String, String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
}

/// Full package metadata (all versions).
/// 包的完整元数据（所有版本）。
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct PackageMetadata {
    name: String,
    versions: Vec<VersionMetadata>,
}

// =============================================================================
// In-memory registry state / 内存中的注册表状态
// =============================================================================

struct RegistryState {
    data_dir: PathBuf,
    /// Cached index for fast responses. / 缓存的索引以快速响应。
    index: Vec<IndexEntry>,
    /// Cached per-package metadata. / 缓存的每包元数据。
    packages: HashMap<String, PackageMetadata>,
}

impl RegistryState {
    fn load(data_dir: &Path) -> Self {
        let mut state = Self {
            data_dir: data_dir.to_path_buf(),
            index: Vec::new(),
            packages: HashMap::new(),
        };
        state.reload_index();
        state
    }

    /// Reload the index from disk. / 从磁盘重新加载索引。
    fn reload_index(&mut self) {
        let index_path = self.data_dir.join("v1").join("index.json");
        if let Ok(content) = fs::read_to_string(&index_path)
            && let Ok(entries) = serde_json::from_str::<Vec<IndexEntry>>(&content)
        {
            self.index = entries;
        }
    }

    /// Load a package's metadata from disk. / 从磁盘加载包的元数据。
    fn load_package(&mut self, name: &str) -> Option<&PackageMetadata> {
        if self.packages.contains_key(name) {
            return self.packages.get(name);
        }
        let pkg_path = self
            .data_dir
            .join("v1")
            .join("packages")
            .join(format!("{name}.json"));
        if let Ok(content) = fs::read_to_string(&pkg_path)
            && let Ok(meta) = serde_json::from_str::<PackageMetadata>(&content)
        {
            self.packages.insert(name.to_string(), meta);
            return self.packages.get(name);
        }
        None
    }

    /// Save a package's metadata to disk and update the index.
    /// 将包元数据保存到磁盘并更新索引。
    fn save_package(&mut self, meta: PackageMetadata) -> Result<(), String> {
        let name = meta.name.clone();
        let pkg_dir = self.data_dir.join("v1").join("packages");
        fs::create_dir_all(&pkg_dir).map_err(|e| format!("failed to create packages dir: {e}"))?;

        let pkg_path = pkg_dir.join(format!("{name}.json"));
        let content = serde_json::to_string_pretty(&meta).map_err(|e| format!("serialize: {e}"))?;
        fs::write(&pkg_path, &content).map_err(|e| format!("write: {e}"))?;

        // Update index
        let versions: Vec<String> = meta.versions.iter().map(|v| v.version.clone()).collect();
        let description = meta
            .versions
            .first()
            .map(|v| v.description.clone())
            .unwrap_or_default();

        // Update or insert in index
        if let Some(entry) = self.index.iter_mut().find(|e| e.name == name) {
            entry.versions = versions;
            entry.description = description;
        } else {
            self.index.push(IndexEntry {
                name: name.clone(),
                versions,
                description,
            });
        }

        // Save index
        self.save_index()?;

        // Update cache
        self.packages.insert(name, meta);

        Ok(())
    }

    fn save_index(&self) -> Result<(), String> {
        let index_dir = self.data_dir.join("v1");
        fs::create_dir_all(&index_dir).map_err(|e| format!("failed to create v1 dir: {e}"))?;
        let index_path = index_dir.join("index.json");
        let content =
            serde_json::to_string_pretty(&self.index).map_err(|e| format!("serialize: {e}"))?;
        fs::write(&index_path, &content).map_err(|e| format!("write: {e}"))?;
        Ok(())
    }

    /// Search packages by name substring (case-insensitive).
    /// 按名称子串搜索包（不区分大小写）。
    fn search(&self, query: &str) -> Vec<&IndexEntry> {
        let q = query.to_lowercase();
        self.index
            .iter()
            .filter(|e| e.name.to_lowercase().contains(&q))
            .collect()
    }
}

// =============================================================================
// HTTP response helpers / HTTP 响应辅助
// =============================================================================

fn json_response(body: &str, status: &str) -> String {
    let len = body.len();
    format!(
        "{status}\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{body}"
    )
}

fn ok_json(body: &str) -> String {
    json_response(body, "HTTP/1.1 200 OK")
}

fn not_found_json(body: &str) -> String {
    json_response(body, "HTTP/1.1 404 Not Found")
}

fn bad_request(body: &str) -> String {
    json_response(body, "HTTP/1.1 400 Bad Request")
}

fn method_not_allowed() -> String {
    let body = r#"{"error":"method not allowed"}"#;
    json_response(body, "HTTP/1.1 405 Method Not Allowed")
}

fn text_response(body: &str, status: &str) -> String {
    let len = body.len();
    format!("{status}\r\nContent-Type: text/plain\r\nContent-Length: {len}\r\n\r\n{body}")
}

/// Binary file response (returns headers string, body written separately).
fn binary_headers(len: usize, content_type: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {len}\r\nCache-Control: public, max-age=31536000, immutable\r\n\r\n"
    )
}

// =============================================================================
// Route handlers / 路由处理器
// =============================================================================

fn handle_index(_state: &RwLock<RegistryState>) -> String {
    let body = serde_json::json!({
        "registry": "neve",
        "version": "1.0",
        "apiVersions": ["v1"],
        "endpoints": {
            "index": "/v1/index.json",
            "search": "/v1/search?q={query}",
            "package": "/v1/packages/{name}",
            "version": "/v1/packages/{name}/{version}",
            "publish": "POST /v1/packages/{name}",
            "narinfo": "/v1/{hash}.narinfo",
            "nar": "/v1/nar/{hash}.nar"
        }
    });
    ok_json(&body.to_string())
}

fn handle_health() -> String {
    text_response("OK", "HTTP/1.1 200 OK")
}

fn handle_v1_index(state: &RwLock<RegistryState>) -> String {
    let state = state.read().unwrap();
    let body = serde_json::to_string(&state.index).unwrap_or_else(|_| "[]".to_string());
    ok_json(&body)
}

fn handle_v1_search(query: &str, state: &RwLock<RegistryState>) -> String {
    let state = state.read().unwrap();
    if query.is_empty() {
        let body = serde_json::json!({"results": [], "total": 0});
        return ok_json(&body.to_string());
    }
    let results: Vec<&IndexEntry> = state.search(query);
    let total = results.len();
    let body = serde_json::json!({
        "results": results,
        "total": total,
    });
    ok_json(&body.to_string())
}

fn handle_v1_package(name: &str, state: &RwLock<RegistryState>) -> String {
    let mut state = state.write().unwrap();
    match state.load_package(name) {
        Some(meta) => {
            let body = serde_json::to_string(meta).unwrap_or_default();
            ok_json(&body)
        }
        None => {
            let body = serde_json::json!({"error": format!("package '{name}' not found")});
            not_found_json(&body.to_string())
        }
    }
}

fn handle_v1_package_version(name: &str, version: &str, state: &RwLock<RegistryState>) -> String {
    let mut state = state.write().unwrap();
    match state.load_package(name) {
        Some(meta) => match meta.versions.iter().find(|v| v.version == version) {
            Some(ver_meta) => {
                let body = serde_json::to_string(ver_meta).unwrap_or_default();
                ok_json(&body)
            }
            None => {
                let body = serde_json::json!({
                    "error": format!("version '{version}' not found for package '{name}'")
                });
                not_found_json(&body.to_string())
            }
        },
        None => {
            let body = serde_json::json!({"error": format!("package '{name}' not found")});
            not_found_json(&body.to_string())
        }
    }
}

fn handle_v1_publish(name: &str, body: &str, state: &RwLock<RegistryState>) -> String {
    // Sanitize name
    let safe_name: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();

    if safe_name.is_empty() || safe_name != name {
        return bad_request(&serde_json::json!({"error": "invalid package name"}).to_string());
    }

    // Parse the version metadata
    let new_version: VersionMetadata = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return bad_request(
                &serde_json::json!({"error": format!("invalid request body: {e}")}).to_string(),
            );
        }
    };

    if new_version.version.is_empty() {
        return bad_request(&serde_json::json!({"error": "version is required"}).to_string());
    }

    let mut state = state.write().unwrap();

    // Load existing package or create new
    let mut pkg = state
        .load_package(&safe_name)
        .cloned()
        .unwrap_or_else(|| PackageMetadata {
            name: safe_name.clone(),
            versions: Vec::new(),
        });

    // Check for duplicate version
    if pkg
        .versions
        .iter()
        .any(|v| v.version == new_version.version)
    {
        return bad_request(
            &serde_json::json!({"error": format!("version {} already exists", new_version.version)})
                .to_string(),
        );
    }

    // Add timestamp
    let mut version = new_version;
    if version.published_at.is_none() {
        use std::time::SystemTime;
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        version.published_at = Some(format!("{ts}"));
    }

    // Insert in reverse order (newest first)
    pkg.versions.insert(0, version);
    pkg.name = safe_name.clone();

    match state.save_package(pkg) {
        Ok(()) => {
            let body = serde_json::json!({"status": "published", "name": safe_name});
            ok_json(&body.to_string())
        }
        Err(e) => {
            let body = serde_json::json!({"error": e});
            json_response(&body.to_string(), "HTTP/1.1 500 Internal Server Error")
        }
    }
}

// =============================================================================
// Legacy backward-compat handlers / 向后兼容的旧版处理器
// =============================================================================

fn handle_legacy_packages_json(data_dir: &Path) -> String {
    let index_path = data_dir.join("packages.json");
    match fs::read_to_string(&index_path) {
        Ok(content) => ok_json(&content),
        Err(_) => ok_json("[]"),
    }
}

fn handle_legacy_package(data_dir: &Path, name: &str) -> String {
    let safe_name: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();

    let pkg_path = data_dir.join(format!("packages/{}.json", safe_name));
    match fs::read_to_string(&pkg_path) {
        Ok(content) => ok_json(&content),
        Err(_) => {
            let body = serde_json::json!({"error": format!("package '{name}' not found")});
            not_found_json(&body.to_string())
        }
    }
}

// =============================================================================
// Request router / 请求路由
// =============================================================================

fn parse_query_string(path: &str) -> (&str, HashMap<String, String>) {
    let (base, qs) = path.split_once('?').unwrap_or((path, ""));
    let params: HashMap<String, String> = qs
        .split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            Some((k.to_string(), v.to_string()))
        })
        .collect();
    (base, params)
}

fn route_request(
    method: &str,
    path: &str,
    body: &str,
    data_dir: &Path,
    state: &RwLock<RegistryState>,
) -> (String, Option<Vec<u8>>) {
    let (base_path, query) = parse_query_string(path);

    // Binary cache / NAR serving
    if let Some(hash) = base_path.strip_prefix("/v1/nar/") {
        if method == "GET" && hash.ends_with(".nar") {
            let hash = hash.trim_end_matches(".nar");
            return handle_nar_download(hash, data_dir);
        }
        return (not_found_json(r#"{"error":"invalid nar path"}"#), None);
    }
    if let Some(hash) = base_path.strip_prefix("/v1/")
        && hash.ends_with(".narinfo")
        && method == "GET"
    {
        let hash = hash.trim_end_matches(".narinfo");
        return (handle_narinfo(hash, data_dir), None);
    }

    // v1 API / v1 API
    if base_path == "/v1/index.json" && method == "GET" {
        return (handle_v1_index(state), None);
    }
    if base_path == "/v1/search" && method == "GET" {
        let q = query.get("q").cloned().unwrap_or_default();
        return (handle_v1_search(&q, state), None);
    }
    if let Some(rest) = base_path.strip_prefix("/v1/packages/") {
        let segments: Vec<&str> = rest.split('/').collect();
        match segments.len() {
            1 => {
                let name = segments[0];
                if method == "GET" {
                    return (handle_v1_package(name, state), None);
                }
                if method == "POST" {
                    return (handle_v1_publish(name, body, state), None);
                }
                return (method_not_allowed(), None);
            }
            2 => {
                if method == "GET" {
                    return (
                        handle_v1_package_version(segments[0], segments[1], state),
                        None,
                    );
                }
                return (method_not_allowed(), None);
            }
            _ => {}
        }
    }

    // Legacy endpoints
    if path == "/" && method == "GET" {
        return (handle_index(state), None);
    }
    if path == "/health" && method == "GET" {
        return (handle_health(), None);
    }
    if path == "/packages.json" && method == "GET" {
        return (handle_legacy_packages_json(data_dir), None);
    }
    if let Some(name) = path.strip_prefix("/packages/") {
        if method == "GET" {
            return (handle_legacy_package(data_dir, name), None);
        }
        if method == "POST" {
            return (handle_v1_publish(name, body, state), None);
        }
        return (method_not_allowed(), None);
    }

    (not_found_json(r#"{"error":"not found"}"#), None)
}

// =============================================================================
// NAR / Binary cache handlers / NAR / 二进制缓存处理器
// =============================================================================

/// Serve a NAR archive file.
/// 提供 NAR 归档文件。
fn handle_nar_download(hash: &str, data_dir: &Path) -> (String, Option<Vec<u8>>) {
    let safe_hash: String = hash
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .take(128)
        .collect();

    if safe_hash.is_empty() {
        return (not_found_json(r#"{"error":"invalid nar hash"}"#), None);
    }

    let nar_path = data_dir.join("nar").join(format!("{safe_hash}.nar"));
    match fs::read(&nar_path) {
        Ok(data) => {
            let headers = binary_headers(data.len(), "application/x-nix-archive");
            (headers, Some(data))
        }
        Err(_) => (not_found_json(r#"{"error":"nar not found"}"#), None),
    }
}

/// Serve a narinfo metadata file.
/// 提供 narinfo 元数据文件。
fn handle_narinfo(hash: &str, data_dir: &Path) -> String {
    let safe_hash: String = hash
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();

    if safe_hash.is_empty() || safe_hash.len() > 128 {
        return not_found_json(r#"{"error":"invalid narinfo hash"}"#);
    }

    let narinfo_path = data_dir.join(format!("{safe_hash}.narinfo"));
    match fs::read_to_string(&narinfo_path) {
        Ok(content) => {
            let len = content.len();
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {len}\r\nCache-Control: public, max-age=31536000, immutable\r\n\r\n{content}"
            )
        }
        Err(_) => {
            let body = serde_json::json!({"error": "narinfo not found"});
            not_found_json(&body.to_string())
        }
    }
}

pub fn run(dir: &str, port: u16) -> Result<(), String> {
    let data_dir = PathBuf::from(dir);
    fs::create_dir_all(&data_dir)
        .map_err(|e| format!("failed to create registry directory: {e}"))?;

    // Ensure v1 subdirectories exist
    fs::create_dir_all(data_dir.join("v1").join("packages"))
        .map_err(|e| format!("failed to create v1 directories: {e}"))?;

    let state = Arc::new(RwLock::new(RegistryState::load(&data_dir)));

    let addr = format!("0.0.0.0:{port}");
    let listener =
        TcpListener::bind(&addr).map_err(|e| format!("failed to bind to {addr}: {e}"))?;

    output::info(&format!("Registry server listening on http://{addr}"));
    output::info(&format!("Serving packages from {}", data_dir.display()));
    output::info("API v1 endpoints available under /v1/");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let data_dir = data_dir.clone();
                let state = state.clone();
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(&mut stream);
                    let mut request_line = String::new();
                    if reader.read_line(&mut request_line).is_err() {
                        return;
                    }

                    let parts: Vec<&str> = request_line.split_whitespace().collect();
                    if parts.len() < 2 {
                        return;
                    }

                    let method = parts[0];
                    let path = parts[1];

                    // Read headers to find Content-Length for POST/PUT bodies
                    let mut content_length = 0usize;
                    loop {
                        let mut header_line = String::new();
                        if reader.read_line(&mut header_line).is_err() {
                            break;
                        }
                        let trimmed = header_line.trim();
                        if trimmed.is_empty() {
                            break;
                        }
                        if let Some(len_str) = trimmed
                            .to_lowercase()
                            .strip_prefix("content-length:")
                            .map(|s| s.trim())
                        {
                            content_length = len_str.parse().unwrap_or(0);
                        }
                    }

                    // Read body if present
                    let mut body = String::new();
                    if content_length > 0 && content_length <= 10 * 1024 * 1024 {
                        let mut buf = vec![0u8; content_length];
                        use std::io::Read;
                        let inner = reader.get_mut();
                        if inner.read_exact(&mut buf).is_ok() {
                            body = String::from_utf8_lossy(&buf).to_string();
                        }
                    }

                    let (headers, binary_body) =
                        route_request(method, path, &body, &data_dir, &state);
                    let _ = stream.write_all(headers.as_bytes());
                    if let Some(binary) = binary_body {
                        let _ = stream.write_all(&binary);
                    }
                });
            }
            Err(e) => {
                output::warning(&format!("connection error: {e}"));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_state() -> (TempDir, RwLock<RegistryState>) {
        let dir = tempfile::tempdir().expect("temp dir");
        let state = RwLock::new(RegistryState::load(dir.path()));
        (dir, state)
    }

    #[test]
    fn test_registry_publish_and_search() {
        let (_dir, state) = test_state();

        let meta = PackageMetadata {
            name: "hello".to_string(),
            versions: vec![VersionMetadata {
                version: "1.0.0".to_string(),
                nar_hash: Some("sha256-abc".to_string()),
                file_hash: None,
                dependencies: HashMap::new(),
                description: "A test package".to_string(),
                license: Some("MIT".to_string()),
                published_at: Some("1234567890".to_string()),
            }],
        };

        state.write().unwrap().save_package(meta).expect("save");

        let guard = state.read().unwrap();
        let results = guard.search("hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "hello");
        assert!(results[0].versions.contains(&"1.0.0".to_string()));
        drop(guard);

        let mut guard = state.write().unwrap();
        let pkg = guard.load_package("hello");
        assert!(pkg.is_some());
        let pkg = pkg.unwrap();
        assert_eq!(pkg.versions[0].version, "1.0.0");
    }

    #[test]
    fn test_registry_search_case_insensitive() {
        let (_dir, state) = test_state();

        let meta = PackageMetadata {
            name: "MyPackage".to_string(),
            versions: vec![VersionMetadata {
                version: "1.0.0".to_string(),
                nar_hash: None,
                file_hash: None,
                dependencies: HashMap::new(),
                description: String::new(),
                license: None,
                published_at: None,
            }],
        };

        state.write().unwrap().save_package(meta).expect("save");
        let guard = state.read().unwrap();
        assert_eq!(guard.search("mypackage").len(), 1);
        assert_eq!(guard.search("nonexistent").len(), 0);
    }

    #[test]
    fn test_registry_multi_version() {
        let (_dir, state) = test_state();
        let meta = PackageMetadata {
            name: "lib".to_string(),
            versions: vec![
                VersionMetadata {
                    version: "2.0.0".to_string(),
                    nar_hash: Some("sha256-xyz".to_string()),
                    file_hash: None,
                    dependencies: HashMap::new(),
                    description: "v2".to_string(),
                    license: None,
                    published_at: None,
                },
                VersionMetadata {
                    version: "1.0.0".to_string(),
                    nar_hash: Some("sha256-abc".to_string()),
                    file_hash: None,
                    dependencies: HashMap::new(),
                    description: "v1".to_string(),
                    license: None,
                    published_at: None,
                },
            ],
        };
        state.write().unwrap().save_package(meta).expect("save");
        let mut guard = state.write().unwrap();
        let pkg = guard.load_package("lib").cloned().unwrap();
        assert_eq!(pkg.versions.len(), 2);
    }
}
