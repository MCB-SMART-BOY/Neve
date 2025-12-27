//! Store operations.

use crate::path::store_dir;
use neve_derive::{Derivation, Hash, StorePath};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during store operations.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    
    #[error("path not found: {0}")]
    PathNotFound(String),
    
    #[error("path already exists: {0}")]
    PathExists(String),
    
    #[error("invalid store path: {0}")]
    InvalidPath(String),
    
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: Hash, actual: Hash },
    
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// The Neve store.
pub struct Store {
    /// The root directory of the store.
    root: PathBuf,
    /// Cache of loaded derivations.
    derivation_cache: HashMap<StorePath, Derivation>,
}

impl Store {
    /// Open the store at the default location.
    pub fn open() -> Result<Self, StoreError> {
        Self::open_at(store_dir())
    }

    /// Open the store at a specific location.
    pub fn open_at(root: PathBuf) -> Result<Self, StoreError> {
        // Ensure the store directory exists
        fs::create_dir_all(&root)?;
        
        Ok(Self {
            root,
            derivation_cache: HashMap::new(),
        })
    }

    /// Get the store root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Check if a path exists in the store.
    pub fn path_exists(&self, path: &StorePath) -> bool {
        self.to_path(path).exists()
    }

    /// Convert a StorePath to an absolute filesystem path.
    pub fn to_path(&self, store_path: &StorePath) -> PathBuf {
        store_path.path_with_prefix(&self.root.to_string_lossy())
    }

    /// Add a file to the store with a specific hash.
    pub fn add_file(&self, source: &Path, name: &str) -> Result<StorePath, StoreError> {
        // Read and hash the file
        let content = fs::read(source)?;
        let hash = Hash::of(&content);
        
        let store_path = StorePath::new(hash, name.to_string());
        let dest = self.to_path(&store_path);
        
        if dest.exists() {
            // Already in store, verify hash
            let existing_content = fs::read(&dest)?;
            let existing_hash = Hash::of(&existing_content);
            if existing_hash != hash {
                return Err(StoreError::HashMismatch {
                    expected: hash,
                    actual: existing_hash,
                });
            }
        } else {
            // Copy to store
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source, &dest)?;
            // Make read-only
            let mut perms = fs::metadata(&dest)?.permissions();
            perms.set_readonly(true);
            fs::set_permissions(&dest, perms)?;
        }
        
        Ok(store_path)
    }

    /// Add a directory to the store.
    pub fn add_dir(&self, source: &Path, name: &str) -> Result<StorePath, StoreError> {
        // Hash the directory contents (simplified: just hash file names and contents)
        let hash = hash_dir(source)?;
        
        let store_path = StorePath::new(hash, name.to_string());
        let dest = self.to_path(&store_path);
        
        if !dest.exists() {
            copy_dir_recursive(source, &dest)?;
            make_readonly_recursive(&dest)?;
        }
        
        Ok(store_path)
    }

    /// Add content directly to the store.
    pub fn add_content(&self, content: &[u8], name: &str) -> Result<StorePath, StoreError> {
        let hash = Hash::of(content);
        let store_path = StorePath::new(hash, name.to_string());
        let dest = self.to_path(&store_path);
        
        if !dest.exists() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&dest, content)?;
            let mut perms = fs::metadata(&dest)?.permissions();
            perms.set_readonly(true);
            fs::set_permissions(&dest, perms)?;
        }
        
        Ok(store_path)
    }

    /// Add a derivation to the store.
    pub fn add_derivation(&mut self, drv: &Derivation) -> Result<StorePath, StoreError> {
        let drv_path = drv.drv_path();
        let dest = self.to_path(&drv_path);
        
        if !dest.exists() {
            let json = drv.to_json()?;
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&dest, &json)?;
        }
        
        self.derivation_cache.insert(drv_path.clone(), drv.clone());
        Ok(drv_path)
    }

    /// Read a derivation from the store.
    pub fn read_derivation(&mut self, path: &StorePath) -> Result<Derivation, StoreError> {
        if let Some(drv) = self.derivation_cache.get(path) {
            return Ok(drv.clone());
        }
        
        let fs_path = self.to_path(path);
        if !fs_path.exists() {
            return Err(StoreError::PathNotFound(path.display_name()));
        }
        
        let content = fs::read_to_string(&fs_path)?;
        let drv = Derivation::from_json(&content)?;
        self.derivation_cache.insert(path.clone(), drv.clone());
        
        Ok(drv)
    }

    /// Delete a path from the store (for garbage collection).
    pub fn delete(&self, path: &StorePath) -> Result<(), StoreError> {
        let fs_path = self.to_path(path);
        if !fs_path.exists() {
            return Ok(());
        }
        
        // Make writable first
        make_writable_recursive(&fs_path)?;
        
        if fs_path.is_dir() {
            fs::remove_dir_all(&fs_path)?;
        } else {
            fs::remove_file(&fs_path)?;
        }
        
        Ok(())
    }

    /// List all paths in the store.
    pub fn list_paths(&self) -> Result<Vec<StorePath>, StoreError> {
        let mut paths = Vec::new();
        
        if !self.root.exists() {
            return Ok(paths);
        }
        
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(store_path) = StorePath::parse(&path) {
                paths.push(store_path);
            }
        }
        
        Ok(paths)
    }

    /// Get the total size of the store in bytes.
    pub fn size(&self) -> Result<u64, StoreError> {
        dir_size(&self.root)
    }
}

