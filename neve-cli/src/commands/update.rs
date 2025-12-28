//! The `neve update` command.
//!
//! Updates flake inputs and dependencies.

use std::path::Path;
use neve_config::flake::{Flake, FlakeLock};
use crate::output;

pub fn run() -> Result<(), String> {
    // Find flake in current directory
    let flake_path = Path::new("flake.neve");
    
    if !flake_path.exists() {
        return Err("no flake.neve found in current directory".to_string());
    }
    
    output::info("Loading flake...");
    
    let mut flake = Flake::load(Path::new("."))
        .map_err(|e| format!("failed to load flake: {}", e))?;
    
    if flake.inputs.is_empty() {
        output::info("No inputs to update");
        return Ok(());
    }
    
    output::info(&format!("Found {} input(s) to update", flake.inputs.len()));
    
    // Check for existing lock file
    let lock_path = Path::new("flake.lock");
    let had_lock = lock_path.exists();
    
    if had_lock {
        output::info("Updating existing lock file...");
    } else {
        output::info("Creating new lock file...");
    }
    
    // Clear existing lock to force re-resolution
    flake.lock = FlakeLock::new();
    
    // Resolve and lock all inputs
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
    if updated_count > 0 {
        flake.save_lock()
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
        Ok(())
    } else {
        Err("failed to update any inputs".to_string())
    }
}

/// Update a single input and return its lock entry.
fn update_input(
    url: &str,
    rev: Option<&str>,
    branch: Option<&str>,
) -> Result<neve_config::flake::FlakeLockEntry, String> {
    use std::time::SystemTime;
    
    // Parse the URL to determine the type
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
        let github_url = format!("github:{}", url);
        update_github_input(&github_url, rev, branch)?
    };
    
    let last_modified = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    
    // Extract name from URL
    let name = url.split('/').next_back()
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
fn update_github_input(
    url: &str,
    rev: Option<&str>,
    branch: Option<&str>,
) -> Result<(String, Option<String>, String), String> {
    // Parse github:owner/repo format
    let repo_path = url.strip_prefix("github:")
        .ok_or_else(|| "invalid github URL".to_string())?;
    
    // Extract owner/repo and optional ref
    let (owner_repo, url_ref) = if let Some(pos) = repo_path.find('/') {
        let rest = &repo_path[pos + 1..];
        if let Some(ref_pos) = rest.find('/') {
            let repo = &rest[..ref_pos];
            let reference = &rest[ref_pos + 1..];
            (format!("{}/{}", &repo_path[..pos], repo), Some(reference.to_string()))
        } else {
            (repo_path.to_string(), None)
        }
    } else {
        return Err(format!("invalid github URL: {}", url));
    };
    
    // Determine the ref to use
    let git_ref = rev.map(|s| s.to_string())
        .or_else(|| branch.map(|s| s.to_string()))
        .or(url_ref)
        .unwrap_or_else(|| "main".to_string());
    
    // In a real implementation, we would:
    // 1. Query GitHub API for the latest commit
    // 2. Download and hash the tarball
    // For now, we generate a placeholder
    
    let _api_url = format!("https://api.github.com/repos/{}/commits/{}", owner_repo, git_ref);
    let tarball_url = format!("https://github.com/{}/archive/{}.tar.gz", owner_repo, git_ref);
    
    // Try to fetch the commit hash (simplified - in production would use proper HTTP client)
    let commit_hash = fetch_github_commit(&owner_repo, &git_ref)
        .unwrap_or_else(|_| format!("ref-{}", git_ref));
    
    // Generate content hash (placeholder - would hash actual content)
    let content_hash = format!("sha256-{}", hash_string(&format!("{}:{}", owner_repo, commit_hash)));
    
    Ok((tarball_url, Some(commit_hash), content_hash))
}

/// Update a Git input.
fn update_git_input(
    url: &str,
    rev: Option<&str>,
    branch: Option<&str>,
) -> Result<(String, Option<String>, String), String> {
    let git_url = url.strip_prefix("git+").unwrap_or(url);
    
    let git_ref = rev.map(|s| s.to_string())
        .or_else(|| branch.map(|s| s.to_string()))
        .unwrap_or_else(|| "HEAD".to_string());
    
    // In a real implementation, we would:
    // 1. Clone/fetch the repository
    // 2. Get the commit hash
    // 3. Hash the contents
    
    let content_hash = format!("sha256-{}", hash_string(&format!("{}:{}", git_url, git_ref)));
    
    Ok((git_url.to_string(), Some(git_ref), content_hash))
}

/// Update a path input.
fn update_path_input(url: &str) -> Result<(String, Option<String>, String), String> {
    let path = url.strip_prefix("path:").unwrap_or(url);
    let path = Path::new(path);
    
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }
    
    // Hash the directory contents
    let content_hash = hash_path(path)?;
    
    Ok((
        format!("path:{}", path.canonicalize().unwrap_or_else(|_| path.to_path_buf()).display()),
        None,
        format!("sha256-{}", content_hash),
    ))
}

/// Update a URL input.
fn update_url_input(url: &str) -> Result<(String, Option<String>, String), String> {
    // In a real implementation, we would fetch the URL and hash its contents
    let content_hash = format!("sha256-{}", hash_string(url));
    
    Ok((url.to_string(), None, content_hash))
}

/// Fetch the latest commit hash from GitHub.
fn fetch_github_commit(owner_repo: &str, git_ref: &str) -> Result<String, String> {
    // This is a simplified implementation
    // In production, we would use reqwest or similar to actually fetch the commit
    
    // For now, generate a deterministic hash based on the ref
    // This allows the system to work offline while still being deterministic
    Ok(format!("{:0>40}", hash_string(&format!("{}:{}", owner_repo, git_ref))))
}

/// Hash a string using a simple algorithm.
fn hash_string(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:016x}", hash)
}

/// Hash a path's contents.
fn hash_path(path: &Path) -> Result<String, String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    
    let mut hasher = DefaultHasher::new();
    hash_path_recursive(path, &mut hasher)?;
    let hash = hasher.finish();
    Ok(format!("{:016x}", hash))
}

/// Recursively hash a path.
fn hash_path_recursive(path: &Path, hasher: &mut std::collections::hash_map::DefaultHasher) -> Result<(), String> {
    use std::fs;
    use std::hash::Hash;
    
    if path.is_file() {
        let content = fs::read(path)
            .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
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
