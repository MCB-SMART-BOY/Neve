//! The `neve config` commands.
//! `neve config` 命令。

use crate::output;
use crate::platform::{PlatformCapabilities, warn_system_config_unavailable};
use neve_config::{
    activate::Activator,
    generate::{GeneratedConfig, GeneratedFile, Generator},
    generation::{Generation, GenerationManager, GenerationMetadata},
    module::Module,
};
use neve_derive::Hash;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Snapshot manifest file name under a generation directory.
/// generation 目录中的快照清单文件名。
const GENERATED_SNAPSHOT_FILE: &str = "generated-config.json";
/// Snapshot artifact directory under a generation directory.
/// generation 目录中的快照产物目录。
const GENERATED_ARTIFACTS_DIR: &str = "generated-artifacts";

/// Serializable snapshot of a generated configuration.
/// 可序列化的已生成配置快照。
#[derive(Debug, Serialize, Deserialize)]
struct GeneratedConfigSnapshot {
    files: Vec<GeneratedFileSnapshot>,
    services: Vec<String>,
    activation_script: Option<String>,
    #[serde(default)]
    activation_script_hash: Option<String>,
}

/// Serializable snapshot entry for a generated file.
/// 生成文件的可序列化快照条目。
#[derive(Debug, Serialize, Deserialize)]
struct GeneratedFileSnapshot {
    source: String,
    target: String,
    mode: u32,
    #[serde(default)]
    hash: Option<String>,
}

/// Get the default configuration file path.
/// 获取默认配置文件路径。
fn default_config_path() -> PathBuf {
    // Look for configuration in standard locations
    // 在标准位置查找配置
    let candidates = [
        PathBuf::from("./configuration.neve"),
        PathBuf::from("/etc/neve/configuration.neve"),
    ];

    for path in &candidates {
        if path.exists() {
            return path.clone();
        }
    }

    // Also check user config
    // 也检查用户配置
    if let Some(path) = dirs_config_path()
        && path.exists()
    {
        return path;
    }

    PathBuf::from("./configuration.neve")
}

/// Get the user's config directory path.
/// 获取用户的配置目录路径。
fn dirs_config_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".config/neve/configuration.neve"))
}

/// Get the generations directory.
/// 获取代目录。
fn generations_dir() -> PathBuf {
    std::env::var("NEVE_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/lib/neve"))
}

