//! `n3v3 test` — find and run test files.

use crate::output;
use std::path::Path;
use std::process::Command;

/// Discover and run all n3v3 test files under `dir` through the canonical HIR pipeline.
/// 发现并运行 `dir` 下所有 n3v3 测试文件（通过 canonical HIR 管线）。
pub fn run(dir: &str, verbose: bool) -> Result<(), String> {
    let root = Path::new(dir);
    if !root.is_dir() {
        return Err(format!("not a directory: {}", dir));
    }

    // Find test files: *_test.n3v3 or test/*.n3v3
    let test_dir = root.join("test");
    let mut test_files = Vec::new();

    // Find *_test.n3v3 in root
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with("_test.n3v3") {
                test_files.push(entry.path());
            }
        }
    }

    // Find test/*.n3v3
    if test_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&test_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "n3v3").unwrap_or(false) {
                test_files.push(path);
            }
        }
    }

    if test_files.is_empty() {
        output::info("no test files found (*_test.n3v3 or test/*.n3v3)");
        return Ok(());
    }

    test_files.sort();
    let mut passed = 0usize;
    let mut failed = 0usize;

    for file in &test_files {
        let display = file.display().to_string();
        if verbose {
            output::info(&format!("running {}", display));
        }

        let status = Command::new(
            std::env::current_exe().unwrap_or_else(|_| Path::new("n3v3").to_path_buf()),
        )
        .arg("run")
        .arg(file)
        .status()
        .map_err(|e| format!("failed to run {}: {e}", display))?;

        if status.success() {
            passed += 1;
            if verbose {
                output::success(&format!("✓ {}", display));
            }
        } else {
            failed += 1;
            output::error(&format!("✗ {}", display));
        }
    }

    println!();
    println!(
        "test result: {} passed, {} failed, {} total",
        passed,
        failed,
        test_files.len()
    );

    if failed > 0 {
        Err(format!("{} test(s) failed", failed))
    } else {
        Ok(())
    }
}
