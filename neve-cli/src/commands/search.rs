//! The `neve search` command.
//!
//! Searches for packages in the store and available package sources.

use crate::output;
use std::fs;
use std::path::PathBuf;

/// Search for packages matching a query.
pub fn run(query: &str) -> Result<(), String> {
    let store_dir = get_store_dir();

    output::info(&format!("Searching for '{query}'..."));
    println!();

    let mut found = false;

    // Search in store
    if store_dir.exists() {
        let matches = search_store(&store_dir, query)?;
        if !matches.is_empty() {
            println!("Installed packages:");
            for (name, path) in &matches {
                println!("  {} - {}", name, path.display());
            }
            found = true;
        }
    }

    // Search in package index (if available)
    let index_matches = search_index(query)?;
    if !index_matches.is_empty() {
        if found {
            println!();
        }
        println!("Available packages:");
        for (name, description) in &index_matches {
            println!("  {} - {}", name, description);
        }
        found = true;
    }

    if !found {
        println!("No packages found matching '{}'", query);
    }

    Ok(())
}

/// Get the store directory.
fn get_store_dir() -> PathBuf {
    std::env::var("NEVE_STORE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/neve/store"))
}

/// Search for packages in the store.
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
    matches.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(matches)
}

/// Search the package index.
fn search_index(query: &str) -> Result<Vec<(String, String)>, String> {
    // In a real implementation, this would query a package database
    // For now, return some example packages that match common queries
    let packages = vec![
        ("gcc", "GNU Compiler Collection"),
        ("clang", "LLVM C/C++ compiler"),
        ("rust", "Rust programming language"),
        ("python", "Python programming language"),
        ("node", "Node.js JavaScript runtime"),
        ("git", "Distributed version control system"),
        ("vim", "Vi IMproved text editor"),
        ("neovim", "Hyperextensible Vim-based text editor"),
        ("emacs", "Extensible text editor"),
        ("zsh", "Z shell"),
        ("bash", "Bourne Again SHell"),
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
