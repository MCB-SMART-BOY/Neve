//! Integration tests for neve-fetch crate.

use std::env;
use std::fs;
use std::path::PathBuf;
use neve_fetch::Source;
use neve_fetch::archive::ArchiveFormat;
use neve_fetch::verify::{verify_content, verify_file, hash_dir};
use neve_derive::Hash;

// Archive format tests

#[test]
fn test_format_detection() {
    assert_eq!(
        ArchiveFormat::from_name("foo.tar.gz"),
        Some(ArchiveFormat::TarGz)
    );
    assert_eq!(
        ArchiveFormat::from_name("foo.tgz"),
        Some(ArchiveFormat::TarGz)
    );
    assert_eq!(
        ArchiveFormat::from_name("foo.tar.xz"),
        Some(ArchiveFormat::TarXz)
    );
    assert_eq!(
        ArchiveFormat::from_name("foo.tar"),
        Some(ArchiveFormat::Tar)
    );
    assert_eq!(
        ArchiveFormat::from_name("foo.zip"),
        None
    );
}

// Source tests

#[test]
fn test_source_builder() {
    let source = Source::url("https://example.com/file.tar.gz")
        .with_name("my-source");

    match source {
        Source::Url { url, name, .. } => {
            assert_eq!(url, "https://example.com/file.tar.gz");
            assert_eq!(name, Some("my-source".to_string()));
        }
        _ => panic!("expected Url source"),
    }
}

#[test]
fn test_source_path() {
    let source = Source::path("/tmp/test.txt");
    match source {
        Source::Path { path, .. } => {
            assert_eq!(path, PathBuf::from("/tmp/test.txt"));
        }
        _ => panic!("expected Path source"),
    }
}

#[test]
fn test_source_git() {
    let source = Source::git("https://github.com/user/repo.git", "main");
    
    match source {
        Source::Git { url, rev, .. } => {
            assert_eq!(url, "https://github.com/user/repo.git");
            assert_eq!(rev, "main");
        }
        _ => panic!("expected Git source"),
    }
}

// Verify tests

#[test]
fn test_verify_content() {
    let content = b"hello world";
    let hash = Hash::of(content);

    assert!(verify_content(content, &hash).is_ok());

    let wrong_hash = Hash::of(b"different content");
    assert!(verify_content(content, &wrong_hash).is_err());
}

#[test]
fn test_verify_file() {
    let dir = env::temp_dir().join(format!("neve-verify-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();

    let file_path = dir.join("test.txt");
    let content = b"test content";
    fs::write(&file_path, content).unwrap();

    let hash = Hash::of(content);
    assert!(verify_file(&file_path, &hash).is_ok());

    // Cleanup
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_hash_dir() {
    let dir = env::temp_dir().join(format!("neve-hash-dir-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("a.txt"), b"aaa").unwrap();
    fs::write(dir.join("b.txt"), b"bbb").unwrap();

    let hash1 = hash_dir(&dir).unwrap();
    let hash2 = hash_dir(&dir).unwrap();

    // Same content should produce same hash
    assert_eq!(hash1, hash2);

    // Different content should produce different hash
    fs::write(dir.join("c.txt"), b"ccc").unwrap();
    let hash3 = hash_dir(&dir).unwrap();
    assert_ne!(hash1, hash3);

    // Cleanup
    let _ = fs::remove_dir_all(&dir);
}

// Network tests (ignored by default)

#[test]
#[ignore]
fn test_fetch_url() {
    use neve_fetch::url::fetch_url;
    let content = fetch_url("https://httpbin.org/bytes/100").unwrap();
    assert_eq!(content.len(), 100);
}

#[test]
#[ignore]
fn test_clone_repo() {
    use neve_fetch::git::clone_repo;
    use tempfile::TempDir;
    
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("repo");

    let result = clone_repo(
        "https://github.com/octocat/Hello-World.git",
        &repo_path,
    );

    assert!(result.is_ok());
    assert!(repo_path.join(".git").exists());
}
