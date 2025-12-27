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

// ============================================================================
// BuilderConfig 边缘测试
// ============================================================================

#[test]
fn test_builder_config_zero_cores() {
    let config = BuilderConfig {
        cores: 0,
        max_jobs: 1,
        ..Default::default()
    };
    // Should still work even with 0 cores (edge case)
    assert_eq!(config.cores, 0);
}

#[test]
fn test_builder_config_many_jobs() {
    let config = BuilderConfig {
        cores: 8,
        max_jobs: 100,
        ..Default::default()
    };
    assert_eq!(config.max_jobs, 100);
}

#[test]
fn test_builder_config_single_core_single_job() {
    let config = BuilderConfig {
        cores: 1,
        max_jobs: 1,
        ..Default::default()
    };
    assert_eq!(config.cores, 1);
    assert_eq!(config.max_jobs, 1);
}

// ============================================================================
// Output format_size 边缘测试
// ============================================================================

#[test]
fn test_format_size_one_byte() {
    assert_eq!(format_size(1), "1 B");
}

#[test]
fn test_format_size_just_under_kib() {
    assert_eq!(format_size(1023), "1023 B");
}

#[test]
fn test_format_size_exactly_kib() {
    assert_eq!(format_size(1024), "1.00 KiB");
}

#[test]
fn test_format_size_just_over_kib() {
    assert_eq!(format_size(1025), "1.00 KiB");
}

#[test]
fn test_format_size_exactly_mib() {
    assert_eq!(format_size(1024 * 1024), "1.00 MiB");
}

#[test]
fn test_format_size_exactly_gib() {
    assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GiB");
}

#[test]
fn test_format_size_large_value() {
    // 10 GiB
    let size = 10 * 1024 * 1024 * 1024;
    let formatted = format_size(size);
    assert!(formatted.contains("GiB"));
}

#[test]
fn test_format_size_fractional_kib() {
    // 1.5 KiB = 1536 bytes
    let formatted = format_size(1536);
    assert!(formatted.contains("KiB"));
}

#[test]
fn test_format_size_fractional_mib() {
    // 2.5 MiB
    let size = (2.5 * 1024.0 * 1024.0) as u64;
    let formatted = format_size(size);
    assert!(formatted.contains("MiB"));
}

// ============================================================================
// output_size 边缘测试
// ============================================================================

