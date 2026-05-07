//! `neve test` — find and run test files.

use crate::output;
use std::path::Path;
use std::process::Command;

pub fn run(dir: &str, verbose: bool) -> Result<(), String> {
    let root = Path::new(dir);
    if !root.is_dir() {
        return Err(format!("not a directory: {}", dir));
    }

    // Find test files: *_test.neve or test/*.neve
    let test_dir = root.join("test");
    let mut test_files = Vec::new();

    // Find *_test.neve in root
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with("_test.neve") {
                test_files.push(entry.path());
            }
        }
    }

    // Find test/*.neve
    if test_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&test_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "neve").unwrap_or(false) {
                    test_files.push(path);
                }
            }
        }
    }

    if test_files.is_empty() {
        output::info("no test files found (*_test.neve or test/*.neve)");
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
            std::env::current_exe().unwrap_or_else(|_| Path::new("neve").to_path_buf()),
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
