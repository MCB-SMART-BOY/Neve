//! Metadata database for the store.
//!
//! Stores information about derivations, their outputs, and references.

use crate::StoreError;
use neve_derive::{Hash, StorePath};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// Metadata about a store path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    /// The store path.
    pub path: StorePath,
    /// Hash of the path contents.
    pub nar_hash: Hash,
    /// Size of the path in bytes.
    pub nar_size: u64,
    /// Paths that this path references.
    pub references: HashSet<StorePath>,
    /// The derivation that produced this path (if any).
    pub deriver: Option<StorePath>,
    /// Registration time (Unix timestamp).
    pub registration_time: u64,
    /// Whether this is a valid path.
    pub valid: bool,
}

impl PathInfo {
    /// Create a new PathInfo.
    pub fn new(path: StorePath, nar_hash: Hash, nar_size: u64) -> Self {
        Self {
            path,
            nar_hash,
            nar_size,
            references: HashSet::new(),
            deriver: None,
            registration_time: current_time(),
            valid: true,
        }
    }

    /// Add a reference.
    pub fn add_reference(&mut self, path: StorePath) {
        self.references.insert(path);
    }

    /// Set the deriver.
    pub fn set_deriver(&mut self, drv: StorePath) {
        self.deriver = Some(drv);
    }
}

/// The metadata database.
pub struct Database {
    /// Root directory for the database.
    root: PathBuf,
    /// Cached path info.
    cache: HashMap<StorePath, PathInfo>,
}

impl Database {
    /// Get the root directory of the database.
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Open the database at the given root.
    pub fn open(root: PathBuf) -> Result<Self, StoreError> {
        let db_dir = root.join("db");
        fs::create_dir_all(&db_dir)?;
        
        Ok(Self {
            root: db_dir,
            cache: HashMap::new(),
        })
    }

    /// Get the path to the info file for a store path.
    fn info_path(&self, store_path: &StorePath) -> PathBuf {
        self.root.join(format!("{}.json", store_path.hash()))
    }

    /// Register a path in the database.
    pub fn register(&mut self, info: PathInfo) -> Result<(), StoreError> {
        let path = self.info_path(&info.path);
        let json = serde_json::to_string_pretty(&info)?;
        fs::write(&path, json)?;
        self.cache.insert(info.path.clone(), info);
        Ok(())
    }

    /// Query path info.
    pub fn query(&mut self, store_path: &StorePath) -> Result<Option<PathInfo>, StoreError> {
        if let Some(info) = self.cache.get(store_path) {
            return Ok(Some(info.clone()));
        }
        
        let path = self.info_path(store_path);
        if !path.exists() {
            return Ok(None);
        }
        
        let json = fs::read_to_string(&path)?;
        let info: PathInfo = serde_json::from_str(&json)?;
        self.cache.insert(store_path.clone(), info.clone());
        
        Ok(Some(info))
    }

    /// Check if a path is valid (registered and exists).
    pub fn is_valid(&mut self, store_path: &StorePath) -> Result<bool, StoreError> {
        Ok(self.query(store_path)?.map(|i| i.valid).unwrap_or(false))
    }

    /// Get all references of a path.
    pub fn get_references(&mut self, store_path: &StorePath) -> Result<HashSet<StorePath>, StoreError> {
        Ok(self.query(store_path)?
            .map(|i| i.references)
            .unwrap_or_default())
    }

    /// Get paths that reference the given path (referrers).
    pub fn get_referrers(&mut self, store_path: &StorePath) -> Result<HashSet<StorePath>, StoreError> {
        let mut referrers = HashSet::new();
        
        // Scan all info files (inefficient, but simple)
        if !self.root.exists() {
            return Ok(referrers);
        }
        
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|e| e == "json") {
                let json = fs::read_to_string(entry.path())?;
                if let Ok(info) = serde_json::from_str::<PathInfo>(&json)
                    && info.references.contains(store_path) {
                        referrers.insert(info.path);
                    }
            }
        }
        
        Ok(referrers)
    }

    /// Delete path info from the database.
    pub fn delete(&mut self, store_path: &StorePath) -> Result<(), StoreError> {
        let path = self.info_path(store_path);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        self.cache.remove(store_path);
        Ok(())
    }

    /// Invalidate a path (mark as not valid).
    pub fn invalidate(&mut self, store_path: &StorePath) -> Result<(), StoreError> {
        if let Some(mut info) = self.query(store_path)? {
            info.valid = false;
            self.register(info)?;
        }
        Ok(())
    }

    /// List all registered paths.
    pub fn list_all(&self) -> Result<Vec<StorePath>, StoreError> {
        let mut paths = Vec::new();
        
        if !self.root.exists() {
            return Ok(paths);
        }
        
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|e| e == "json") {
                let json = fs::read_to_string(entry.path())?;
                if let Ok(info) = serde_json::from_str::<PathInfo>(&json) {
                    paths.push(info.path);
                }
            }
        }
        
        Ok(paths)
    }
}

/// Get current Unix timestamp.
fn current_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_db(suffix: &str) -> Database {
        let dir = env::temp_dir().join(format!("neve-db-test-{}-{}", std::process::id(), suffix));
        let _ = fs::remove_dir_all(&dir); // Clean up any previous run
        Database::open(dir).unwrap()
    }

    #[test]
    fn test_register_and_query() {
        let mut db = temp_db("register");
        
        let hash = Hash::of(b"test content");
        let store_path = StorePath::new(hash, "test-1.0".to_string());
        let info = PathInfo::new(store_path.clone(), hash, 1024);
        
        db.register(info.clone()).unwrap();
        
        let queried = db.query(&store_path).unwrap();
        assert!(queried.is_some());
        let queried = queried.unwrap();
        assert_eq!(queried.nar_size, 1024);
        
        // Cleanup
        let _ = fs::remove_dir_all(&db.root);
    }

    #[test]
    fn test_references() {
        let mut db = temp_db("refs");
        
        let hash1 = Hash::of(b"path1");
        let hash2 = Hash::of(b"path2");
        let path1 = StorePath::new(hash1, "pkg1-1.0".to_string());
        let path2 = StorePath::new(hash2, "pkg2-1.0".to_string());
        
        let mut info1 = PathInfo::new(path1.clone(), hash1, 100);
        info1.add_reference(path2.clone());
        db.register(info1).unwrap();
        
        let refs = db.get_references(&path1).unwrap();
        assert!(refs.contains(&path2));
        
        // Cleanup
        let _ = fs::remove_dir_all(&db.root);
    }
}
