//! The `neve store` commands.
//! `neve store` 命令。

use crate::output;
use neve_store::{Store, gc::GarbageCollector};

/// Run garbage collection.
/// 运行垃圾回收。
pub fn gc() -> Result<(), String> {
    let status = output::Status::new("Analyzing store for garbage collection");

    let store_result = Store::open();
    let mut store = match store_result {
        Ok(s) => s,
        Err(e) => {
            status.fail(Some("Failed to open store"));
            return Err(format!("Failed to open store: {}", e));
        }
    };

    let mut gc = GarbageCollector::new(&mut store);

    // First do a dry run
    // 首先进行模拟运行
    let to_delete = gc
        .dry_run()
        .map_err(|e| format!("Failed to analyze store: {}", e))?;

    status.success(Some("Store analysis complete"));

    if to_delete.is_empty() {
        output::success("No garbage to collect.");
        return Ok(());
    }

    output::header("Garbage Collection");
    output::kv("Paths to delete", &to_delete.len().to_string());
    println!();

    for path in &to_delete {
        output::list_item(&path.display_name());
    }

    println!();

    // Confirm before deletion
    // 删除前确认
    if !output::confirm("Proceed with deletion?") {
        output::info("Garbage collection cancelled");
        return Ok(());
    }

    let delete_status = output::Status::new("Deleting garbage paths");

    let collect_result = gc.collect();
    match collect_result {
        Ok(result) => {
            delete_status.success(None);
            output::success(&format!(
                "Deleted {} paths, freed {}.",
                result.deleted,
                result.freed_human()
            ));
            Ok(())
        }
        Err(e) => {
            delete_status.fail(Some("Deletion failed"));
            Err(format!("Failed to collect garbage: {}", e))
        }
    }
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

    output::header("Neve Store Information");
    output::kv("Location", &store.root().display().to_string());
    output::kv("Paths", &paths.len().to_string());
    output::kv("Size", &output::format_size(size));
    println!();

    if !paths.is_empty() {
        output::section("Recent paths");
        let mut table = output::Table::new(vec!["#", "Path"]);
        for (i, path) in paths.iter().take(10).enumerate() {
            table.add_row(vec![&(i + 1).to_string(), &path.display_name()]);
        }
        table.print();

        if paths.len() > 10 {
            output::info(&format!("... and {} more", paths.len() - 10));
        }
    }

    Ok(())
}
