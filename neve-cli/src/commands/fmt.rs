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
        // 文件未找到：{}
    }

    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    // 读取文件失败：{}

    let formatted = neve_fmt::format(&source).map_err(|e| format!("Format error: {}", e))?;
    // 格式化错误：{}

    if write {
        if formatted != source {
            fs::write(path, &formatted).map_err(|e| format!("Failed to write file: {}", e))?;
            // 写入文件失败：{}
            output::success(&format!("Formatted: {file}"));
            // 已格式化：{}
        } else {
            output::info(&format!("Already formatted: {file}"));
            // 已经格式化：{}
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
        // 文件未找到：{}
    }

    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    // 读取文件失败：{}

    let is_formatted = neve_fmt::check(&source).map_err(|e| format!("Format error: {}", e))?;
    // 格式化错误：{}

    if is_formatted {
        output::success(&format!("OK: {file}"));
        Ok(())
    } else {
        Err(format!("Would reformat: {file}"))
        // 需要重新格式化：{}
    }
}

/// Format all Neve files in a directory.
/// 格式化目录中的所有 Neve 文件。
pub fn format_dir(dir: &str, write: bool) -> Result<(), String> {
    let path = Path::new(dir);

    if !path.is_dir() {
        return Err(format!("Not a directory: {}", dir));
        // 不是目录：{}
    }

    let mut errors = Vec::new();
    format_dir_recursive(path, write, &mut errors)?;

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("{} files would be reformatted", errors.len()))
        // {} 个文件需要重新格式化
    }
}

/// Recursively format all Neve files in a directory.
/// 递归格式化目录中的所有 Neve 文件。
fn format_dir_recursive(dir: &Path, write: bool, errors: &mut Vec<String>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;
    // 读取目录失败：{}

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        // 读取条目失败：{}
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
