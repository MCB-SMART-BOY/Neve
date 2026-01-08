//! Module graph helpers shared by CLI commands.
//! CLI 命令共享的模块图辅助函数。

use std::path::{Path, PathBuf};

/// Resolve a file path to a module root directory and module path segments.
/// 将文件路径解析为模块根目录与模块路径段。
pub fn resolve_module_path(path: &Path) -> Result<(PathBuf, Vec<String>), String> {
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("cannot resolve path '{}': {}", path.display(), e))?;

    let mut root_dir = canonical
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // If the file is under a src/ directory, use its parent as root.
    // 如果文件位于 src/ 目录下，则使用其父目录作为根目录。
    let mut rel_path = canonical.clone();
    let mut saw_src = false;
    for ancestor in canonical.ancestors() {
        if ancestor.file_name().is_some_and(|name| name == "src") {
            if let Some(parent) = ancestor.parent() {
                root_dir = parent.to_path_buf();
                rel_path = canonical
                    .strip_prefix(ancestor)
                    .unwrap_or(&canonical)
                    .to_path_buf();
                saw_src = true;
            }
            break;
        }
    }

    if !saw_src {
        rel_path = canonical
            .strip_prefix(&root_dir)
            .unwrap_or(&canonical)
            .to_path_buf();
    }

    let mut segments: Vec<String> = rel_path
        .components()
        .filter_map(|c| {
            let part = c.as_os_str().to_string_lossy().to_string();
            if part.ends_with(".neve") {
                Some(part.trim_end_matches(".neve").to_string())
            } else {
                Some(part)
            }
        })
        .collect();

    if segments.last().map(|s| s.as_str()) == Some("mod") {
        segments.pop();
    }

    if segments.len() == 1 && segments[0] == "lib" {
        segments.clear();
    }

    if segments.is_empty() {
        return Ok((root_dir, Vec::new()));
    }

    Ok((root_dir, segments))
}