#[test]
fn test_output_size_empty_directory() {
    let dir = env::temp_dir().join(format!("neve-output-empty-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    
    let size = output_size(&dir).unwrap();
    assert_eq!(size, 0);
    
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_output_size_single_file() {
    let dir = env::temp_dir().join(format!("neve-output-single-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("file.txt"), b"12345").unwrap();
    
    let size = output_size(&dir).unwrap();
    assert_eq!(size, 5);
    
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_output_size_multiple_files() {
    let dir = env::temp_dir().join(format!("neve-output-multi-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("a.txt"), b"aaa").unwrap(); // 3 bytes
    fs::write(dir.join("b.txt"), b"bbbbb").unwrap(); // 5 bytes
    
    let size = output_size(&dir).unwrap();
    assert_eq!(size, 8);
    
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_output_size_nested_directories() {
    let dir = env::temp_dir().join(format!("neve-output-nested-{}", std::process::id()));
    fs::create_dir_all(dir.join("subdir")).unwrap();
    fs::write(dir.join("root.txt"), b"root").unwrap(); // 4 bytes
    fs::write(dir.join("subdir/nested.txt"), b"nested").unwrap(); // 6 bytes
    
    let size = output_size(&dir).unwrap();
    assert_eq!(size, 10);
    
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_output_size_deeply_nested() {
    let dir = env::temp_dir().join(format!("neve-output-deep-{}", std::process::id()));
    fs::create_dir_all(dir.join("a/b/c/d")).unwrap();
    fs::write(dir.join("a/b/c/d/deep.txt"), b"deep content").unwrap(); // 12 bytes
    
    let size = output_size(&dir).unwrap();
    assert_eq!(size, 12);
    
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_output_size_binary_file() {
    let dir = env::temp_dir().join(format!("neve-output-binary-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("binary.bin"), [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap();
    
    let size = output_size(&dir).unwrap();
    assert_eq!(size, 10);
    
    let _ = fs::remove_dir_all(&dir);
}

// ============================================================================
// SandboxConfig 边缘测试
// ============================================================================

#[test]
fn test_sandbox_config_paths() {
    let root = env::temp_dir().join("sandbox-paths-test");
    let config = SandboxConfig::new(root.clone());
    
    assert!(config.build_dir.starts_with(&root));
    assert!(config.output_dir.starts_with(&root));
    assert_ne!(config.build_dir, config.output_dir);
}

#[test]
fn test_sandbox_config_network_default_off() {
    let root = env::temp_dir().join("sandbox-net-default");
    let config = SandboxConfig::new(root);
    
    assert!(!config.network);
}

#[test]
fn test_sandbox_config_with_special_path() {
    let root = env::temp_dir().join("sandbox-special-chars-!@#");
    let config = SandboxConfig::new(root.clone());
    
    assert!(config.build_dir.to_string_lossy().contains("!@#"));
}

// ============================================================================
// Sandbox 边缘测试
// ============================================================================

#[test]
fn test_sandbox_directories_exist_after_create() {
    let root = env::temp_dir().join(format!("neve-sandbox-exist-{}", std::process::id()));
    let config = SandboxConfig::new(root.clone());
    
    let sandbox = Sandbox::new(config).unwrap();
    
    assert!(sandbox.build_dir().exists());
    assert!(sandbox.build_dir().is_dir());
    assert!(sandbox.output_dir().exists());
    assert!(sandbox.output_dir().is_dir());
    
    sandbox.cleanup().unwrap();
}

#[test]
fn test_sandbox_cleanup_removes_all() {
    let root = env::temp_dir().join(format!("neve-sandbox-cleanup-{}", std::process::id()));
    let config = SandboxConfig::new(root.clone());
    
    let sandbox = Sandbox::new(config).unwrap();
    
    // Create some files in the sandbox
    fs::write(sandbox.build_dir().join("test.txt"), b"test").unwrap();
    fs::write(sandbox.output_dir().join("output.txt"), b"output").unwrap();
    
    sandbox.cleanup().unwrap();
    
    assert!(!root.exists());
}

#[test]
fn test_sandbox_cleanup_handles_nested_files() {
    let root = env::temp_dir().join(format!("neve-sandbox-nested-clean-{}", std::process::id()));
    let config = SandboxConfig::new(root.clone());
    
    let sandbox = Sandbox::new(config).unwrap();
    
    // Create nested structure
    let nested = sandbox.build_dir().join("a/b/c");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("deep.txt"), b"deep").unwrap();
    
    sandbox.cleanup().unwrap();
    
    assert!(!root.exists());
}

#[test]
fn test_sandbox_multiple_instances() {
    let root1 = env::temp_dir().join(format!("neve-sandbox-multi1-{}", std::process::id()));
    let root2 = env::temp_dir().join(format!("neve-sandbox-multi2-{}", std::process::id()));
    
    let sandbox1 = Sandbox::new(SandboxConfig::new(root1.clone())).unwrap();
    let sandbox2 = Sandbox::new(SandboxConfig::new(root2.clone())).unwrap();
    
    // Both should exist independently
    assert!(sandbox1.build_dir().exists());
    assert!(sandbox2.build_dir().exists());
    
    // Paths should be different
    assert_ne!(sandbox1.build_dir(), sandbox2.build_dir());
    
    sandbox1.cleanup().unwrap();
    sandbox2.cleanup().unwrap();
}

// ============================================================================
// IsolationLevel 边缘测试
// ============================================================================

#[test]
fn test_isolation_level_variants() {
    // Test that we can create both variants
    let full = IsolationLevel::Full;
    let basic = IsolationLevel::Basic;
    
    assert!(full == IsolationLevel::Full);
    assert!(basic == IsolationLevel::Basic);
    assert!(full != basic);
}

#[test]
fn test_isolation_level_best_available_is_valid() {
    let level = IsolationLevel::best_available();
    
    // Should be one of the valid variants
    assert!(level == IsolationLevel::Full || level == IsolationLevel::Basic);
}

// ============================================================================
// 压力测试
// ============================================================================

#[test]
fn test_output_size_many_small_files() {
    let dir = env::temp_dir().join(format!("neve-output-many-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    
    // Create 100 small files
    for i in 0..100 {
        fs::write(dir.join(format!("file{}.txt", i)), b"x").unwrap();
    }
    
    let size = output_size(&dir).unwrap();
    assert_eq!(size, 100);
    
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_sandbox_rapid_create_cleanup() {
    for i in 0..5 {
        let root = env::temp_dir().join(format!("neve-sandbox-rapid-{}-{}", std::process::id(), i));
        let config = SandboxConfig::new(root.clone());
        
        let sandbox = Sandbox::new(config).unwrap();
        assert!(sandbox.build_dir().exists());
        
        sandbox.cleanup().unwrap();
        assert!(!root.exists());
    }
}