/// Get the build output directory.
/// 获取构建输出目录。
fn build_dir() -> PathBuf {
    std::env::var("NEVE_BUILD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("neve-build"))
}

/// Get activation root for `neve config switch`.
/// 获取 `neve config switch` 的激活根目录。
fn activation_root() -> PathBuf {
    std::env::var("NEVE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

/// Whether to run activation in dry-run mode.
/// 是否以 dry-run 模式运行激活。
fn activation_dry_run() -> bool {
    std::env::var("NEVE_CONFIG_DRY_RUN")
        .ok()
        .map(|v| parse_env_bool(v.as_str()))
        .unwrap_or(false)
}

/// Parse common truthy/falsy environment boolean values.
/// 解析常见环境变量布尔值。
fn parse_env_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Build system configuration.
/// 构建系统配置。
pub fn build() -> Result<(), String> {
    // Check platform support
    // 检查平台支持
    let caps = PlatformCapabilities::detect();
    if !caps.system_config {
        warn_system_config_unavailable();
        return Err("System configuration is only supported on Linux.".to_string());
    }

    let config_path = default_config_path();

    output::info(&format!(
        "Building system configuration from {}...",
        config_path.display()
    ));

    // Load the configuration module
    // 加载配置模块
    let mut system_config = if config_path.exists() {
        Module::load_merged(&config_path)
            .map_err(|e| format!("Failed to load configuration graph: {}", e))?
    } else {
        output::warning("No configuration file found, using default configuration.");
        Module::new("default")
            .to_system_config()
            .map_err(|e| format!("Failed to build default configuration: {}", e))?
    };

    // Create a new generation
    // 创建新的代
    let gen_manager = GenerationManager::new(generations_dir())
        .map_err(|e| format!("Failed to initialize generation manager: {}", e))?;
    system_config.generation = gen_manager
        .next_generation()
        .map_err(|e| format!("Failed to determine next generation number: {}", e))?;

    // Generate configuration files
    // 生成配置文件
    let output_dir = build_dir();
    let generator = Generator::new(output_dir.clone());
    let generated = generator
        .generate(&system_config)
        .map_err(|e| format!("Failed to generate configuration: {}", e))?;

    output::info(&format!(
        "Generated {} configuration files.",
        generated.files.len()
    ));

    // Create store path from derivation
    // 从派生创建存储路径
    let drv = generator.to_derivation(&system_config);
    let store_path = drv.drv_path();

    let metadata = GenerationMetadata::new()
        .name(&system_config.name)
        .description("Built by neve config build");

    let generation = gen_manager
        .create_generation(&store_path, metadata)
        .map_err(|e| format!("Failed to create generation: {}", e))?;

    system_config.generation = generation.number;
    save_generated_snapshot(&generated, &generation.path)?;

    output::success(&format!("Created generation {}.", generation.number));
    output::success("Configuration built successfully.");
    println!();
    output::info("To activate this configuration, run:");
    println!("  neve config switch");

    Ok(())
}

/// Convert an absolute path into a path string relative to `base`.
/// 将绝对路径转换为相对于 `base` 的路径字符串。
fn rel_path_string(base: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(base)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|_| {
            format!(
                "Failed to store generated artifact outside generation dir: {}",
                path.display()
            )
        })
}

/// Normalize an artifact filename for snapshot storage.
/// 规范化快照存储用的产物文件名。
fn normalize_artifact_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Hash file contents to a hex string.
/// 将文件内容哈希为十六进制字符串。
fn hash_file_hex(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
    Ok(Hash::of(&bytes).to_hex())
}

/// Resolve a snapshot artifact path and reject path traversal.
/// 解析快照产物路径并拒绝路径穿越。
fn resolve_snapshot_artifact_path(generation_dir: &Path, rel: &str) -> Result<PathBuf, String> {
    use std::ffi::OsStr;
    use std::path::Component;

    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return Err(format!(
            "Invalid snapshot artifact path '{}': absolute paths are not allowed",
            rel
        ));
    }

    let mut components = rel_path.components();
    let first = components.next().ok_or_else(|| {
        format!(
            "Invalid snapshot artifact path '{}': empty path is not allowed",
            rel
        )
    })?;

    match first {
        Component::Normal(seg) if seg == OsStr::new(GENERATED_ARTIFACTS_DIR) => {}
        _ => {
            return Err(format!(
                "Invalid snapshot artifact path '{}': must be under '{}'",
                rel, GENERATED_ARTIFACTS_DIR
            ));
        }
    }

    for comp in components {
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "Invalid snapshot artifact path '{}': path traversal is not allowed",
                    rel
                ));
            }
        }
    }

    Ok(generation_dir.join(rel_path))
}

/// Save generated files into generation-local snapshot artifacts.
/// 将生成文件保存为 generation 本地快照产物。
fn save_generated_snapshot(
    generated: &GeneratedConfig,
    generation_dir: &Path,
) -> Result<(), String> {
    let artifacts_dir = generation_dir.join(GENERATED_ARTIFACTS_DIR);
    fs::create_dir_all(&artifacts_dir)
        .map_err(|e| format!("Failed to create generation artifact dir: {}", e))?;

    let mut snapshot = GeneratedConfigSnapshot {
        files: Vec::new(),
        services: generated.services.clone(),
        activation_script: None,
        activation_script_hash: None,
    };

    for (index, file) in generated.files.iter().enumerate() {
        let name = file
            .source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");
        let artifact_name = format!("{:04}-{}", index, normalize_artifact_name(name));
        let artifact_path = artifacts_dir.join(artifact_name);
        fs::copy(&file.source, &artifact_path).map_err(|e| {
            format!(
                "Failed to copy generated file '{}' into generation snapshot: {}",
                file.source.display(),
                e
            )
        })?;

        snapshot.files.push(GeneratedFileSnapshot {
            source: rel_path_string(generation_dir, &artifact_path)?,
            target: file.target.to_string_lossy().to_string(),
            mode: file.mode,
            hash: Some(hash_file_hex(&artifact_path)?),
        });
    }

    if let Some(script) = &generated.activation_script {
        let script_artifact = artifacts_dir.join("activate");
        fs::copy(script, &script_artifact).map_err(|e| {
            format!(
                "Failed to copy activation script '{}' into generation snapshot: {}",
                script.display(),
                e
            )
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(script)
                .map_err(|e| format!("Failed to read activation script metadata: {}", e))?
                .permissions()
                .mode();
            fs::set_permissions(&script_artifact, fs::Permissions::from_mode(mode))
                .map_err(|e| format!("Failed to set activation script permissions: {}", e))?;
        }
        snapshot.activation_script = Some(rel_path_string(generation_dir, &script_artifact)?);
        snapshot.activation_script_hash = Some(hash_file_hex(&script_artifact)?);
    }

    let snapshot_path = generation_dir.join(GENERATED_SNAPSHOT_FILE);
    let content = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| format!("Failed to serialize generation snapshot: {}", e))?;
    fs::write(&snapshot_path, content)
        .map_err(|e| format!("Failed to write generation snapshot: {}", e))?;

    Ok(())
}

