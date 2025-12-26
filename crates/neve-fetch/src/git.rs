//! Git repository fetching.

use std::path::Path;
use git2::{Repository, FetchOptions, Oid, build::RepoBuilder};
use neve_derive::Hash;

use crate::FetchError;

/// Clone or fetch a Git repository.
pub fn clone_repo(url: &str, dest: &Path) -> Result<Repository, FetchError> {
    if dest.exists() {
        // Open existing repo and fetch updates
        let repo = Repository::open(dest)
            .map_err(|e| FetchError::Git(format!("failed to open repository: {}", e)))?;
        
        // Fetch all remotes in a block so the borrow ends
        {
            let mut remote = repo.find_remote("origin")
                .map_err(|e| FetchError::Git(format!("failed to find remote: {}", e)))?;
            
            let mut fetch_options = FetchOptions::new();
            remote.fetch(&[] as &[&str], Some(&mut fetch_options), None)
                .map_err(|e| FetchError::Git(format!("failed to fetch: {}", e)))?;
        }
        
        Ok(repo)
    } else {
        // Clone the repository
        RepoBuilder::new()
            .clone(url, dest)
            .map_err(|e| FetchError::Git(format!("failed to clone: {}", e)))
    }
}

/// Checkout a specific revision.
pub fn checkout_rev(repo: &Repository, rev: &str) -> Result<Oid, FetchError> {
    // Try to parse as a commit hash first
    if let Ok(oid) = Oid::from_str(rev)
        && let Ok(commit) = repo.find_commit(oid) {
            repo.checkout_tree(commit.as_object(), None)
                .map_err(|e| FetchError::Git(format!("failed to checkout: {}", e)))?;
            
            repo.set_head_detached(oid)
                .map_err(|e| FetchError::Git(format!("failed to set HEAD: {}", e)))?;
            
            return Ok(oid);
        }
    
    // Try as a branch name
    if let Ok(reference) = repo.find_reference(&format!("refs/remotes/origin/{}", rev)) {
        let commit = reference.peel_to_commit()
            .map_err(|e| FetchError::Git(format!("failed to peel to commit: {}", e)))?;
        
        let oid = commit.id();
        
        repo.checkout_tree(commit.as_object(), None)
            .map_err(|e| FetchError::Git(format!("failed to checkout: {}", e)))?;
        
        repo.set_head_detached(oid)
            .map_err(|e| FetchError::Git(format!("failed to set HEAD: {}", e)))?;
        
        return Ok(oid);
    }
    
    // Try as a tag
    if let Ok(reference) = repo.find_reference(&format!("refs/tags/{}", rev)) {
        let commit = reference.peel_to_commit()
            .map_err(|e| FetchError::Git(format!("failed to peel to commit: {}", e)))?;
        
        let oid = commit.id();
        
        repo.checkout_tree(commit.as_object(), None)
            .map_err(|e| FetchError::Git(format!("failed to checkout: {}", e)))?;
        
        repo.set_head_detached(oid)
            .map_err(|e| FetchError::Git(format!("failed to set HEAD: {}", e)))?;
        
        return Ok(oid);
    }
    
    // Try to resolve as a reference
    let obj = repo.revparse_single(rev)
        .map_err(|e| FetchError::Git(format!("failed to resolve revision '{}': {}", rev, e)))?;
    
    let commit = obj.peel_to_commit()
        .map_err(|e| FetchError::Git(format!("failed to peel to commit: {}", e)))?;
    
    let oid = commit.id();
    
    repo.checkout_tree(commit.as_object(), None)
        .map_err(|e| FetchError::Git(format!("failed to checkout: {}", e)))?;
    
    repo.set_head_detached(oid)
        .map_err(|e| FetchError::Git(format!("failed to set HEAD: {}", e)))?;
    
    Ok(oid)
}

/// Hash a directory's contents for content-addressing.
pub fn hash_directory(path: &Path) -> Result<Hash, FetchError> {
    crate::verify::hash_dir(path)
}

/// Get the short hash (first 7 characters) of a commit.
pub fn short_hash(oid: &Oid) -> String {
    oid.to_string()[..7].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    #[ignore] // Requires network access
    fn test_clone_repo() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("repo");
        
        // Clone a small test repo
        let result = clone_repo(
            "https://github.com/octocat/Hello-World.git",
            &repo_path,
        );
        
        assert!(result.is_ok());
        assert!(repo_path.join(".git").exists());
    }
    
    #[test]
    fn test_short_hash() {
        let oid = Oid::from_str("abc1234567890def").unwrap();
        assert_eq!(short_hash(&oid), "abc1234");
    }
}
