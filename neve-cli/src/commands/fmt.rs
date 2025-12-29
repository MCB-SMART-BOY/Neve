//! The `neve fmt` command.
//! `neve fmt` 命令。

use crate::output;
use std::fs;
use std::path::Path;

/// Format a Neve source file.
/// 格式化 Neve 源文件。
pub fn run(file: &str, write: bool) -> Result<(), String> {
    let path = Path::new(file);

    if !path.exists() {
        return Err(format!("File not found: {}", file));
    }

    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let formatted = neve_fmt::format(&source).map_err(|e| format!("Format error: {}", e))?;

    if write {
        if formatted != source {
            fs::write(path, &formatted).map_err(|e| format!("Failed to write file: {}", e))?;
            output::success(&format!("Formatted: {file}"));
        } else {
            output::info(&format!("Already formatted: {file}"));
        }
    } else {
        // Print the formatted code
        // 打印格式化后的代码
        print!("{}", formatted);
    }

    Ok(())
}

/// Check if a file is formatted.
/// 检查文件是否已格式化。
pub fn check(file: &str) -> Result<(), String> {
    let path = Path::new(file);

    if !path.exists() {
        return Err(format!("File not found: {}", file));
    }

    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let is_formatted = neve_fmt::check(&source).map_err(|e| format!("Format error: {}", e))?;

    if is_formatted {
        output::success(&format!("OK: {file}"));
        Ok(())
    } else {
        Err(format!("Would reformat: {file}"))
    }
}

/// Format all Neve files in a directory.
/// 格式化目录中的所有 Neve 文件。
pub fn format_dir(dir: &str, write: bool) -> Result<(), String> {
    let path = Path::new(dir);

    if !path.is_dir() {
        return Err(format!("Not a directory: {}", dir));
    }

    let mut errors = Vec::new();
    format_dir_recursive(path, write, &mut errors)?;

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("{} files would be reformatted", errors.len()))
    }
}

/// Recursively format all Neve files in a directory.
/// 递归格式化目录中的所有 Neve 文件。
fn format_dir_recursive(dir: &Path, write: bool, errors: &mut Vec<String>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            format_dir_recursive(&path, write, errors)?;
        } else if path.extension().is_some_and(|ext| ext == "neve")
            && let Err(e) = run(path.to_str().unwrap(), write)
        {
            errors.push(e);
        }
    }

    Ok(())
}