/// Load generated snapshot artifacts from a generation directory.
/// 从 generation 目录加载生成快照产物。
fn load_generated_snapshot(generation_dir: &Path) -> Result<Option<GeneratedConfig>, String> {
    let snapshot_path = generation_dir.join(GENERATED_SNAPSHOT_FILE);
    if !snapshot_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&snapshot_path)
        .map_err(|e| format!("Failed to read generation snapshot: {}", e))?;
    let snapshot: GeneratedConfigSnapshot = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid generation snapshot JSON: {}", e))?;

    let mut generated = GeneratedConfig::new();
    generated.services = snapshot.services;

    for file in snapshot.files {
        let source = resolve_snapshot_artifact_path(generation_dir, &file.source)?;
        if !source.exists() {
            return Err(format!(
                "Missing generated artifact '{}' referenced by snapshot",
                source.display()
            ));
        }
        if let Some(expected_hash) = &file.hash {
            let actual_hash = hash_file_hex(&source)?;
            if &actual_hash != expected_hash {
                return Err(format!(
                    "Hash mismatch for generated artifact '{}': expected {}, got {}",
                    source.display(),
                    expected_hash,
                    actual_hash
                ));
            }
        }
        generated.files.push(GeneratedFile {
            source,
            target: PathBuf::from(file.target),
            mode: file.mode,
        });
    }

    if let Some(script_rel) = snapshot.activation_script {
        let script = resolve_snapshot_artifact_path(generation_dir, &script_rel)?;
        if !script.exists() {
            return Err(format!(
                "Missing activation script artifact '{}' referenced by snapshot",
                script.display()
            ));
        }
        if let Some(expected_hash) = snapshot.activation_script_hash {
            let actual_hash = hash_file_hex(&script)?;
            if actual_hash != expected_hash {
                return Err(format!(
                    "Hash mismatch for activation script artifact '{}': expected {}, got {}",
                    script.display(),
                    expected_hash,
                    actual_hash
                ));
            }
        }
        generated.activation_script = Some(script);
    }

    Ok(Some(generated))
}

/// Activate a specific generation.
/// 激活指定 generation。
fn activate_generation(generation: &Generation) -> Result<(), String> {
    output::info(&format!(
        "Activating generation {} ({})...",
        generation.number,
        generation.store_path.display_name()
    ));

    let generated = load_generated_snapshot(&generation.path)?.ok_or_else(|| {
        format!(
            "Generation {} is missing '{}' snapshot. Rebuild the configuration to restore immutable activation artifacts.",
            generation.number, GENERATED_SNAPSHOT_FILE
        )
    })?;

    let root = activation_root();
    let dry_run = activation_dry_run();
    if dry_run {
        output::warning("Dry-run activation enabled via NEVE_CONFIG_DRY_RUN.");
    }

    let activator = Activator::new().root(&root).dry_run(dry_run).verbose(true);
    let result = activator
        .activate(&generated)
        .map_err(|e| format!("Failed to activate generation {}: {}", generation.number, e))?;

    output::success(&format!("Activated generation {}.", generation.number));
    output::info(&format!(
        "Installed {} file(s), enabled {} service(s).",
        result.files_installed, result.services_enabled
    ));
    if let Some(script_output) = result.script_output
        && !script_output.trim().is_empty()
    {
        output::info("Activation script output:");
        println!("{}", script_output.trim_end());
    }

    Ok(())
}

