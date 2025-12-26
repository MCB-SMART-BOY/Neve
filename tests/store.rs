//! Integration tests for neve-store crate.

use std::env;
use std::fs;
use neve_store::{Store, Database, PathInfo, GcResult, store_dir};
use neve_derive::{Derivation, Hash, StorePath};

fn temp_store(suffix: &str) -> Store {
    let dir = env::temp_dir().join(format!("neve-store-test-{}-{}", std::process::id(), suffix));
    let _ = fs::remove_dir_all(&dir); // Clean up any previous run
    Store::open_at(dir).unwrap()
}

fn temp_db(suffix: &str) -> Database {
    let dir = env::temp_dir().join(format!("neve-db-test-{}-{}", std::process::id(), suffix));
    let _ = fs::remove_dir_all(&dir); // Clean up any previous run
    Database::open(dir).unwrap()
}

// Path tests

#[test]
fn test_store_dir() {
    let dir = store_dir();
    assert!(!dir.as_os_str().is_empty());
}

// Store tests

#[test]
fn test_add_content() {
    let store = temp_store("content");
    let content = b"hello world";
    let path = store.add_content(content, "test.txt").unwrap();
    
    assert!(store.path_exists(&path));
    
    let fs_path = store.to_path(&path);
    let read_content = fs::read(&fs_path).unwrap();
    assert_eq!(read_content, content);
    
    // Cleanup
    let _ = fs::remove_dir_all(store.root());
}

#[test]
fn test_add_derivation() {
    let mut store = temp_store("drv");
    let drv = Derivation::builder("test", "1.0")
        .system("x86_64-linux")
        .build();
    
    let path = store.add_derivation(&drv).unwrap();
    assert!(store.path_exists(&path));
    
    let read_drv = store.read_derivation(&path).unwrap();
    assert_eq!(read_drv.name, drv.name);
    
    // Cleanup
    let _ = fs::remove_dir_all(store.root());
}

#[test]
fn test_store_content_addressable() {
    let store = temp_store("ca");
    let content = b"same content";
    
    let path1 = store.add_content(content, "file1.txt").unwrap();
    let path2 = store.add_content(content, "file1.txt").unwrap();
    
    // Same content should produce same path
    assert_eq!(path1, path2);
    
    // Cleanup
    let _ = fs::remove_dir_all(store.root());
}

// Database tests

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
    let _ = fs::remove_dir_all(db.root());
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
    let _ = fs::remove_dir_all(db.root());
}

// GC tests

#[test]
fn test_gc_result_human() {
    let result = GcResult { deleted: 5, freed_bytes: 1024 * 1024 * 100 };
    assert!(result.freed_human().contains("MiB"));
    
    let result = GcResult { deleted: 5, freed_bytes: 500 };
    assert!(result.freed_human().contains("B"));
}

#[test]
fn test_gc_result_zero() {
    let result = GcResult { deleted: 0, freed_bytes: 0 };
    assert_eq!(result.deleted, 0);
    assert!(result.freed_human().contains("0"));
}
