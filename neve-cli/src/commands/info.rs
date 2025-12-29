//! The `neve info` command.
//!
//! Shows detailed information about a package or platform.

#[cfg(unix)]
use crate::output;
use crate::platform::{PlatformCapabilities, print_cross_platform_note};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::PathBuf;

/// Show platform capabilities and information.
pub fn platform_info() -> Result<(), String> {
    println!("Neve Platform Information");
    println!("==========================");
    println!();

    let caps = PlatformCapabilities::detect();
    caps.print_info();

    println!();
    println!("Feature Availability:");
    println!("  Language (eval, check, repl):  \x1b[32myes\x1b[0m");
    println!("  Formatting:                    \x1b[32myes\x1b[0m");
    println!("  LSP:                           \x1b[32myes\x1b[0m");

    if caps.native_sandbox {
        println!("  Native sandboxed builds:       \x1b[32myes\x1b[0m");
    } else if caps.docker_available {
        println!("  Native sandboxed builds:       \x1b[33mno (using Docker)\x1b[0m");
    } else {
        println!("  Native sandboxed builds:       \x1b[31mno\x1b[0m");
    }

    if caps.docker_available {
        println!("  Docker builds:                 \x1b[32myes\x1b[0m");
    } else {
        println!("  Docker builds:                 \x1b[31mno (Docker not found)\x1b[0m");
    }

    if caps.system_config {
        println!("  System configuration:          \x1b[32myes\x1b[0m");
    } else {
        println!("  System configuration:          \x1b[33mLinux only\x1b[0m");
    }

    // Show cross-platform note if not on Linux
    print_cross_platform_note();

    Ok(())
}

/// Show detailed information about a package (Unix only).
#[cfg(unix)]
pub fn run(package: &str) -> Result<(), String> {
    let store_dir = get_store_dir();

    // Try to find the package in the store
    if store_dir.exists() {
        for entry in fs::read_dir(&store_dir).map_err(|e| format!("Failed to read store: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.contains(package) {
                let path = entry.path();

                output::info(&format!("Package: {name_str}"));
                println!("Path: {}", path.display());

                // Read derivation info if available
                let drv_path = path.with_extension("drv");
                if drv_path.exists()
                    && let Ok(drv_content) = fs::read_to_string(&drv_path)
                {
                    println!("Derivation: {}", drv_path.display());

                    // Parse JSON derivation
                    if let Ok(drv) = serde_json::from_str::<serde_json::Value>(&drv_content) {
                        if let Some(name) = drv.get("name").and_then(|v| v.as_str()) {
                            println!("Name: {name}");
                        }
                        if let Some(system) = drv.get("system").and_then(|v| v.as_str()) {
                            println!("System: {system}");
                        }
                    }
                }

                // Show size
                if let Ok(size) = get_dir_size(&path) {
                    println!("Size: {}", format_size(size));
                }

                // Show contents
                println!("\nContents:");
                show_dir_tree(&path, "", 2)?;

                return Ok(());
            }
        }
    }

    Err(format!("Package '{}' not found", package))
}

/// Get the store directory.
#[cfg(unix)]
fn get_store_dir() -> PathBuf {
    std::env::var("NEVE_STORE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/neve/store"))
}

/// Get the total size of a directory.
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

/// Format a size in bytes as a human-readable string.
#[cfg(unix)]
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// Show a directory tree up to a certain depth.
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
