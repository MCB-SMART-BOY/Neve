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

    output::info(&format!("Searching for '{query}'..."));
    // 正在搜索 '{}'...
    println!();

    let mut found = false;

    // Search in store
    // 在存储中搜索
    if store_dir.exists() {
        let matches = search_store(&store_dir, query)?;
        if !matches.is_empty() {
            println!("Installed packages:");
            // 已安装的软件包：
            for (name, path) in &matches {
                println!("  {} - {}", name, path.display());
            }
            found = true;
        }
    }

    // Search in package index (if available)
    // 在软件包索引中搜索（如果可用）
    let index_matches = search_index(query)?;
    if !index_matches.is_empty() {
        if found {
            println!();
        }
        println!("Available packages:");
        // 可用的软件包：
        for (name, description) in &index_matches {
            println!("  {} - {}", name, description);
        }
        found = true;
    }

    if !found {
        println!("No packages found matching '{}'", query);
        // 没有找到匹配 '{}' 的软件包
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
        // 读取存储失败：{}
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        // 读取条目失败：{}
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
        // 分布式版本控制系统
        ("vim", "Vi IMproved text editor"),
        // Vi 改进版文本编辑器
        ("neovim", "Hyperextensible Vim-based text editor"),
        // 高度可扩展的基于 Vim 的文本编辑器
        ("emacs", "Extensible text editor"),
        // 可扩展文本编辑器
        ("zsh", "Z shell"),
        // Z shell
        ("bash", "Bourne Again SHell"),
        // Bourne Again SHell
        ("fish", "Friendly interactive shell"),
        // 友好的交互式 shell
        ("tmux", "Terminal multiplexer"),
        // 终端多路复用器
        ("htop", "Interactive process viewer"),
        // 交互式进程查看器
        ("curl", "Command line tool for transferring data"),
        // 用于传输数据的命令行工具
        ("wget", "Network downloader"),
        // 网络下载器
        ("jq", "Command-line JSON processor"),
        // 命令行 JSON 处理器
        ("ripgrep", "Fast line-oriented search tool"),
        // 快速面向行的搜索工具
        ("fd", "Fast and user-friendly find alternative"),
        // 快速且用户友好的 find 替代品
        ("fzf", "Fuzzy finder"),
        // 模糊查找器
        ("bat", "Cat clone with syntax highlighting"),
        // 带语法高亮的 cat 克隆
        ("exa", "Modern replacement for ls"),
        // ls 的现代替代品
        ("tokei", "Code statistics tool"),
        // 代码统计工具
        ("hyperfine", "Command-line benchmarking tool"),
        // 命令行基准测试工具
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
