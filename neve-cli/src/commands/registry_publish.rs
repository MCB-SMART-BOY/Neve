//! Package publishing for the Neve registry.
//! Neve 注册表的软件包发布。

use crate::output;
use std::path::PathBuf;

pub fn run(package_dir: &str, registry_url: Option<&str>) -> Result<(), String> {
    let url = registry_url
        .map(|s| s.to_string())
        .or_else(|| std::env::var("NEVE_REGISTRY").ok())
        .ok_or_else(|| "NEVE_REGISTRY not set and no --registry-url provided".to_string())?;

    let dir = PathBuf::from(package_dir);
    if !dir.exists() {
        return Err(format!("directory '{}' not found", package_dir));
    }

    // Look for flake.neve or package.neve
    let flake_path = dir.join("flake.neve");
    let package_path = dir.join("package.neve");

    let manifest_path = if flake_path.exists() {
        flake_path
    } else if package_path.exists() {
        package_path
    } else {
        return Err("no flake.neve or package.neve found in directory".to_string());
    };

    let manifest = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read manifest: {e}"))?;

    // Extract package name and version from manifest
    let name = extract_field(&manifest, "name")?;
    let version = extract_field(&manifest, "version")?;
    let description = extract_field(&manifest, "description").unwrap_or_default();

    output::info(&format!("Publishing {name} v{version} to {url}"));

    // Build package metadata
    let metadata = serde_json::json!({
        "name": name,
        "version": version,
        "description": description,
        "manifest": manifest,
    });

    let body = serde_json::to_string(&metadata)
        .map_err(|e| format!("failed to serialize metadata: {e}"))?;

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("failed to create HTTP client: {e}"))?;

    let publish_url = format!("{url}/packages/{name}");
    let response = client
        .post(&publish_url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .map_err(|e| format!("failed to publish: {e}"))?;

    if response.status().is_success() {
        output::info(&format!("Successfully published {name} v{version}"));
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        Err(format!("publish failed: HTTP {status}: {body}"))
    }
}

fn extract_field(manifest: &str, field: &str) -> Result<String, String> {
    // Simple field extraction from flake.neve / package.neve format
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&format!("{field} =")) || trimmed.starts_with(&format!("{field}=")) {
            let value = trimmed
                .split_once('=')
                .map(|x| x.1)
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches(';')
                .trim();
            return Ok(value.to_string());
        }
    }
    Err(format!("field '{}' not found in manifest", field))
}
