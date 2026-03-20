//! The `neve remove` command.
//! `neve remove` 命令。
//!
//! Removes packages from the user environment.
//! 从用户环境中移除软件包。

use crate::output;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Remove a package from the user environment.
/// 从用户环境中移除软件包。
pub fn run(package: &str) -> Result<(), String> {
    let profile_dir = get_profile_dir();
    let current_link = profile_dir.join("current");

    if !current_link.exists() {
        return Err("No packages installed".to_string());
    }

    let current_gen =
        fs::read_link(&current_link).map_err(|e| format!("Failed to read current link: {}", e))?;

    let manifest_path = current_gen.join("manifest");
    if !manifest_path.exists() {
        return Err("No packages installed".to_string());
    }

    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    // Find the package to remove
    // 查找要移除的软件包
    let mut matched = Vec::new();
    let mut kept = Vec::new();

    for line in manifest.lines() {
        if line.is_empty() {
            continue;
        }

        let path = PathBuf::from(line);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| line.to_string());

        let logical_name = logical_store_name(&name);
        let is_match = logical_name == package
            || name == package
            || logical_name
                .strip_prefix(package)
                .is_some_and(|rest| rest.starts_with('-'));

        if is_match {
            matched.push(path);
        } else {
            kept.push(line.to_string());
        }
    }

    if matched.is_empty() {
        return Err(format!("Package '{}' is not installed", package));
    }
    if matched.len() > 1 {
        let matches = matched
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "Package '{}' is ambiguous in current profile. Please specify a more exact name. Matches: {}",
            package, matches
        ));
    }

    let removed_path = matched.remove(0);
    let mut new_manifest = String::new();
    for line in kept {
        new_manifest.push_str(&line);
        new_manifest.push('\n');
    }

    // Create new generation
    // 创建新的代
    let generation = get_next_generation(&profile_dir)?;
    let gen_dir = profile_dir.join(format!("generation-{}", generation));
    fs::create_dir_all(&gen_dir)
        .map_err(|e| format!("Failed to create generation directory: {}", e))?;

    // Write new manifest
    // 写入新清单
    let new_manifest_path = gen_dir.join("manifest");
    fs::write(&new_manifest_path, &new_manifest)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    // Rebuild bin directory
    // 重建 bin 目录
    let bin_dir = gen_dir.join("bin");
    fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create bin directory: {}", e))?;

    for line in new_manifest.lines() {
        if line.is_empty() {
            continue;
        }

        let pkg_path = PathBuf::from(line);
        let pkg_bin = pkg_path.join("bin");

        if pkg_bin.exists() {
            for entry in
                fs::read_dir(&pkg_bin).map_err(|e| format!("Failed to read package bin: {}", e))?
            {
                let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
                let src = entry.path();
                let dst = bin_dir.join(entry.file_name());

                if !dst.exists() {
                    symlink(&src, &dst).map_err(|e| format!("Failed to create symlink: {}", e))?;
                }
            }
        }
    }

    // Update current symlink
    // 更新当前符号链接
    replace_current_link_atomically(&current_link, &gen_dir)?;

    output::success(&format!("Removed '{package}' (generation {generation})"));
    println!("  Removed: {}", removed_path.display());

    Ok(())
}

/// Get the profile directory.
/// 获取配置目录。
fn get_profile_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".neve").join("profile")
}

