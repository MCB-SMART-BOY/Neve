//! The `neve install` command.
//! `neve install` 命令。
//!
//! Installs packages into the user environment.
//! 将软件包安装到用户环境。

use crate::output;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

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
    // 创建配置目录失败：{}

    // Create generation directory
    // 创建代目录
    let generation = get_next_generation(&profile_dir)?;
    let gen_dir = profile_dir.join(format!("generation-{}", generation));
    fs::create_dir_all(&gen_dir)
        .map_err(|e| format!("Failed to create generation directory: {}", e))?;
    // 创建代目录失败：{}

    // Copy current generation's packages
    // 复制当前代的软件包
    let current_link = profile_dir.join("current");
    if current_link.exists() {
        let current_gen = fs::read_link(&current_link)
            .map_err(|e| format!("Failed to read current link: {}", e))?;
        // 读取当前链接失败：{}

        // Copy manifest from current generation
        // 从当前代复制清单
        let manifest_src = current_gen.join("manifest");
        if manifest_src.exists() {
            let manifest_dst = gen_dir.join("manifest");
            fs::copy(&manifest_src, &manifest_dst)
                .map_err(|e| format!("Failed to copy manifest: {}", e))?;
            // 复制清单失败：{}
        }
    }

    // Add the new package to the manifest
    // 将新软件包添加到清单
    let manifest_path = gen_dir.join("manifest");
    let mut manifest = if manifest_path.exists() {
        fs::read_to_string(&manifest_path).map_err(|e| format!("Failed to read manifest: {}", e))?
        // 读取清单失败：{}
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
        // 软件包 '{}' 已安装
        // Clean up empty generation
        // 清理空的代
        let _ = fs::remove_dir_all(&gen_dir);
        return Ok(());
    }

    manifest.push_str(&format!("{}\n", package_path.display()));
    fs::write(&manifest_path, manifest).map_err(|e| format!("Failed to write manifest: {}", e))?;
    // 写入清单失败：{}

    // Create bin directory with symlinks
    // 创建带有符号链接的 bin 目录
    let bin_dir = gen_dir.join("bin");
    fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create bin directory: {}", e))?;
    // 创建 bin 目录失败：{}

    // Link binaries from the package
    // 从软件包链接二进制文件
    let pkg_bin = package_path.join("bin");
    if pkg_bin.exists() {
        for entry in
            fs::read_dir(&pkg_bin).map_err(|e| format!("Failed to read package bin: {}", e))?
        // 读取软件包 bin 失败：{}
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            // 读取条目失败：{}
            let src = entry.path();
            let dst = bin_dir.join(entry.file_name());

            if dst.exists() {
                fs::remove_file(&dst)
                    .map_err(|e| format!("Failed to remove existing symlink: {}", e))?;
                // 删除现有符号链接失败：{}
            }

            symlink(&src, &dst).map_err(|e| format!("Failed to create symlink: {}", e))?;
            // 创建符号链接失败：{}
        }
    }

    // Update current symlink
    // 更新当前符号链接
    if current_link.exists() {
        fs::remove_file(&current_link)
            .map_err(|e| format!("Failed to remove current link: {}", e))?;
        // 删除当前链接失败：{}
    }

    symlink(&gen_dir, &current_link)
        .map_err(|e| format!("Failed to create current link: {}", e))?;
    // 创建当前链接失败：{}

    output::success(&format!("Installed '{package}' to generation {generation}"));
    // 已将 '{}' 安装到代 {}
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

/// Find a package in the store.
/// 在存储中查找软件包。
fn find_package(store_dir: &PathBuf, package: &str) -> Result<PathBuf, String> {
    // Direct path
    // 直接路径
    let direct = store_dir.join(package);
    if direct.exists() {
        return Ok(direct);
    }

    // Search for matching packages
    // 搜索匹配的软件包
    if store_dir.exists() {
        for entry in fs::read_dir(store_dir).map_err(|e| format!("Failed to read store: {}", e))? {
            // 读取存储失败：{}
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            // 读取条目失败：{}
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Match by name (hash-name format)
            // 按名称匹配（hash-name 格式）
            if name_str.contains(package) {
                return Ok(entry.path());
            }
        }
    }

    Err(format!("Package '{}' not found in store", package))
    // 软件包 '{}' 在存储中未找到
}

/// Get the next generation number.
/// 获取下一个代编号。
fn get_next_generation(profile_dir: &PathBuf) -> Result<u32, String> {
    let mut max_gen = 0;

    if profile_dir.exists() {
        for entry in
            fs::read_dir(profile_dir).map_err(|e| format!("Failed to read profile: {}", e))?
        // 读取配置失败：{}
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            // 读取条目失败：{}
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
        // 没有安装软件包
        return Ok(());
    }

    let current_gen =
        fs::read_link(&current_link).map_err(|e| format!("Failed to read current link: {}", e))?;
    // 读取当前链接失败：{}

    let manifest_path = current_gen.join("manifest");
    if !manifest_path.exists() {
        output::info("No packages installed");
        // 没有安装软件包
        return Ok(());
    }

    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    // 读取清单失败：{}

    println!("Installed packages:");
    // 已安装的软件包：
    for line in manifest.lines() {
        if !line.is_empty() {
            // Extract package name from path
            // 从路径中提取软件包名称
            let name = PathBuf::from(line)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| line.to_string());
            println!("  {}", name);
        }
    }

    Ok(())
}
