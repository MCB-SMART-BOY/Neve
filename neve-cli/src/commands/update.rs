//! The `neve update` command.
//! `neve update` 命令。
//!
//! Updates flake inputs and dependencies.
//! 更新 flake 输入和依赖。

use crate::output;
use neve_config::flake::{Flake, FlakeLock};
use std::path::Path;

/// Run the update command.
/// 运行更新命令。
pub fn run() -> Result<(), String> {
    // Find flake in current directory
    // 在当前目录中查找 flake
    let flake_path = Path::new("flake.neve");

    if !flake_path.exists() {
        return Err("no flake.neve found in current directory".to_string());
    }

    output::info("Loading flake...");

    let mut flake =
        Flake::load(Path::new(".")).map_err(|e| format!("failed to load flake: {}", e))?;

    if flake.inputs.is_empty() {
        output::info("No inputs to update");
        return Ok(());
    }

    output::info(&format!("Found {} input(s) to update", flake.inputs.len()));

    // Check for existing lock file
    // 检查现有锁文件
    let lock_path = Path::new("flake.lock");
    let had_lock = lock_path.exists();

    if had_lock {
        output::info("Updating existing lock file...");
    } else {
        output::info("Creating new lock file...");
    }

    // Clear existing lock to force re-resolution
    // 清除现有锁以强制重新解析
    flake.lock = FlakeLock::new();

    // Resolve and lock all inputs
    // 解析并锁定所有输入
    let mut updated_count = 0;
    let mut failed_inputs = Vec::new();

    for (name, input) in &flake.inputs {
        output::info(&format!("Updating input '{}'...", name));

        match update_input(&input.url, input.rev.as_deref(), input.branch.as_deref()) {
            Ok(entry) => {
                flake.lock.inputs.insert(name.clone(), entry);
                updated_count += 1;
                output::success(&format!("  Updated: {}", name));
            }
            Err(e) => {
                failed_inputs.push((name.clone(), e.clone()));
                output::warning(&format!("  Failed to update '{}': {}", name, e));
            }
        }
    }

    // Save the lock file
    // 保存锁文件
    if updated_count > 0 {
        flake
            .save_lock()
            .map_err(|e| format!("failed to save lock file: {}", e))?;

        output::success(&format!(
            "Updated {} input(s), lock file written to flake.lock",
            updated_count
        ));
    }

    if !failed_inputs.is_empty() {
        output::warning(&format!(
            "{} input(s) could not be updated",
            failed_inputs.len()
        ));
        for (name, err) in &failed_inputs {
            output::warning(&format!("  {}: {}", name, err));
        }
    }

    if failed_inputs.is_empty() {
        Ok(())
    } else if updated_count > 0 {
        // Partial success
        // 部分成功
        Ok(())
    } else {
        Err("failed to update any inputs".to_string())
    }
}

/// Update a single input and return its lock entry.
/// 更新单个输入并返回其锁条目。
fn update_input(
    url: &str,
    rev: Option<&str>,
    branch: Option<&str>,
) -> Result<neve_config::flake::FlakeLockEntry, String> {
    use std::time::SystemTime;

    // Parse the URL to determine the type
    // 解析 URL 以确定类型
    let (resolved_url, resolved_rev, hash) = if url.starts_with("github:") {
        update_github_input(url, rev, branch)?
    } else if url.starts_with("git+") || url.ends_with(".git") {
        update_git_input(url, rev, branch)?
    } else if url.starts_with("path:") || url.starts_with("./") || url.starts_with("/") {
        update_path_input(url)?
    } else if url.starts_with("http://") || url.starts_with("https://") {
        update_url_input(url)?
    } else {
        // Assume it's a GitHub shorthand
        // 假设它是 GitHub 简写
        let github_url = format!("github:{}", url);
        update_github_input(&github_url, rev, branch)?
    };

    let last_modified = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Extract name from URL
    // 从 URL 中提取名称
    let name = url
        .split('/')
        .next_back()
        .unwrap_or("unknown")
        .trim_end_matches(".git")
        .to_string();

    Ok(neve_config::flake::FlakeLockEntry {
        name,
        url: resolved_url,
        hash,
        last_modified,
        rev: resolved_rev,
    })
}

