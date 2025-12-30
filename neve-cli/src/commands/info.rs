//! The `neve info` command.
//! `neve info` 命令。
//!
//! Shows detailed information about a package or platform.
//! 显示软件包或平台的详细信息。

use crate::output;
use crate::platform::{PlatformCapabilities, print_cross_platform_note};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::PathBuf;

/// Show platform capabilities and information.
/// 显示平台功能和信息。
pub fn platform_info() -> Result<(), String> {
    output::header("Neve Platform Information");

    let caps = PlatformCapabilities::detect();
    caps.print_info();

    output::section("Feature Availability");
    
    let mut table = output::Table::new(vec!["Feature", "Status"]);
    table.add_row(vec!["Language (eval, check, repl)", "yes"]);
    table.add_row(vec!["Formatting", "yes"]);
    table.add_row(vec!["LSP", "yes"]);
    
    if caps.native_sandbox {
        table.add_row(vec!["Native sandboxed builds", "yes"]);
    } else if caps.docker_available {
        table.add_row(vec!["Native sandboxed builds", "no (using Docker)"]);
    } else {
        table.add_row(vec!["Native sandboxed builds", "no"]);
    }

    if caps.docker_available {
        table.add_row(vec!["Docker builds", "yes"]);
    } else {
        table.add_row(vec!["Docker builds", "no (Docker not found)"]);
    }

    if caps.system_config {
        table.add_row(vec!["System configuration", "yes"]);
    } else {
        table.add_row(vec!["System configuration", "Linux only"]);
    }
    
    table.print();

    // Show cross-platform note if not on Linux
    // 如果不在 Linux 上，显示跨平台说明
    print_cross_platform_note();

    Ok(())
}

/// Show detailed information about a package (Unix only).
/// 显示软件包的详细信息（仅限 Unix）。
#[cfg(unix)]
pub fn run(package: &str) -> Result<(), String> {
    let store_dir = get_store_dir();

    // Try to find the package in the store
    // 尝试在存储中查找软件包
    if store_dir.exists() {
        for entry in fs::read_dir(&store_dir).map_err(|e| format!("Failed to read store: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.contains(package) {
                let path = entry.path();

                output::header(&format!("Package: {name_str}"));
                output::kv("Path", &path.display().to_string());

                // Read derivation info if available
                // 如果可用，读取派生信息
                let drv_path = path.with_extension("drv");
                if drv_path.exists()
                    && let Ok(drv_content) = fs::read_to_string(&drv_path)
                {
                    output::kv("Derivation", &drv_path.display().to_string());

                    // Parse JSON derivation
                    // 解析 JSON 派生
                    if let Ok(drv) = serde_json::from_str::<serde_json::Value>(&drv_content) {
                        if let Some(name) = drv.get("name").and_then(|v| v.as_str()) {
                            output::kv("Name", name);
                        }
                        if let Some(system) = drv.get("system").and_then(|v| v.as_str()) {
                            output::kv("System", system);
                        }
                    }
                }

                // Show size
                // 显示大小
                if let Ok(size) = get_dir_size(&path) {
                    output::kv("Size", &output::format_size(size));
                }

                // Show contents
                // 显示内容
                output::section("Contents");
                show_dir_tree(&path, "", 2)?;

                return Ok(());
            }
        }
    }

    Err(format!("Package '{}' not found", package))
}

/// Get the store directory.
/// 获取存储目录。
#[cfg(unix)]
fn get_store_dir() -> PathBuf {
    std::env::var("NEVE_STORE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/neve/store"))
}

/// Get the total size of a directory.
/// 获取目录的总大小。
#[cfg(unix)]
fn get_dir_size(path: &PathBuf) -> Result<u64, String> {
    let mut size = 0;

    if path.is_file() {
        return path
            .metadata()
            .map(|m| m.len())
            .map_err(|e| format!("Failed to get file size: {}", e));
    }

    for entry in fs::read_dir(path).map_err(|e| format!("Failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let entry_path = entry.path();

        if entry_path.is_file() {
            size += entry_path.metadata().map(|m| m.len()).unwrap_or(0);
        } else if entry_path.is_dir() {
            size += get_dir_size(&entry_path).unwrap_or(0);
        }
    }

    Ok(size)
}

/// Show a directory tree up to a certain depth.
/// 显示目录树到一定深度。
#[cfg(unix)]
fn show_dir_tree(path: &PathBuf, prefix: &str, max_depth: usize) -> Result<(), String> {
    if max_depth == 0 {
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory: {}", e))?
        .filter_map(|e| e.ok())
        .collect();

    entries.sort_by_key(|e| e.file_name());

    let count = entries.len();
    for (i, entry) in entries.into_iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let name = entry.file_name();

        println!("{}{}{}", prefix, connector, name.to_string_lossy());

        if entry.path().is_dir() {
            let new_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            show_dir_tree(&entry.path(), &new_prefix, max_depth - 1)?;
        }
    }

    Ok(())
}
