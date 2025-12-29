//! The `neve store` commands.
//! `neve store` 命令。

use crate::output;
use neve_store::{Store, gc::GarbageCollector};

/// Run garbage collection.
/// 运行垃圾回收。
pub fn gc() -> Result<(), String> {
    output::info("Running garbage collection...");

    let mut store = Store::open().map_err(|e| format!("Failed to open store: {}", e))?;

    let mut gc = GarbageCollector::new(&mut store);

    // First do a dry run
    // 首先进行模拟运行
    let to_delete = gc
        .dry_run()
        .map_err(|e| format!("Failed to analyze store: {}", e))?;

    if to_delete.is_empty() {
        output::success("No garbage to collect.");
        return Ok(());
    }

    output::info(&format!("Found {} paths to delete:", to_delete.len()));
    for path in &to_delete {
        println!("  {}", path.display_name());
    }

    println!();
    output::info("Deleting...");

    let result = gc
        .collect()
        .map_err(|e| format!("Failed to collect garbage: {}", e))?;

    output::success(&format!(
        "Deleted {} paths, freed {}.",
        result.deleted,
        result.freed_human()
    ));

    Ok(())
}

/// Show store information.
/// 显示存储信息。
pub fn info() -> Result<(), String> {
    let store = Store::open().map_err(|e| format!("Failed to open store: {}", e))?;

    let paths = store
        .list_paths()
        .map_err(|e| format!("Failed to list paths: {}", e))?;

    let size = store
        .size()
        .map_err(|e| format!("Failed to get store size: {}", e))?;

    println!("Neve Store Information");
    // Neve 存储信息
    println!("======================");
    println!();
    println!("Location: {}", store.root().display());
    println!("Paths:    {}", paths.len());
    println!("Size:     {}", format_size(size));
    println!();

    if !paths.is_empty() {
        println!("Recent paths:");
        for path in paths.iter().take(10) {
            println!("  {}", path.display_name());
        }
        if paths.len() > 10 {
            println!("  ... and {} more", paths.len() - 10);
        }
    }

    Ok(())
}

/// Format a size in bytes to a human-readable string.
/// 将字节大小格式化为人类可读的字符串。
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