/// Verify that a generation has a valid activation snapshot.
/// 校验 generation 的激活快照是否有效。
fn verify_generation_snapshot(generation: &Generation) -> Result<(), String> {
    match load_generated_snapshot(&generation.path)? {
        Some(_) => Ok(()),
        None => Err(format!(
            "generation {} is missing '{}'",
            generation.number, GENERATED_SNAPSHOT_FILE
        )),
    }
}

/// Switch generation pointer and activate; restore previous pointer on activation failure.
/// 切换 generation 指针并激活；若激活失败则恢复之前的指针。
fn switch_to_generation_with_activation<F>(
    gen_manager: &GenerationManager,
    gen_num: u64,
    mut activate: F,
) -> Result<Generation, String>
where
    F: FnMut(&Generation) -> Result<(), String>,
{
    let previous_current = gen_manager
        .current_generation()
        .map_err(|e| format!("Failed to get current generation: {}", e))?;

    let generation = gen_manager
        .switch_to(gen_num)
        .map_err(|e| format!("Failed to switch to generation {}: {}", gen_num, e))?;

    if let Err(err) = activate(&generation) {
        if let Some(prev) = previous_current {
            if prev == gen_num {
                return Err(format!(
                    "Activation failed for generation {}: {}",
                    gen_num, err
                ));
            }
            let restore = gen_manager.switch_to(prev);
            return match restore {
                Ok(_) => Err(format!(
                    "Switched generation pointer to {} but activation failed: {}. Restored current generation pointer to {}.",
                    gen_num, err, prev
                )),
                Err(restore_err) => Err(format!(
                    "Switched generation pointer to {} but activation failed: {}. Failed to restore current generation pointer: {}",
                    gen_num, err, restore_err
                )),
            };
        }

        return Err(format!(
            "Switched generation pointer to {} but activation failed: {}. No previous current generation to restore.",
            gen_num, err
        ));
    }

    Ok(generation)
}

/// Switch to a new or specific configuration.
/// 切换到新配置或特定配置。
pub fn switch() -> Result<(), String> {
    // Check platform support
    // 检查平台支持
    let caps = PlatformCapabilities::detect();
    if !caps.system_config {
        warn_system_config_unavailable();
        return Err("System configuration is only supported on Linux.".to_string());
    }

    let gen_manager = GenerationManager::new(generations_dir())
        .map_err(|e| format!("Failed to initialize generation manager: {}", e))?;

    let current = gen_manager
        .current_generation()
        .map_err(|e| format!("Failed to get current generation: {}", e))?;

    match current {
        Some(gen_num) => {
            let generation = gen_manager
                .load_generation(gen_num)
                .map_err(|e| format!("Failed to load generation: {}", e))?;
            activate_generation(&generation)
        }
        None => {
            Err("No configuration has been built yet. Run 'neve config build' first.".to_string())
        }
    }
}

/// Verify generation activation snapshot integrity.
/// 校验 generation 激活快照完整性。
pub fn verify(generation: Option<u64>, all: bool) -> Result<(), String> {
    if all && generation.is_some() {
        return Err("Cannot specify both --all and a generation number.".to_string());
    }

    let gen_manager = GenerationManager::new(generations_dir())
        .map_err(|e| format!("Failed to initialize generation manager: {}", e))?;

    let targets: Vec<Generation> = if all {
        let gens = gen_manager
            .list_generations()
            .map_err(|e| format!("Failed to list generations: {}", e))?;
        if gens.is_empty() {
            return Err("No configuration generations found.".to_string());
        }
        gens
    } else if let Some(gen_num) = generation {
        vec![
            gen_manager
                .load_generation(gen_num)
                .map_err(|e| format!("Failed to load generation {}: {}", gen_num, e))?,
        ]
    } else {
        let current = gen_manager
            .current_generation()
            .map_err(|e| format!("Failed to get current generation: {}", e))?
            .ok_or_else(|| "No configuration has been built yet.".to_string())?;
        vec![
            gen_manager
                .load_generation(current)
                .map_err(|e| format!("Failed to load generation {}: {}", current, e))?,
        ]
    };

    output::header("Generation Snapshot Verification");

    let mut failed = 0usize;
    for generation in targets {
        match verify_generation_snapshot(&generation) {
            Ok(()) => output::success(&format!(
                "Generation {}: snapshot integrity OK",
                generation.number
            )),
            Err(err) => {
                failed += 1;
                output::error(&format!(
                    "Generation {}: snapshot verification failed: {}",
                    generation.number, err
                ));
            }
        }
    }

    if failed == 0 {
        Ok(())
    } else {
        Err(format!("{} generation(s) failed verification", failed))
    }
}

