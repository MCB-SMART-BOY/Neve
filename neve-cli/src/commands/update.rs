//! The `neve update` command.
//! `neve update` 命令。
//!
//! Updates flake inputs and dependencies.
//! 更新 flake 输入和依赖。

use crate::output;
use neve_config::flake::{Flake, FlakeLock};
use neve_derive::Hash;
use neve_fetch::{git as fetch_git, url as fetch_url, verify as fetch_verify};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
    let total_inputs = flake.inputs.len();

    let mut progress = output::ProgressBar::new(total_inputs, "Updating inputs");

    for (i, (name, input)) in flake.inputs.iter().enumerate() {
        output::numbered_item(i + 1, &format!("Updating '{}'", name));

        match update_input(
            &input.url,
            input.rev.as_deref(),
            input.branch.as_deref(),
            input.tag.as_deref(),
        ) {
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
        progress.update(i + 1);
    }

    progress.finish_with_message(&format!(
        "Updated {} of {} input(s)",
        updated_count, total_inputs
    ));

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
    tag: Option<&str>,
) -> Result<neve_config::flake::FlakeLockEntry, String> {
    // Parse the URL to determine the type
    // 解析 URL 以确定类型
    let (resolved_url, resolved_rev, hash) = if url.starts_with("github:") {
        update_github_input(url, rev, branch, tag)?
    } else if url.starts_with("git+") || url.ends_with(".git") {
        update_git_input(url, rev, branch, tag)?
    } else if url.starts_with("path:") || url.starts_with("./") || url.starts_with("/") {
        update_path_input(url)?
    } else if url.starts_with("http://") || url.starts_with("https://") {
        update_url_input(url)?
    } else {
        // Assume it's a GitHub shorthand
        // 假设它是 GitHub 简写
        let github_url = format!("github:{}", url);
        update_github_input(&github_url, rev, branch, tag)?
    };

    let last_modified = stable_last_modified_from_lock_hash(&hash);

    // Extract name from URL
    // 从 URL 中提取名称
    let name = infer_input_name(url);

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
    tag: Option<&str>,
) -> Result<(String, Option<String>, String), String> {
    // Parse github:owner/repo format
    // 解析 github:owner/repo 格式
    let repo_path = url
        .strip_prefix("github:")
        .ok_or_else(|| "invalid github URL".to_string())?;

    let parts: Vec<&str> = repo_path.split('/').collect();
    if parts.len() < 2 {
        return Err(format!("invalid github URL: {}", url));
    }
    let owner = parts[0];
    let repo = parts[1];
    let owner_repo = format!("{}/{}", owner, repo);
    let url_ref = if parts.len() > 2 {
        Some(parts[2..].join("/"))
    } else {
        None
    };

    // Determine the ref to use
    // 确定要使用的 ref
    let git_ref = rev
        .map(|s| s.to_string())
        .or_else(|| tag.map(|s| s.to_string()))
        .or_else(|| branch.map(|s| s.to_string()))
        .or(url_ref)
        .unwrap_or_else(|| "HEAD".to_string());

    let git_url = format!("https://github.com/{}.git", owner_repo);
    let (commit_hash, content_hash) = resolve_git_source(&git_url, &git_ref)?;
    let tarball_url = format!(
        "https://github.com/{}/archive/{}.tar.gz",
        owner_repo, commit_hash
    );

    Ok((tarball_url, Some(commit_hash), hash_to_lock(&content_hash)))
}

/// Update a Git input.
/// 更新 Git 输入。
fn update_git_input(
    url: &str,
    rev: Option<&str>,
    branch: Option<&str>,
    tag: Option<&str>,
) -> Result<(String, Option<String>, String), String> {
    let git_url = url.strip_prefix("git+").unwrap_or(url);

    let git_ref = rev
        .map(|s| s.to_string())
        .or_else(|| tag.map(|s| s.to_string()))
        .or_else(|| branch.map(|s| s.to_string()))
        .unwrap_or_else(|| "HEAD".to_string());

    let (resolved_rev, content_hash) = resolve_git_source(git_url, &git_ref)?;

    Ok((
        git_url.to_string(),
        Some(resolved_rev),
        hash_to_lock(&content_hash),
    ))
}