/// Hash a directory's contents.
fn hash_dir(path: &Path) -> Result<Hash, StoreError> {
    let mut hasher = neve_derive::Hasher::new();
    hash_dir_recursive(path, &mut hasher)?;
    Ok(hasher.finalize())
}

fn hash_dir_recursive(path: &Path, hasher: &mut neve_derive::Hasher) -> Result<(), StoreError> {
    let mut entries: Vec<_> = fs::read_dir(path)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        hasher.update(name.as_encoded_bytes());
        
        if path.is_dir() {
            hasher.update(b"d");
            hash_dir_recursive(&path, hasher)?;
        } else {
            hasher.update(b"f");
            let content = fs::read(&path)?;
            hasher.update(&content);
        }
    }
    
    Ok(())
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), StoreError> {
    fs::create_dir_all(dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}

/// Recursively make a path read-only.
fn make_readonly_recursive(path: &Path) -> Result<(), StoreError> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            make_readonly_recursive(&entry?.path())?;
        }
    }
    
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_readonly(true);
    fs::set_permissions(path, perms)?;
    
    Ok(())
}

/// Recursively make a path writable.
#[cfg(unix)]
fn make_writable_recursive(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt;
    
    let perms = fs::metadata(path)?.permissions();
    // Set user read/write permissions (0o644 for files, 0o755 for dirs)
    let mode = if path.is_dir() { 0o755 } else { 0o644 };
    let new_perms = fs::Permissions::from_mode(perms.mode() | mode);
    fs::set_permissions(path, new_perms)?;
    
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            make_writable_recursive(&entry?.path())?;
        }
    }
    
    Ok(())
}

#[cfg(not(unix))]
fn make_writable_recursive(path: &Path) -> Result<(), StoreError> {
    let mut perms = fs::metadata(path)?.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    fs::set_permissions(path, perms)?;
    
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            make_writable_recursive(&entry?.path())?;
        }
    }
    
    Ok(())
}

/// Calculate the size of a directory.
fn dir_size(path: &Path) -> Result<u64, StoreError> {
    let mut size = 0;
    
    if !path.exists() {
        return Ok(0);
    }
    
    if path.is_file() {
        return Ok(fs::metadata(path)?.len());
    }
    
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            size += dir_size(&path)?;
        } else {
            size += fs::metadata(&path)?.len();
        }
    }
    
    Ok(size)
}