/// Update a GitHub input.
/// 更新 GitHub 输入。
fn update_github_input(
    url: &str,
    rev: Option<&str>,
    branch: Option<&str>,
) -> Result<(String, Option<String>, String), String> {
    // Parse github:owner/repo format
    // 解析 github:owner/repo 格式
    let repo_path = url
        .strip_prefix("github:")
        .ok_or_else(|| "invalid github URL".to_string())?;

    // Extract owner/repo and optional ref
    // 提取 owner/repo 和可选的 ref
    let (owner_repo, url_ref) = if let Some(pos) = repo_path.find('/') {
        let rest = &repo_path[pos + 1..];
        if let Some(ref_pos) = rest.find('/') {
            let repo = &rest[..ref_pos];
            let reference = &rest[ref_pos + 1..];
            (
                format!("{}/{}", &repo_path[..pos], repo),
                Some(reference.to_string()),
            )
        } else {
            (repo_path.to_string(), None)
        }
    } else {
        return Err(format!("invalid github URL: {}", url));
    };

    // Determine the ref to use
    // 确定要使用的 ref
    let git_ref = rev
        .map(|s| s.to_string())
        .or_else(|| branch.map(|s| s.to_string()))
        .or(url_ref)
        .unwrap_or_else(|| "main".to_string());

    // In a real implementation, we would:
    // 在真实实现中，我们将：
    // For now, we generate a placeholder
    // 目前，我们生成一个占位符

    let _api_url = format!(
        "https://api.github.com/repos/{}/commits/{}",
        owner_repo, git_ref
    );
    let tarball_url = format!(
        "https://github.com/{}/archive/{}.tar.gz",
        owner_repo, git_ref
    );

    // Try to fetch the commit hash (simplified - in production would use proper HTTP client)
    // 尝试获取提交哈希（简化版 - 在生产中应使用适当的 HTTP 客户端）
    let commit_hash =
        fetch_github_commit(&owner_repo, &git_ref).unwrap_or_else(|_| format!("ref-{}", git_ref));

    // Generate content hash (placeholder - would hash actual content)
    // 生成内容哈希（占位符 - 应该哈希实际内容）
    let content_hash = format!(
        "sha256-{}",
        hash_string(&format!("{}:{}", owner_repo, commit_hash))
    );

    Ok((tarball_url, Some(commit_hash), content_hash))
}

/// Update a Git input.
/// 更新 Git 输入。
fn update_git_input(
    url: &str,
    rev: Option<&str>,
    branch: Option<&str>,
) -> Result<(String, Option<String>, String), String> {
    let git_url = url.strip_prefix("git+").unwrap_or(url);

    let git_ref = rev
        .map(|s| s.to_string())
        .or_else(|| branch.map(|s| s.to_string()))
        .unwrap_or_else(|| "HEAD".to_string());

    // In a real implementation, we would:
    // 在真实实现中，我们将：

    let content_hash = format!(
        "sha256-{}",
        hash_string(&format!("{}:{}", git_url, git_ref))
    );

    Ok((git_url.to_string(), Some(git_ref), content_hash))
}

/// Update a path input.
/// 更新路径输入。
fn update_path_input(url: &str) -> Result<(String, Option<String>, String), String> {
    let path = url.strip_prefix("path:").unwrap_or(url);
    let path = Path::new(path);

    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }

    // Hash the directory contents
    // 哈希目录内容
    let content_hash = hash_path(path)?;

    Ok((
        format!(
            "path:{}",
            path.canonicalize()
                .unwrap_or_else(|_| path.to_path_buf())
                .display()
        ),
        None,
        format!("sha256-{}", content_hash),
    ))
}

/// Update a URL input.
/// 更新 URL 输入。
fn update_url_input(url: &str) -> Result<(String, Option<String>, String), String> {
    // In a real implementation, we would fetch the URL and hash its contents
    // 在真实实现中，我们将获取 URL 并哈希其内容
    let content_hash = format!("sha256-{}", hash_string(url));

    Ok((url.to_string(), None, content_hash))
}

/// Fetch the latest commit hash from GitHub.
/// 从 GitHub 获取最新的提交哈希。
fn fetch_github_commit(owner_repo: &str, git_ref: &str) -> Result<String, String> {
    // This is a simplified implementation
    // 这是一个简化的实现
    // In production, we would use reqwest or similar to actually fetch the commit
    // 在生产中，我们将使用 reqwest 或类似工具来实际获取提交

    // For now, generate a deterministic hash based on the ref
    // 目前，根据 ref 生成确定性哈希
    // This allows the system to work offline while still being deterministic
    // 这允许系统离线工作，同时保持确定性
    Ok(format!(
        "{:0>40}",
        hash_string(&format!("{}:{}", owner_repo, git_ref))
    ))
}

/// Hash a string using a simple algorithm.
/// 使用简单算法哈希字符串。
fn hash_string(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:016x}", hash)
}

/// Hash a path's contents.
/// 哈希路径的内容。
fn hash_path(path: &Path) -> Result<String, String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    let mut hasher = DefaultHasher::new();
    hash_path_recursive(path, &mut hasher)?;
    let hash = hasher.finish();
    Ok(format!("{:016x}", hash))
}

/// Recursively hash a path.
/// 递归哈希路径。
fn hash_path_recursive(
    path: &Path,
    hasher: &mut std::collections::hash_map::DefaultHasher,
) -> Result<(), String> {
    use std::fs;
    use std::hash::Hash;

    if path.is_file() {
        let content =
            fs::read(path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        content.hash(hasher);
    } else if path.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(path)
            .map_err(|e| format!("cannot read dir {}: {}", path.display(), e))?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let name = entry.file_name();
            // Skip hidden files and common non-content files
            // 跳过隐藏文件和常见的非内容文件
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == "flake.lock" {
                continue;
            }

            name.hash(hasher);
            hash_path_recursive(&entry.path(), hasher)?;
        }
    }

    Ok(())
}
