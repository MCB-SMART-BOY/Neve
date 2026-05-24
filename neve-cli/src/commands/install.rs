//! The `neve install` command.
//! `neve install` 命令。
//!
//! Installs packages into the user environment.
//! 将软件包安装到用户环境。

use crate::output;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Install a package to the user environment.
/// 将软件包安装到用户环境。
pub fn run(package: &str) -> Result<(), String> {
    let store_dir = get_store_dir();
    let profile_dir = get_profile_dir();

    // Find the package in the store
    // 在存储中查找软件包
    let package_path = find_package(&store_dir, package)?;

    // Create profile directory if it doesn't exist
    // 如果配置目录不存在，则创建它
    fs::create_dir_all(&profile_dir)
        .map_err(|e| format!("Failed to create profile directory: {}", e))?;

    // Create generation directory
    // 创建代目录
    let generation = get_next_generation(&profile_dir)?;
    let gen_dir = profile_dir.join(format!("generation-{}", generation));
    fs::create_dir_all(&gen_dir)
        .map_err(|e| format!("Failed to create generation directory: {}", e))?;

    // Copy current generation's packages
    // 复制当前代的软件包
    let current_link = profile_dir.join("current");
    if current_link.exists() {
        let current_gen = fs::read_link(&current_link)
            .map_err(|e| format!("Failed to read current link: {}", e))?;

        // Copy manifest from current generation
        // 从当前代复制清单
        let manifest_src = current_gen.join("manifest");
        if manifest_src.exists() {
            let manifest_dst = gen_dir.join("manifest");
            fs::copy(&manifest_src, &manifest_dst)
                .map_err(|e| format!("Failed to copy manifest: {}", e))?;
        }
    }

    // Add the new package to the manifest
    // 将新软件包添加到清单
    let manifest_path = gen_dir.join("manifest");
    let mut manifest = if manifest_path.exists() {
        fs::read_to_string(&manifest_path).map_err(|e| format!("Failed to read manifest: {}", e))?
    } else {
        String::new()
    };

    // Check if already installed
    // 检查是否已安装
    if manifest
        .lines()
        .any(|line| line == package_path.to_string_lossy())
    {
        output::info(&format!("Package '{package}' is already installed"));
        // Clean up empty generation
        // 清理空的代
        let _ = fs::remove_dir_all(&gen_dir);
        return Ok(());
    }

    manifest.push_str(&format!("{}\n", package_path.display()));
    fs::write(&manifest_path, manifest).map_err(|e| format!("Failed to write manifest: {}", e))?;

    // Create bin directory with symlinks
    // 创建带有符号链接的 bin 目录
    let bin_dir = gen_dir.join("bin");
    fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create bin directory: {}", e))?;

    // Link binaries from the package
    // 从软件包链接二进制文件
    let pkg_bin = package_path.join("bin");
    if pkg_bin.exists() {
        for entry in
            fs::read_dir(&pkg_bin).map_err(|e| format!("Failed to read package bin: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let src = entry.path();
            let dst = bin_dir.join(entry.file_name());

            if dst.exists() {
                fs::remove_file(&dst)
                    .map_err(|e| format!("Failed to remove existing symlink: {}", e))?;
            }

            symlink(&src, &dst).map_err(|e| format!("Failed to create symlink: {}", e))?;
        }
    }

    // Update current symlink
    // 更新当前符号链接
    replace_current_link_atomically(&current_link, &gen_dir)?;

    output::success(&format!("Installed '{package}' to generation {generation}"));
    println!("  {package} -> {}", package_path.display());

    Ok(())
}

/// Get the store directory.
/// 获取存储目录。
fn get_store_dir() -> PathBuf {
    std::env::var("NEVE_STORE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/neve/store"))
}

/// Get the profile directory.
/// 获取配置目录。
fn get_profile_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".neve").join("profile")
}

/// Find a package in the store, with registry fallback.
/// 在存储中查找软件包，带注册表回退。
fn find_package(store_dir: &PathBuf, package: &str) -> Result<PathBuf, String> {
    // Direct path
    let direct = store_dir.join(package);
    if direct.exists() {
        return Ok(direct);
    }

    // Search for matching packages
    if store_dir.exists() {
        let mut exact_matches = Vec::new();
        let mut fuzzy_matches = Vec::new();

        for entry in fs::read_dir(store_dir).map_err(|e| format!("Failed to read store: {e}"))? {
            let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            let logical_name = logical_store_name(&name_str);

            if logical_name == package || name_str == package {
                exact_matches.push(entry.path());
                continue;
            }

            if logical_name
                .strip_prefix(package)
                .is_some_and(|rest| rest.starts_with('-'))
            {
                fuzzy_matches.push(entry.path());
            }
        }

        if exact_matches.len() == 1 {
            return Ok(exact_matches.remove(0));
        }
        if exact_matches.len() > 1 {
            let candidates = exact_matches
                .iter()
                .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "Package '{}' is ambiguous. Please use a full store path name. Matches: {}",
                package, candidates
            ));
        }

        if fuzzy_matches.len() == 1 {
            return Ok(fuzzy_matches.remove(0));
        }
        if fuzzy_matches.len() > 1 {
            let candidates = fuzzy_matches
                .iter()
                .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "Package '{}' matches multiple versions. Please specify a more exact name: {}",
                package, candidates
            ));
        }
    }

    // Query remote registry for available versions
    if let Some(client) = crate::registry_client::RegistryClient::from_env() {
        match client.get_package(package) {
            Ok(pkg) => {
                let versions: Vec<String> =
                    pkg.versions.iter().map(|v| v.version.clone()).collect();
                if versions.is_empty() {
                    return Err(format!(
                        "Package '{package}' not found in store or registry"
                    ));
                }
                return Err(format!(
                    "Package '{package}' not found in local store.\n\
                     Available versions from registry: {}\n\
                     To install: fetch and add to local store first.",
                    versions.join(", ")
                ));
            }
            Err(e) => {
                return Err(format!(
                    "Package '{package}' not found in store (registry query failed: {e})"
                ));
            }
        }
    }

    Err(format!(
        "Package '{package}' not found in store. Set NEVE_REGISTRY to query a remote registry."
    ))
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

/// List installed packages.
/// 列出已安装的软件包。
pub fn list() -> Result<(), String> {
    let profile_dir = get_profile_dir();
    let current_link = profile_dir.join("current");

    if !current_link.exists() {
        output::info("No packages installed");
        return Ok(());
    }

    let current_gen =
        fs::read_link(&current_link).map_err(|e| format!("Failed to read current link: {}", e))?;

    let manifest_path = current_gen.join("manifest");
    if !manifest_path.exists() {
        output::info("No packages installed");
        return Ok(());
    }

    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    output::header("Installed Packages");

    let mut table = output::Table::new(vec!["#", "Package"]);
    let mut count = 0;

    for line in manifest.lines() {
        if !line.is_empty() {
            count += 1;
            // Extract package name from path
            // 从路径中提取软件包名称
            let name = PathBuf::from(line)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| line.to_string());
            table.add_row(vec![&count.to_string(), &name]);
        }
    }

    table.print();

    Ok(())
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
        let entry = "hello-1.0";
        assert_eq!(logical_store_name(entry), "hello-1.0");
    }

    #[cfg(unix)]
    #[test]
    fn test_replace_current_link_atomically() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "neve-install-link-test-{}-{}",
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