/// Get the next generation number.
/// 获取下一个代编号。
fn get_next_generation(profile_dir: &PathBuf) -> Result<u32, String> {
    let mut max_gen = 0;

    if profile_dir.exists() {
        for entry in
            fs::read_dir(profile_dir).map_err(|e| format!("Failed to read profile: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if let Some(num_str) = name_str.strip_prefix("generation-")
                && let Ok(num) = num_str.parse::<u32>()
            {
                max_gen = max_gen.max(num);
            }
        }
    }

    Ok(max_gen + 1)
}

/// Rollback to a previous generation.
/// 回滚到上一代。
pub fn rollback() -> Result<(), String> {
    let profile_dir = get_profile_dir();
    let current_link = profile_dir.join("current");

    if !current_link.exists() {
        return Err("No generations to rollback to".to_string());
    }

    let current_gen =
        fs::read_link(&current_link).map_err(|e| format!("Failed to read current link: {}", e))?;

    // Extract current generation number
    // 提取当前代编号
    let current_name = current_gen
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid current generation")?;

    let current_num: u32 = current_name
        .strip_prefix("generation-")
        .and_then(|s| s.parse().ok())
        .ok_or("Invalid generation number")?;

    if current_num <= 1 {
        return Err("No previous generation to rollback to".to_string());
    }

    // Find previous generation
    // 查找上一代
    let prev_gen = profile_dir.join(format!("generation-{}", current_num - 1));
    if !prev_gen.exists() {
        return Err(format!("Generation {} not found", current_num - 1));
    }

    // Update current symlink
    // 更新当前符号链接
    replace_current_link_atomically(&current_link, &prev_gen)?;

    output::success(&format!("Rolled back to generation {}", current_num - 1));

    Ok(())
}

/// Extract logical store name from a store entry (strip leading hash prefix when present).
/// 从 store 条目提取逻辑名称（存在时去掉前导哈希前缀）。
fn logical_store_name(entry: &str) -> &str {
    if let Some((prefix, rest)) = entry.split_once('-')
        && (prefix.len() == 64 || prefix.len() == 32)
        && prefix.bytes().all(|b| b.is_ascii_hexdigit())
    {
        rest
    } else {
        entry
    }
}

/// Atomically replace the current generation symlink.
/// 原子替换当前代符号链接。
fn replace_current_link_atomically(link_path: &PathBuf, target: &PathBuf) -> Result<(), String> {
    if link_path.is_dir() && !link_path.is_symlink() {
        return Err(format!(
            "Failed to update current link: path is a directory: {}",
            link_path.display()
        ));
    }

    let parent = link_path.parent().ok_or_else(|| {
        format!(
            "Failed to update current link: no parent for {}",
            link_path.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|e| {
        format!(
            "Failed to create profile directory '{}': {}",
            parent.display(),
            e
        )
    })?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let temp_link = parent.join(format!(".current.tmp-{}-{}", std::process::id(), nonce));

    if temp_link.exists() || temp_link.is_symlink() {
        fs::remove_file(&temp_link).map_err(|e| {
            format!(
                "Failed to clean temporary link '{}': {}",
                temp_link.display(),
                e
            )
        })?;
    }

    symlink(target, &temp_link).map_err(|e| {
        format!(
            "Failed to create temporary current link '{}': {}",
            temp_link.display(),
            e
        )
    })?;

    fs::rename(&temp_link, link_path).map_err(|e| {
        let _ = fs::remove_file(&temp_link);
        format!(
            "Failed to atomically replace current link '{}': {}",
            link_path.display(),
            e
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_logical_store_name_with_hash_prefix() {
        let entry = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef-hello-1.0";
        assert_eq!(logical_store_name(entry), "hello-1.0");
    }

    #[test]
    fn test_logical_store_name_without_hash_prefix() {
        assert_eq!(logical_store_name("hello-1.0"), "hello-1.0");
    }

    #[cfg(unix)]
    #[test]
    fn test_replace_current_link_atomically() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "neve-remove-link-test-{}-{}",
            std::process::id(),
            nonce
        ));
        let target1 = root.join("generation-1");
        let target2 = root.join("generation-2");
        let current = root.join("current");

        fs::create_dir_all(&target1).unwrap();
        fs::create_dir_all(&target2).unwrap();

        replace_current_link_atomically(&current, &target1).unwrap();
        assert_eq!(fs::read_link(&current).unwrap(), target1);

        replace_current_link_atomically(&current, &target2).unwrap();
        assert_eq!(fs::read_link(&current).unwrap(), target2);

        let _ = fs::remove_dir_all(root);
    }
}
