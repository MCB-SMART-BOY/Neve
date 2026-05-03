//! Integration tests for neve-builder crate.

use neve_builder::output::{format_size, output_size, validate_output};
use neve_builder::sandbox::{IsolationLevel, Sandbox, SandboxConfig};
use neve_builder::{BuildBackend, Builder, BuilderConfig};
use neve_derive::Derivation;
use neve_store::{Database, Store};
use std::env;
use std::fs;

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

#[cfg(unix)]
#[test]
fn test_output_size_does_not_follow_directory_symlink() {
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = env::temp_dir().join(format!(
        "neve-output-symlink-loop-{}-{}",
        std::process::id(),
        nonce
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("file.txt"), b"1234").unwrap();
    symlink(".", dir.join("loop")).unwrap();

    // Symlink should not be followed (otherwise this would recurse infinitely).
    let size = output_size(&dir).unwrap();
    assert_eq!(size, 4);

    let _ = fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn test_validate_output_does_not_recurse_into_directory_symlink() {
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = env::temp_dir().join(format!(
        "neve-validate-symlink-loop-{}-{}",
        std::process::id(),
        nonce
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("ok.txt"), b"ok").unwrap();
    symlink(".", dir.join("loop")).unwrap();

    // Should succeed without traversing into `loop`.
    validate_output(&dir).unwrap();

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

#[cfg(unix)]
#[test]
fn test_builder_links_real_input_output_path() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let store_root = env::temp_dir().join(format!(
        "neve-builder-input-store-{}-{}",
        std::process::id(),
        nonce
    ));
    let build_root = env::temp_dir().join(format!(
        "neve-builder-input-build-{}-{}",
        std::process::id(),
        nonce
    ));

    let mut store = Store::open_at(store_root.clone()).unwrap();

    let dep_drv = Derivation::builder("dep", "1.0")
        .builder_path("/bin/sh")
        .arg("-c")
        .arg("mkdir -p \"$out\"; echo dep > \"$out/dep.txt\"")
        .build();
    let dep_drv_path = store.add_derivation(&dep_drv).unwrap();

    let input_link_name = format!("{}-out", dep_drv_path.name());
    let main_script = format!(
        "test -f \"$NIX_BUILD_TOP/inputs/{}/dep.txt\"; mkdir -p \"$out\"; echo main > \"$out/main.txt\"",
        input_link_name
    );
    let main_drv = Derivation::builder("main", "1.0")
        .builder_path("/bin/sh")
        .arg("-c")
        .arg(main_script)
        .input_drv(dep_drv_path.clone(), vec!["out".to_string()])
        .build();

    let config = BuilderConfig {
        backend: BuildBackend::Simple,
        sandbox: false,
        temp_dir: build_root.clone(),
        ..Default::default()
    };

    let mut builder = Builder::with_config(store, config);
    let result = builder.build(&main_drv).unwrap();
    let out_path = result.outputs.get("out").unwrap();
    let out_fs_path = builder.store().to_path(out_path);
    assert!(out_fs_path.join("main.txt").exists());

    let _ = fs::remove_dir_all(store_root);
    let _ = fs::remove_dir_all(build_root);
}

#[cfg(unix)]
#[test]
fn test_builder_registers_output_metadata_with_references() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let store_root = env::temp_dir().join(format!(
        "neve-builder-db-store-{}-{}",
        std::process::id(),
        nonce
    ));
    let build_root = env::temp_dir().join(format!(
        "neve-builder-db-build-{}-{}",
        std::process::id(),
        nonce
    ));

    let mut store = Store::open_at(store_root.clone()).unwrap();
    let input_src = store.add_content(b"source-input", "src-1.0").unwrap();

    let dep_drv = Derivation::builder("dep", "1.0")
        .builder_path("/bin/sh")
        .arg("-c")
        .arg("mkdir -p \"$out\"; echo dep > \"$out/dep.txt\"")
        .build();
    let dep_drv_path = store.add_derivation(&dep_drv).unwrap();

    let dep_link = format!("{}-out", dep_drv_path.name());
    let src_link = input_src.name().to_string();
    let main_script = format!(
        "test -f \"$NIX_BUILD_TOP/inputs/{}/dep.txt\"; test -f \"$NIX_BUILD_TOP/inputs/{}\"; mkdir -p \"$out\"; echo main > \"$out/main.txt\"",
        dep_link, src_link
    );
    let main_drv = Derivation::builder("main", "1.0")
        .builder_path("/bin/sh")
        .arg("-c")
        .arg(main_script)
        .input_drv(dep_drv_path.clone(), vec!["out".to_string()])
        .input_src(input_src.clone())
        .build();

    let config = BuilderConfig {
        backend: BuildBackend::Simple,
        sandbox: false,
        temp_dir: build_root.clone(),
        ..Default::default()
    };

    let mut builder = Builder::with_config(store, config);
    let dep_result = builder.build(&dep_drv).unwrap();
    let dep_output = dep_result.outputs.get("out").unwrap().clone();

    let main_result = builder.build(&main_drv).unwrap();
    let main_output = main_result.outputs.get("out").unwrap().clone();

    let mut db = Database::open(store_root.clone()).unwrap();
    let info = db
        .query(&main_output)
        .unwrap()
        .expect("missing output metadata");
    assert_eq!(info.deriver.as_ref(), Some(&main_drv.drv_path()));
    assert!(info.references.contains(&input_src));
    assert!(info.references.contains(&dep_output));
    assert!(info.nar_size > 0);

    let _ = fs::remove_dir_all(store_root);
    let _ = fs::remove_dir_all(build_root);
}

// ============================================================================
// Phase B exit criteria: reproducibility
// ============================================================================

#[cfg(unix)]
#[test]
fn test_build_twice_produces_identical_store_paths() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let store_root =
        env::temp_dir().join(format!("neve-repro-store-{}-{}", std::process::id(), nonce));
    let build_root1 = env::temp_dir().join(format!(
        "neve-repro-build1-{}-{}",
        std::process::id(),
        nonce
    ));
    let build_root2 = env::temp_dir().join(format!(
        "neve-repro-build2-{}-{}",
        std::process::id(),
        nonce
    ));

    let drv = Derivation::builder("repro-test", "1.0")
        .builder_path("/bin/sh")
        .arg("-c")
        .arg("mkdir -p \"$out\"; echo deterministic > \"$out/output.txt\"")
        .build();

    let store1 = Store::open_at(store_root.clone()).unwrap();
    let config1 = BuilderConfig {
        backend: BuildBackend::Simple,
        sandbox: false,
        temp_dir: build_root1.clone(),
        ..Default::default()
    };
    let mut builder1 = Builder::with_config(store1, config1);
    let result1 = builder1.build(&drv).unwrap();
    let path1 = result1.outputs.get("out").unwrap().clone();

    let store2 = Store::open_at(store_root.clone()).unwrap();
    let config2 = BuilderConfig {
        backend: BuildBackend::Simple,
        sandbox: false,
        temp_dir: build_root2.clone(),
        ..Default::default()
    };
    let mut builder2 = Builder::with_config(store2, config2);
    let result2 = builder2.build(&drv).unwrap();
    let path2 = result2.outputs.get("out").unwrap().clone();

    assert_eq!(path1, path2, "build reproducibility failed");

    let store = Store::open_at(store_root.clone()).unwrap();
    let out_fs_path = store.to_path(&path1);
    assert!(out_fs_path.join("output.txt").exists());
    let content = fs::read_to_string(out_fs_path.join("output.txt")).unwrap();
    assert_eq!(content.trim(), "deterministic");

    let _ = fs::remove_dir_all(store_root);
    let _ = fs::remove_dir_all(build_root1);
    let _ = fs::remove_dir_all(build_root2);
}

#[cfg(unix)]
#[test]
fn test_gc_preserves_live_paths() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let store_root = env::temp_dir().join(format!("neve-gc-test-{}-{}", std::process::id(), nonce));

    let drv = Derivation::builder("gc-test", "1.0")
        .builder_path("/bin/sh")
        .arg("-c")
        .arg("mkdir -p \"$out\"; echo live > \"$out/data.txt\"")
        .build();

    let mut store = Store::open_at(store_root.clone()).unwrap();
    let _drv_path = store.add_derivation(&drv).unwrap();

    let config = BuilderConfig {
        backend: BuildBackend::Simple,
        sandbox: false,
        temp_dir: store_root.join("build"),
        ..Default::default()
    };
    let mut builder = Builder::with_config(store, config);
    let result = builder.build(&drv).unwrap();
    let out_path = result.outputs.get("out").unwrap();

    // Add GC root
    let mut store = Store::open_at(store_root.clone()).unwrap();
    let gc = neve_store::gc::GarbageCollector::new(&mut store);
    gc.add_root("test-gc-root", out_path).unwrap();

    // GC should preserve live path
    let mut store = Store::open_at(store_root.clone()).unwrap();
    let mut gc = neve_store::gc::GarbageCollector::new(&mut store);
    gc.collect().unwrap();

    let store = Store::open_at(store_root.clone()).unwrap();
    let out_fs = store.to_path(out_path);
    assert!(out_fs.join("data.txt").exists(), "GC deleted a live path");

    // Remove root and GC again
    let mut store = Store::open_at(store_root.clone()).unwrap();
    let gc = neve_store::gc::GarbageCollector::new(&mut store);
    gc.remove_root("test-gc-root").unwrap();

    let mut store = Store::open_at(store_root.clone()).unwrap();
    let mut gc = neve_store::gc::GarbageCollector::new(&mut store);
    gc.collect().unwrap();

    let _ = fs::remove_dir_all(store_root);
}