/// Rollback to a previous configuration.
/// 回滚到上一个配置。
pub fn rollback() -> Result<(), String> {
    // Check platform support
    // 检查平台支持
    let caps = PlatformCapabilities::detect();
    if !caps.system_config {
        warn_system_config_unavailable();
        return Err("System configuration is only supported on Linux.".to_string());
    }

    let gen_manager = GenerationManager::new(generations_dir())
        .map_err(|e| format!("Failed to initialize generation manager: {}", e))?;

    let current = gen_manager
        .current_generation()
        .map_err(|e| format!("Failed to get current generation: {}", e))?;

    match current {
        Some(gen_num) if gen_num > 1 => {
            let prev_gen = gen_num - 1;
            println!(
                "Rolling back from generation {} to {}...",
                gen_num, prev_gen
            );

            let generation = gen_manager
                .switch_to(prev_gen)
                .map_err(|e| format!("Failed to switch to generation {}: {}", prev_gen, e))?;

            if let Err(err) = activate_generation(&generation) {
                let restore = gen_manager.switch_to(gen_num);
                return match restore {
                    Ok(_) => Err(format!(
                        "Rolled back generation pointer to {} but activation failed: {}. Restored current generation pointer to {}.",
                        prev_gen, err, gen_num
                    )),
                    Err(restore_err) => Err(format!(
                        "Rolled back generation pointer to {} but activation failed: {}. Failed to restore current generation pointer: {}",
                        prev_gen, err, restore_err
                    )),
                };
            }

            output::success(&format!("Rolled back to generation {}.", generation.number));

            Ok(())
        }
        Some(_) => Err("Already at generation 1, cannot rollback further.".to_string()),
        None => Err("No configuration has been built yet.".to_string()),
    }
}

/// List all configuration generations.
/// 列出所有配置代。
pub fn list_generations() -> Result<(), String> {
    let gen_manager = GenerationManager::new(generations_dir())
        .map_err(|e| format!("Failed to initialize generation manager: {}", e))?;

    let current = gen_manager
        .current_generation()
        .map_err(|e| format!("Failed to get current generation: {}", e))?;

    let generations = gen_manager
        .list_generations()
        .map_err(|e| format!("Failed to list generations: {}", e))?;

    if generations.is_empty() {
        output::info("No configuration generations found.");
        output::info("Run 'neve config build' to create one.");
        return Ok(());
    }

    output::header("System Configuration Generations");

    let mut table = output::Table::new(vec!["#", "Name", "Description", "Status"]);

    for generation in generations.iter().rev() {
        let status = if Some(generation.number) == current {
            "current"
        } else {
            ""
        };
        let name = generation.metadata.name.as_deref().unwrap_or("unnamed");
        let desc = generation.metadata.description.as_deref().unwrap_or("");

        table.add_row(vec![&generation.number.to_string(), name, desc, status]);
    }

    table.print();

    Ok(())
}