/// Update a path input.
/// 更新路径输入。
fn update_path_input(url: &str) -> Result<(String, Option<String>, String), String> {
    let path = url.strip_prefix("path:").unwrap_or(url);
    let path = Path::new(path)
        .canonicalize()
        .map_err(|e| format!("path does not exist or is inaccessible: {} ({})", path, e))?;

    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }

    let metadata = fs::symlink_metadata(&path)
        .map_err(|e| format!("cannot read metadata for {}: {}", path.display(), e))?;
    let content_hash = if metadata.file_type().is_symlink() {
        let target =
            fs::read_link(&path).map_err(|e| format!("cannot read symlink target: {}", e))?;
        Hash::of(target.as_os_str().to_string_lossy().as_bytes())
    } else if metadata.is_file() {
        let content =
            fs::read(&path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        Hash::of(&content)
    } else if metadata.is_dir() {
        fetch_verify::hash_dir(&path)
            .map_err(|e| format!("cannot hash {}: {}", path.display(), e))?
    } else {
        return Err(format!("unsupported path type: {}", path.display()));
    };

    Ok((
        format!("path:{}", path.display()),
        None,
        hash_to_lock(&content_hash),
    ))
}

/// Update a URL input.
/// 更新 URL 输入。
fn update_url_input(url: &str) -> Result<(String, Option<String>, String), String> {
    let content = fetch_url::fetch_url(url).map_err(|e| format!("failed to fetch URL: {}", e))?;
    let content_hash = Hash::of(&content);
    Ok((url.to_string(), None, hash_to_lock(&content_hash)))
}

/// Resolve a git source to a concrete commit and content hash.
/// 将 git 源解析为具体提交和内容哈希。
fn resolve_git_source(url: &str, git_ref: &str) -> Result<(String, Hash), String> {
    let work_dir = temp_work_dir("neve-update-git")?;
    let clone_path = work_dir.join("repo");

    let repo =
        fetch_git::clone_repo(url, &clone_path).map_err(|e| format!("failed to clone: {}", e))?;
    let oid = fetch_git::checkout_rev(&repo, git_ref)
        .map_err(|e| format!("failed to checkout revision '{}': {}", git_ref, e))?;
    drop(repo);

    let git_dir = clone_path.join(".git");
    if git_dir.exists() {
        fs::remove_dir_all(&git_dir).map_err(|e| format!("failed to remove .git: {}", e))?;
    }

    let hash = fetch_git::hash_directory(&clone_path)
        .map_err(|e| format!("failed to hash repo: {}", e))?;
    let _ = fs::remove_dir_all(&work_dir);
    Ok((oid.to_string(), hash))
}

/// Format a hash for lock file storage.
/// 将哈希格式化为锁文件字符串。
fn hash_to_lock(hash: &Hash) -> String {
    format!("blake3-{}", hash.to_hex())
}

/// Produce a deterministic lastModified value from a lock hash.
/// 基于锁哈希生成确定性的 lastModified 值。
fn stable_last_modified_from_lock_hash(lock_hash: &str) -> u64 {
    let raw = lock_hash.strip_prefix("blake3-").unwrap_or(lock_hash);
    let prefix = raw.get(..16).unwrap_or(raw);
    u64::from_str_radix(prefix, 16).unwrap_or(0)
}

/// Create a temporary working directory path.
/// 创建临时工作目录路径。
fn temp_work_dir(prefix: &str) -> Result<PathBuf, String> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let path = std::env::temp_dir().join(format!("{}-{}-{}", prefix, std::process::id(), nonce));
    fs::create_dir_all(&path).map_err(|e| format!("failed to create temp dir: {}", e))?;
    Ok(path)
}

/// Infer a human-friendly input name from URL.
/// 从 URL 推断友好的输入名称。
fn infer_input_name(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("github:") {
        let mut parts = rest.split('/');
        let _owner = parts.next();
        if let Some(repo) = parts.next() {
            return repo.trim_end_matches(".git").to_string();
        }
    }
    url.split('/')
        .next_back()
        .unwrap_or("unknown")
        .trim_end_matches(".git")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stable_last_modified_from_lock_hash_is_deterministic() {
        let hash = "blake3-0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let ts1 = stable_last_modified_from_lock_hash(hash);
        let ts2 = stable_last_modified_from_lock_hash(hash);
        assert_eq!(ts1, ts2);
    }

    #[test]
    fn test_stable_last_modified_from_lock_hash_accepts_raw_hex() {
        let raw_hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let ts = stable_last_modified_from_lock_hash(raw_hash);
        assert_eq!(ts, 0x0123456789abcdef);
    }
}
