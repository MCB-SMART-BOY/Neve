//! The `neve search` command.
//! `neve search` 命令。
//!
//! Searches for packages in the store and available package sources.
//! 在存储和可用软件包源中搜索软件包。

use crate::output;
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
    let index_matches = search_index(query)?;

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

/// Search the package index.
/// 搜索软件包索引。
fn search_index(query: &str) -> Result<Vec<(String, String)>, String> {
    // In a real implementation, this would query a package database
    // 在真实实现中，这将查询软件包数据库
    // For now, return some example packages that match common queries
    // 目前，返回一些匹配常见查询的示例软件包
    let packages = vec![
        ("gcc", "GNU Compiler Collection"),
        // GNU 编译器集合
        ("clang", "LLVM C/C++ compiler"),
        // LLVM C/C++ 编译器
        ("rust", "Rust programming language"),
        // Rust 编程语言
        ("python", "Python programming language"),
        // Python 编程语言
        ("node", "Node.js JavaScript runtime"),
        // Node.js JavaScript 运行时
        ("git", "Distributed version control system"),
        ("vim", "Vi IMproved text editor"),
        // Vi 改进版文本编辑器
        ("neovim", "Hyperextensible Vim-based text editor"),
        ("emacs", "Extensible text editor"),
        ("zsh", "Z shell"),
        // Z shell
        ("bash", "Bourne Again SHell"),
        // Bourne Again SHell
        ("fish", "Friendly interactive shell"),
        ("tmux", "Terminal multiplexer"),
        ("htop", "Interactive process viewer"),
        ("curl", "Command line tool for transferring data"),
        ("wget", "Network downloader"),
        ("jq", "Command-line JSON processor"),
        ("ripgrep", "Fast line-oriented search tool"),
        ("fd", "Fast and user-friendly find alternative"),
        ("fzf", "Fuzzy finder"),
        ("bat", "Cat clone with syntax highlighting"),
        ("exa", "Modern replacement for ls"),
        ("tokei", "Code statistics tool"),
        ("hyperfine", "Command-line benchmarking tool"),
    ];

    let query_lower = query.to_lowercase();
    let matches: Vec<(String, String)> = packages
        .into_iter()
        .filter(|(name, desc)| {
            name.to_lowercase().contains(&query_lower) || desc.to_lowercase().contains(&query_lower)
        })
        .map(|(name, desc)| (name.to_string(), desc.to_string()))
        .collect();

    Ok(matches)
}