/// Interactively switch to a specific generation.
/// 交互式切换到特定代。
pub fn switch_interactive() -> Result<(), String> {
    // Check platform support
    // 检查平台支持
    let caps = PlatformCapabilities::detect();
    if !caps.system_config {
        warn_system_config_unavailable();
        return Err("System configuration is only supported on Linux.".to_string());
    }

    let gen_manager = GenerationManager::new(generations_dir())
        .map_err(|e| format!("Failed to initialize generation manager: {}", e))?;

    let generations = gen_manager
        .list_generations()
        .map_err(|e| format!("Failed to list generations: {}", e))?;

    if generations.is_empty() {
        return Err("No generations available. Run 'neve config build' first.".to_string());
    }

    // Show available generations
    // 显示可用代
    list_generations()?;

    println!();

    // Prompt for generation number
    // 提示输入代编号
    if let Some(input) = output::prompt("Enter generation number to switch to") {
        let gen_num: u64 = input
            .parse()
            .map_err(|_| format!("Invalid generation number: {}", input))?;

        let generation =
            switch_to_generation_with_activation(&gen_manager, gen_num, activate_generation)?;
        output::success(&format!("Switched to generation {}.", generation.number));
    } else {
        output::info("Switch cancelled.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use neve_derive::{Hash as DeriveHash, StorePath};
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "neve-config-command-test-{}-{}-{}",
            prefix,
            std::process::id(),
            nonce
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn dummy_generation(path: PathBuf, number: u64) -> Generation {
        Generation {
            number,
            path,
            store_path: StorePath::new(DeriveHash::of(b"dummy"), format!("dummy-{}", number)),
            metadata: GenerationMetadata::new(),
        }
    }

    #[test]
    fn test_parse_env_bool() {
        assert!(parse_env_bool("1"));
        assert!(parse_env_bool("TRUE"));
        assert!(parse_env_bool(" yes "));
        assert!(!parse_env_bool("0"));
        assert!(!parse_env_bool("false"));
        assert!(!parse_env_bool("off"));
        assert!(!parse_env_bool("random"));
    }

    #[test]
    fn test_verify_rejects_all_with_generation() {
        let err = verify(Some(1), true).expect_err("verify should reject conflicting arguments");
        assert!(err.contains("Cannot specify both --all and a generation number"));
    }

    #[test]
    fn test_generated_snapshot_roundtrip() {
        let dir = temp_dir("snapshot");
        let gen_dir = dir.join("generation-1");
        fs::create_dir_all(&gen_dir).unwrap();

        let source = dir.join("source.conf");
        fs::write(&source, "content\n").unwrap();
        let script = dir.join("activate.sh");
        fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        }

        let mut generated = GeneratedConfig::new();
        generated.files.push(GeneratedFile {
            source,
            target: PathBuf::from("/etc/test.conf"),
            mode: 0o644,
        });
        generated.services.push("demo".to_string());
        generated.activation_script = Some(script);

        save_generated_snapshot(&generated, &gen_dir).unwrap();
        let loaded = load_generated_snapshot(&gen_dir).unwrap().unwrap();

        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files[0].target, PathBuf::from("/etc/test.conf"));
        assert_eq!(loaded.files[0].mode, 0o644);
        assert!(loaded.files[0].source.exists());
        assert_eq!(loaded.services, vec!["demo".to_string()]);
        assert!(
            loaded
                .activation_script
                .as_ref()
                .expect("activation script present")
                .exists()
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generated_snapshot_detects_hash_mismatch() {
        let dir = temp_dir("snapshot-hash-mismatch");
        let gen_dir = dir.join("generation-1");
        fs::create_dir_all(&gen_dir).unwrap();

        let source = dir.join("source.conf");
        fs::write(&source, "content\n").unwrap();
        let mut generated = GeneratedConfig::new();
        generated.files.push(GeneratedFile {
            source,
            target: PathBuf::from("/etc/test.conf"),
            mode: 0o644,
        });

        save_generated_snapshot(&generated, &gen_dir).unwrap();

        let snapshot_path = gen_dir.join(GENERATED_SNAPSHOT_FILE);
        let snapshot: GeneratedConfigSnapshot =
            serde_json::from_str(&fs::read_to_string(&snapshot_path).unwrap()).unwrap();
        let artifact_path = gen_dir.join(&snapshot.files[0].source);
        fs::write(&artifact_path, "tampered\n").unwrap();

        let err = load_generated_snapshot(&gen_dir).unwrap_err();
        assert!(err.contains("Hash mismatch"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_legacy_snapshot_without_hashes() {
        let dir = temp_dir("snapshot-legacy");
        let gen_dir = dir.join("generation-1");
        let artifacts = gen_dir.join(GENERATED_ARTIFACTS_DIR);
        fs::create_dir_all(&artifacts).unwrap();

        let artifact_rel = format!("{}/{}", GENERATED_ARTIFACTS_DIR, "legacy.conf");
        let artifact_path = gen_dir.join(&artifact_rel);
        fs::write(&artifact_path, "legacy\n").unwrap();

        let legacy_snapshot = json!({
            "files": [{
                "source": artifact_rel,
                "target": "/etc/legacy.conf",
                "mode": 0o644
            }],
            "services": ["legacy-service"],
            "activation_script": null
        });
        fs::write(
            gen_dir.join(GENERATED_SNAPSHOT_FILE),
            serde_json::to_string_pretty(&legacy_snapshot).unwrap(),
        )
        .unwrap();

        let loaded = load_generated_snapshot(&gen_dir).unwrap().unwrap();
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files[0].target, PathBuf::from("/etc/legacy.conf"));
        assert_eq!(loaded.services, vec!["legacy-service".to_string()]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_snapshot_rejects_parent_traversal_source_path() {
        let dir = temp_dir("snapshot-traversal-source");
        let gen_dir = dir.join("generation-1");
        fs::create_dir_all(&gen_dir).unwrap();

        let invalid_snapshot = json!({
            "files": [{
                "source": "../outside.conf",
                "target": "/etc/legacy.conf",
                "mode": 0o644
            }],
            "services": [],
            "activation_script": null
        });
        fs::write(
            gen_dir.join(GENERATED_SNAPSHOT_FILE),
            serde_json::to_string_pretty(&invalid_snapshot).unwrap(),
        )
        .unwrap();

        let err = load_generated_snapshot(&gen_dir).unwrap_err();
        assert!(err.contains("Invalid snapshot artifact path"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_snapshot_rejects_absolute_activation_script_path() {
        let dir = temp_dir("snapshot-absolute-script");
        let gen_dir = dir.join("generation-1");
        fs::create_dir_all(&gen_dir).unwrap();

        let invalid_snapshot = json!({
            "files": [],
            "services": [],
            "activation_script": "/tmp/evil.sh"
        });
        fs::write(
            gen_dir.join(GENERATED_SNAPSHOT_FILE),
            serde_json::to_string_pretty(&invalid_snapshot).unwrap(),
        )
        .unwrap();

        let err = load_generated_snapshot(&gen_dir).unwrap_err();
        assert!(err.contains("Invalid snapshot artifact path"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_verify_generation_snapshot_missing_manifest() {
        let dir = temp_dir("verify-missing");
        let gen_dir = dir.join("generation-1");
        fs::create_dir_all(&gen_dir).unwrap();

        let generation = dummy_generation(gen_dir.clone(), 1);
        let err = verify_generation_snapshot(&generation).unwrap_err();
        assert!(err.contains("missing"));
        assert!(err.contains(GENERATED_SNAPSHOT_FILE));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_activate_generation_requires_snapshot_manifest() {
        let dir = temp_dir("activate-missing-snapshot");
        let gen_dir = dir.join("generation-1");
        fs::create_dir_all(&gen_dir).unwrap();

        let generation = dummy_generation(gen_dir.clone(), 1);
        let err = activate_generation(&generation).expect_err("activation should fail");
        assert!(err.contains("missing 'generated-config.json'"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_verify_generation_snapshot_ok() {
        let dir = temp_dir("verify-ok");
        let gen_dir = dir.join("generation-1");
        fs::create_dir_all(&gen_dir).unwrap();

        let source = dir.join("source.conf");
        fs::write(&source, "content\n").unwrap();
        let mut generated = GeneratedConfig::new();
        generated.files.push(GeneratedFile {
            source,
            target: PathBuf::from("/etc/test.conf"),
            mode: 0o644,
        });
        save_generated_snapshot(&generated, &gen_dir).unwrap();

        let generation = dummy_generation(gen_dir.clone(), 1);
        verify_generation_snapshot(&generation).unwrap();

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_switch_to_generation_with_activation_restores_previous_pointer_on_failure() {
        let state_dir = temp_dir("switch-restore");
        let manager = GenerationManager::new(state_dir.clone()).expect("generation manager");

        let gen1_store = StorePath::new(DeriveHash::of(b"gen1"), "gen1".to_string());
        let gen2_store = StorePath::new(DeriveHash::of(b"gen2"), "gen2".to_string());

        manager
            .create_generation(&gen1_store, GenerationMetadata::new())
            .expect("create generation 1");
        manager
            .create_generation(&gen2_store, GenerationMetadata::new())
            .expect("create generation 2");

        assert_eq!(manager.current_generation().unwrap(), Some(2));

        let err = switch_to_generation_with_activation(&manager, 1, |_| {
            Err("simulated activation failure".to_string())
        })
        .expect_err("switch should fail");

        assert!(err.contains("Restored current generation pointer to 2"));
        assert_eq!(manager.current_generation().unwrap(), Some(2));

        let _ = fs::remove_dir_all(&state_dir);
    }
}
