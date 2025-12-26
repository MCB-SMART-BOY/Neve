//! Integration tests for neve-builder crate.

use std::env;
use std::fs;
use neve_builder::BuilderConfig;
use neve_builder::sandbox::{SandboxConfig, Sandbox, IsolationLevel};
use neve_builder::output::{format_size, output_size};

// Config tests

#[test]
fn test_builder_config_default() {
    let config = BuilderConfig::default();
    assert!(config.cores >= 1);
    assert_eq!(config.max_jobs, 1);
}

#[test]
fn test_builder_config_custom() {
    let config = BuilderConfig {
        cores: 4,
        max_jobs: 2,
        ..Default::default()
    };
    assert_eq!(config.cores, 4);
    assert_eq!(config.max_jobs, 2);
}

// Output tests

#[test]
fn test_format_size() {
    assert_eq!(format_size(100), "100 B");
    assert_eq!(format_size(1024), "1.00 KiB");
    assert_eq!(format_size(1024 * 1024), "1.00 MiB");
    assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GiB");
}

#[test]
fn test_format_size_edge_cases() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(1023), "1023 B");
}

#[test]
fn test_output_size() {
    let dir = env::temp_dir().join(format!("neve-output-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("test.txt"), b"hello world").unwrap();

    let size = output_size(&dir).unwrap();
    assert_eq!(size, 11); // "hello world" is 11 bytes

    // Cleanup
    let _ = fs::remove_dir_all(&dir);
}

// Sandbox tests

#[test]
fn test_sandbox_config() {
    let root = env::temp_dir().join("neve-sandbox-test");
    let config = SandboxConfig::new(root.clone());

    assert_eq!(config.build_dir, root.join("build"));
    assert_eq!(config.output_dir, root.join("output"));
    assert!(!config.network);
}

#[test]
fn test_sandbox_create() {
    let root = env::temp_dir().join(format!("neve-sandbox-test-{}", std::process::id()));
    let config = SandboxConfig::new(root.clone());

    let sandbox = Sandbox::new(config).unwrap();
    assert!(sandbox.build_dir().exists());
    assert!(sandbox.output_dir().exists());

    sandbox.cleanup().unwrap();
    assert!(!root.exists());
}

#[test]
fn test_isolation_level() {
    let level = IsolationLevel::best_available();
    // Should be at least Basic
    assert!(level == IsolationLevel::Full || level == IsolationLevel::Basic);
}

#[test]
fn test_sandbox_with_network() {
    let root = env::temp_dir().join(format!("neve-sandbox-net-{}", std::process::id()));
    let mut config = SandboxConfig::new(root.clone());
    config.network = true;

    assert!(config.network);

    // Cleanup
    let _ = fs::remove_dir_all(&root);
}
