//! Git repository fetching.
//! Git 仓库获取。
//!
//! Provides functionality for cloning and checking out Git repositories.
//! 提供克隆和检出 Git 仓库的功能。

use git2::{FetchOptions, Oid, Repository, build::RepoBuilder};
use neve_derive::Hash;
use std::path::Path;

use crate::FetchError;

/// Clone or fetch a Git repository.
/// 克隆或获取 Git 仓库。
pub fn clone_repo(url: &str, dest: &Path) -> Result<Repository, FetchError> {
    if dest.exists() {
        // Open existing repo and fetch updates
        // 打开现有仓库并获取更新
        let repo = Repository::open(dest)
            .map_err(|e| FetchError::Git(format!("failed to open repository: {}", e)))?;

        // Fetch all remotes in a block so the borrow ends
        // 在代码块中获取所有远程仓库，以便借用结束
        {
            let mut remote = repo
                .find_remote("origin")
                .map_err(|e| FetchError::Git(format!("failed to find remote: {}", e)))?;

            let mut fetch_options = FetchOptions::new();
            remote
                .fetch(&[] as &[&str], Some(&mut fetch_options), None)
                .map_err(|e| FetchError::Git(format!("failed to fetch: {}", e)))?;
        }

        Ok(repo)
    } else {
        // Clone the repository
        // 克隆仓库
        RepoBuilder::new()
            .clone(url, dest)
            .map_err(|e| FetchError::Git(format!("failed to clone: {}", e)))
    }
}

/// Checkout a specific revision.
/// 检出指定的修订版本。
pub fn checkout_rev(repo: &Repository, rev: &str) -> Result<Oid, FetchError> {
    // Try to parse as a commit hash first
    // 首先尝试解析为提交哈希
    if let Ok(oid) = Oid::from_str(rev)
        && let Ok(commit) = repo.find_commit(oid)
    {
        repo.checkout_tree(commit.as_object(), None)
            .map_err(|e| FetchError::Git(format!("failed to checkout: {}", e)))?;

        repo.set_head_detached(oid)
            .map_err(|e| FetchError::Git(format!("failed to set HEAD: {}", e)))?;

        return Ok(oid);
    }

    // Try as a branch name
    // 尝试作为分支名
    if let Ok(reference) = repo.find_reference(&format!("refs/remotes/origin/{}", rev)) {
        let commit = reference
            .peel_to_commit()
            .map_err(|e| FetchError::Git(format!("failed to peel to commit: {}", e)))?;

        let oid = commit.id();

        repo.checkout_tree(commit.as_object(), None)
            .map_err(|e| FetchError::Git(format!("failed to checkout: {}", e)))?;

        repo.set_head_detached(oid)
            .map_err(|e| FetchError::Git(format!("failed to set HEAD: {}", e)))?;

        return Ok(oid);
    }

    // Try as a tag
    // 尝试作为标签
    if let Ok(reference) = repo.find_reference(&format!("refs/tags/{}", rev)) {
        let commit = reference
            .peel_to_commit()
            .map_err(|e| FetchError::Git(format!("failed to peel to commit: {}", e)))?;

        let oid = commit.id();

        repo.checkout_tree(commit.as_object(), None)
            .map_err(|e| FetchError::Git(format!("failed to checkout: {}", e)))?;

        repo.set_head_detached(oid)
            .map_err(|e| FetchError::Git(format!("failed to set HEAD: {}", e)))?;

        return Ok(oid);
    }

    // Try to resolve as a reference
    // 尝试解析为引用
    let obj = repo
        .revparse_single(rev)
        .map_err(|e| FetchError::Git(format!("failed to resolve revision '{}': {}", rev, e)))?;

    let commit = obj
        .peel_to_commit()
        .map_err(|e| FetchError::Git(format!("failed to peel to commit: {}", e)))?;

    let oid = commit.id();

    repo.checkout_tree(commit.as_object(), None)
        .map_err(|e| FetchError::Git(format!("failed to checkout: {}", e)))?;

    repo.set_head_detached(oid)
        .map_err(|e| FetchError::Git(format!("failed to set HEAD: {}", e)))?;

    Ok(oid)
}

/// Hash a directory's contents for content-addressing.
/// 对目录内容进行哈希以实现内容寻址。
pub fn hash_directory(path: &Path) -> Result<Hash, FetchError> {
    crate::verify::hash_dir(path)
}

/// Get the short hash (first 7 characters) of a commit.
/// 获取提交的短哈希（前 7 个字符）。
pub fn short_hash(oid: &Oid) -> String {
    oid.to_string()[..7].to_string()
}
