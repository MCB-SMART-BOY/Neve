//! Integration tests for neve-store crate.

use std::env;
use std::fs;
use neve_store::{Store, Database, PathInfo, GcResult, store_dir};
use neve_derive::{Derivation, Hash, StorePath, Output};

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

// ============================================================================
// Hash 边缘测试
// ============================================================================

#[test]
fn test_hash_empty_content() {
    let hash = Hash::of(b"");
    // Should still produce a valid hash
    assert!(!hash.to_string().is_empty());
}

#[test]
fn test_hash_single_byte() {
    let hash = Hash::of(b"x");
    assert!(!hash.to_string().is_empty());
}

#[test]
fn test_hash_same_content_same_hash() {
    let hash1 = Hash::of(b"hello world");
    let hash2 = Hash::of(b"hello world");
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_different_content_different_hash() {
    let hash1 = Hash::of(b"hello");
    let hash2 = Hash::of(b"world");
    assert_ne!(hash1, hash2);
}

#[test]
fn test_hash_case_sensitive() {
    let hash1 = Hash::of(b"Hello");
    let hash2 = Hash::of(b"hello");
    assert_ne!(hash1, hash2);
}

#[test]
fn test_hash_whitespace_matters() {
    let hash1 = Hash::of(b"hello world");
    let hash2 = Hash::of(b"helloworld");
    assert_ne!(hash1, hash2);
}

#[test]
fn test_hash_binary_content() {
    let hash = Hash::of(&[0u8, 1, 2, 3, 255, 254, 253]);
    assert!(!hash.to_string().is_empty());
}

#[test]
fn test_hash_large_content() {
    let large = vec![42u8; 1024 * 1024]; // 1 MiB
    let hash = Hash::of(&large);
    assert!(!hash.to_string().is_empty());
}

// ============================================================================
// StorePath 边缘测试
// ============================================================================

#[test]
fn test_store_path_simple_name() {
    let hash = Hash::of(b"test");
    let path = StorePath::new(hash, "simple".to_string());
    assert!(path.display_name().contains("simple"));
}

#[test]
fn test_store_path_with_version() {
    let hash = Hash::of(b"test");
    let path = StorePath::new(hash, "package-1.2.3".to_string());
    assert!(path.display_name().contains("package-1.2.3"));
}

#[test]
fn test_store_path_with_dots() {
    let hash = Hash::of(b"test");
    let path = StorePath::new(hash, "my.package.name".to_string());
    assert!(path.display_name().contains("my.package.name"));
}

#[test]
fn test_store_path_with_underscores() {
    let hash = Hash::of(b"test");
    let path = StorePath::new(hash, "my_package_name".to_string());
    assert!(path.display_name().contains("my_package_name"));
}

#[test]
fn test_store_path_equality() {
    let hash = Hash::of(b"same");
    let path1 = StorePath::new(hash, "pkg".to_string());
    let path2 = StorePath::new(hash, "pkg".to_string());
    assert_eq!(path1, path2);
}

#[test]
fn test_store_path_inequality_different_hash() {
    let hash1 = Hash::of(b"content1");
    let hash2 = Hash::of(b"content2");
    let path1 = StorePath::new(hash1, "pkg".to_string());
    let path2 = StorePath::new(hash2, "pkg".to_string());
    assert_ne!(path1, path2);
}

#[test]
fn test_store_path_inequality_different_name() {
    let hash = Hash::of(b"same");
    let path1 = StorePath::new(hash, "pkg1".to_string());
    let path2 = StorePath::new(hash, "pkg2".to_string());
    assert_ne!(path1, path2);
}

// ============================================================================
// Store 边缘测试
// ============================================================================

#[test]
fn test_store_add_empty_content() {
    let store = temp_store("empty-content");
    let path = store.add_content(b"", "empty.txt").unwrap();
    
    assert!(store.path_exists(&path));
    
    let _ = fs::remove_dir_all(store.root());
}

#[test]
fn test_store_add_binary_content() {
    let store = temp_store("binary-content");
    let binary = vec![0u8, 1, 2, 255, 254, 253];
    let path = store.add_content(&binary, "binary.bin").unwrap();
    
    let fs_path = store.to_path(&path);
    let read_content = fs::read(&fs_path).unwrap();
    assert_eq!(read_content, binary);
    
    let _ = fs::remove_dir_all(store.root());
}

#[test]
fn test_store_add_large_content() {
    let store = temp_store("large-content");
    let large = vec![42u8; 10000]; // 10 KB
    let path = store.add_content(&large, "large.bin").unwrap();
    
    let fs_path = store.to_path(&path);
    let read_content = fs::read(&fs_path).unwrap();
    assert_eq!(read_content.len(), 10000);
    
    let _ = fs::remove_dir_all(store.root());
}

#[test]
fn test_store_path_does_not_exist() {
    let store = temp_store("nonexistent");
    let hash = Hash::of(b"nonexistent");
    let fake_path = StorePath::new(hash, "fake-1.0".to_string());
    
    assert!(!store.path_exists(&fake_path));
    
    let _ = fs::remove_dir_all(store.root());
}

#[test]
fn test_store_multiple_files() {
    let store = temp_store("multi-files");
    
    let path1 = store.add_content(b"content1", "file1.txt").unwrap();
    let path2 = store.add_content(b"content2", "file2.txt").unwrap();
    let path3 = store.add_content(b"content3", "file3.txt").unwrap();
    
    assert!(store.path_exists(&path1));
    assert!(store.path_exists(&path2));
    assert!(store.path_exists(&path3));
    
    assert_ne!(path1, path2);
    assert_ne!(path2, path3);
    
    let _ = fs::remove_dir_all(store.root());
}

#[test]
fn test_store_derivation_minimal() {
    let mut store = temp_store("drv-minimal");
    let drv = Derivation::builder("minimal", "0.0.1")
        .system("x86_64-linux")
        .build();
    
    let path = store.add_derivation(&drv).unwrap();
    assert!(store.path_exists(&path));
    
    let _ = fs::remove_dir_all(store.root());
}

#[test]
fn test_store_derivation_with_outputs() {
    let mut store = temp_store("drv-outputs");
    let drv = Derivation::builder("with-outputs", "1.0")
        .system("x86_64-linux")
        .output(Output::new("out"))
        .output(Output::new("lib"))
        .output(Output::new("dev"))
        .build();
    
    let path = store.add_derivation(&drv).unwrap();
    let read_drv = store.read_derivation(&path).unwrap();
    
    assert_eq!(read_drv.outputs.len(), 3);
    
    let _ = fs::remove_dir_all(store.root());
}

#[test]
fn test_store_derivation_with_env() {
    let mut store = temp_store("drv-env");
    let drv = Derivation::builder("with-env", "1.0")
        .system("x86_64-linux")
        .env("CC", "/usr/bin/gcc")
        .env("CFLAGS", "-O2")
        .build();
    
    let path = store.add_derivation(&drv).unwrap();
    let read_drv = store.read_derivation(&path).unwrap();
    
    assert!(read_drv.env.contains_key("CC"));
    assert!(read_drv.env.contains_key("CFLAGS"));
    
    let _ = fs::remove_dir_all(store.root());
}

// ============================================================================
// Database 边缘测试
// ============================================================================

#[test]
fn test_database_query_nonexistent() {
    let mut db = temp_db("query-none");
    let hash = Hash::of(b"nonexistent");
    let path = StorePath::new(hash, "fake".to_string());
    
    let result = db.query(&path).unwrap();
    assert!(result.is_none());
    
    let _ = fs::remove_dir_all(db.root());
}

#[test]
fn test_database_multiple_entries() {
    let mut db = temp_db("multi-entries");
    
    for i in 0..10 {
        let hash = Hash::of(format!("content-{}", i).as_bytes());
        let path = StorePath::new(hash, format!("pkg-{}", i));
        let info = PathInfo::new(path, hash, 100 * i);
        db.register(info).unwrap();
    }
    
    // Query a few
    let hash5 = Hash::of(b"content-5");
    let path5 = StorePath::new(hash5, "pkg-5".to_string());
    let result = db.query(&path5).unwrap();
    
    assert!(result.is_some());
    assert_eq!(result.unwrap().nar_size, 500);
    
    let _ = fs::remove_dir_all(db.root());
}

#[test]
fn test_database_zero_size() {
    let mut db = temp_db("zero-size");
    
    let hash = Hash::of(b"empty");
    let path = StorePath::new(hash, "empty-pkg".to_string());
    let info = PathInfo::new(path.clone(), hash, 0);
    
    db.register(info).unwrap();
    
    let result = db.query(&path).unwrap().unwrap();
    assert_eq!(result.nar_size, 0);
    
    let _ = fs::remove_dir_all(db.root());
}

#[test]
fn test_database_large_size() {
    let mut db = temp_db("large-size");
    
    let hash = Hash::of(b"large");
    let path = StorePath::new(hash, "large-pkg".to_string());
    let info = PathInfo::new(path.clone(), hash, u64::MAX);
    
    db.register(info).unwrap();
    
    let result = db.query(&path).unwrap().unwrap();
    assert_eq!(result.nar_size, u64::MAX);
    
    let _ = fs::remove_dir_all(db.root());
}

#[test]
fn test_database_multiple_references() {
    let mut db = temp_db("multi-refs");
    
    let hash1 = Hash::of(b"pkg1");
    let hash2 = Hash::of(b"pkg2");
    let hash3 = Hash::of(b"pkg3");
    
    let path1 = StorePath::new(hash1, "pkg1".to_string());
    let path2 = StorePath::new(hash2, "pkg2".to_string());
    let path3 = StorePath::new(hash3, "pkg3".to_string());
    
    let mut info1 = PathInfo::new(path1.clone(), hash1, 100);
    info1.add_reference(path2.clone());
    info1.add_reference(path3.clone());
    
    db.register(info1).unwrap();
    
    let refs = db.get_references(&path1).unwrap();
    assert_eq!(refs.len(), 2);
    assert!(refs.contains(&path2));
    assert!(refs.contains(&path3));
    
    let _ = fs::remove_dir_all(db.root());
}

#[test]
fn test_database_no_references() {
    let mut db = temp_db("no-refs");
    
    let hash = Hash::of(b"standalone");
    let path = StorePath::new(hash, "standalone".to_string());
    let info = PathInfo::new(path.clone(), hash, 50);
    
    db.register(info).unwrap();
    
    let refs = db.get_references(&path).unwrap();
    assert!(refs.is_empty());
    
    let _ = fs::remove_dir_all(db.root());
}

// ============================================================================
// GcResult 边缘测试
// ============================================================================

#[test]
fn test_gc_result_one_byte() {
    let result = GcResult { deleted: 1, freed_bytes: 1 };
    let human = result.freed_human();
    assert!(human.contains("B"));
}

#[test]
fn test_gc_result_kib() {
    let result = GcResult { deleted: 10, freed_bytes: 1024 };
    let human = result.freed_human();
    assert!(human.contains("KiB"));
}

#[test]
fn test_gc_result_mib() {
    let result = GcResult { deleted: 50, freed_bytes: 1024 * 1024 };
    let human = result.freed_human();
    assert!(human.contains("MiB"));
}

#[test]
fn test_gc_result_gib() {
    let result = GcResult { deleted: 100, freed_bytes: 1024 * 1024 * 1024 };
    let human = result.freed_human();
    assert!(human.contains("GiB"));
}

#[test]
fn test_gc_result_large_deleted_count() {
    let result = GcResult { deleted: 1000000, freed_bytes: 1024 };
    assert_eq!(result.deleted, 1000000);
}

// ============================================================================
// PathInfo 边缘测试
// ============================================================================

#[test]
fn test_path_info_new() {
    let hash = Hash::of(b"test");
    let path = StorePath::new(hash, "test-pkg".to_string());
    let info = PathInfo::new(path.clone(), hash, 12345);
    
    assert_eq!(info.path, path);
    assert_eq!(info.nar_hash, hash);
    assert_eq!(info.nar_size, 12345);
}

#[test]
fn test_path_info_add_multiple_references() {
    let hash = Hash::of(b"main");
    let path = StorePath::new(hash, "main-pkg".to_string());
    let mut info = PathInfo::new(path, hash, 100);
    
    for i in 0..5 {
        let ref_hash = Hash::of(format!("ref-{}", i).as_bytes());
        let ref_path = StorePath::new(ref_hash, format!("ref-{}", i));
        info.add_reference(ref_path);
    }
    
    assert_eq!(info.references.len(), 5);
}

// ============================================================================
// store_dir 边缘测试
// ============================================================================

#[test]
fn test_store_dir_returns_path() {
    let dir = store_dir();
    assert!(!dir.as_os_str().is_empty());
    // Should be an absolute path
    assert!(dir.is_absolute() || dir.to_string_lossy().starts_with('/') || dir.to_string_lossy().contains(':'));
}

#[test]
fn test_store_dir_consistent() {
    let dir1 = store_dir();
    let dir2 = store_dir();
    assert_eq!(dir1, dir2);
}

// ============================================================================
// 压力测试
// ============================================================================

#[test]
fn test_store_many_files() {
    let store = temp_store("stress-many");
    
    for i in 0..50 {
        let content = format!("content-{}", i);
        let name = format!("file-{}.txt", i);
        let path = store.add_content(content.as_bytes(), &name).unwrap();
        assert!(store.path_exists(&path));
    }
    
    let _ = fs::remove_dir_all(store.root());
}

#[test]
fn test_database_many_entries() {
    let mut db = temp_db("stress-db");
    
    for i in 0..100 {
        let hash = Hash::of(format!("entry-{}", i).as_bytes());
        let path = StorePath::new(hash, format!("entry-{}", i));
        let info = PathInfo::new(path, hash, i as u64 * 100);
        db.register(info).unwrap();
    }
    
    // Verify a few entries
    for i in [0, 25, 50, 75, 99] {
        let hash = Hash::of(format!("entry-{}", i).as_bytes());
        let path = StorePath::new(hash, format!("entry-{}", i));
        let result = db.query(&path).unwrap();
        assert!(result.is_some());
    }
    
    let _ = fs::remove_dir_all(db.root());
}
