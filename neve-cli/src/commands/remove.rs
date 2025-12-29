//! The `neve remove` command.
//!
//! Removes packages from the user environment.

use crate::output;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;

/// Remove a package from the user environment.
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
    let mut found = false;
    let mut new_manifest = String::new();
    let mut removed_path = PathBuf::new();

    for line in manifest.lines() {
        if line.is_empty() {
            continue;
        }

        let path = PathBuf::from(line);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| line.to_string());

        if name.contains(package) || line.contains(package) {
            found = true;
            removed_path = path;
        } else {
            new_manifest.push_str(line);
            new_manifest.push('\n');
        }
    }

    if !found {
        return Err(format!("Package '{}' is not installed", package));
    }

    // Create new generation
    let generation = get_next_generation(&profile_dir)?;
    let gen_dir = profile_dir.join(format!("generation-{}", generation));
    fs::create_dir_all(&gen_dir)
        .map_err(|e| format!("Failed to create generation directory: {}", e))?;

    // Write new manifest
    let new_manifest_path = gen_dir.join("manifest");
    fs::write(&new_manifest_path, &new_manifest)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;

    // Rebuild bin directory
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
    fs::remove_file(&current_link).map_err(|e| format!("Failed to remove current link: {}", e))?;

    symlink(&gen_dir, &current_link)
        .map_err(|e| format!("Failed to create current link: {}", e))?;

    output::success(&format!("Removed '{package}' (generation {generation})"));
    println!("  Removed: {}", removed_path.display());

    Ok(())
}

/// Get the profile directory.
fn get_profile_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".neve").join("profile")
}

/// Get the next generation number.
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
pub fn rollback() -> Result<(), String> {
    let profile_dir = get_profile_dir();
    let current_link = profile_dir.join("current");

    if !current_link.exists() {
        return Err("No generations to rollback to".to_string());
    }

    let current_gen =
        fs::read_link(&current_link).map_err(|e| format!("Failed to read current link: {}", e))?;

    // Extract current generation number
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
    let prev_gen = profile_dir.join(format!("generation-{}", current_num - 1));
    if !prev_gen.exists() {
        return Err(format!("Generation {} not found", current_num - 1));
    }

    // Update current symlink
    fs::remove_file(&current_link).map_err(|e| format!("Failed to remove current link: {}", e))?;

    symlink(&prev_gen, &current_link)
        .map_err(|e| format!("Failed to create current link: {}", e))?;

    output::success(&format!("Rolled back to generation {}", current_num - 1));

    Ok(())
}
