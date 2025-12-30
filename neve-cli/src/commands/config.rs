//! The `neve config` commands.
//! `neve config` 命令。

use crate::output;
use crate::platform::{PlatformCapabilities, warn_system_config_unavailable};
use neve_config::{
    generate::Generator,
    generation::{GenerationManager, GenerationMetadata},
    module::Module,
};
use std::path::PathBuf;

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
    let module = if config_path.exists() {
        Module::load(&config_path).map_err(|e| format!("Failed to load configuration: {}", e))?
    } else {
        output::warning("No configuration file found, using default configuration.");
        Module::new("default")
    };

    // Convert to SystemConfig
    // 转换为 SystemConfig
    let mut system_config = module
        .to_system_config()
        .map_err(|e| format!("Failed to parse configuration: {}", e))?;

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

    // Create a new generation
    // 创建新的代
    let gen_manager = GenerationManager::new(generations_dir())
        .map_err(|e| format!("Failed to initialize generation manager: {}", e))?;

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

    output::success(&format!("Created generation {}.", generation.number));
    output::success("Configuration built successfully.");
    println!();
    output::info("To activate this configuration, run:");
    println!("  neve config switch");

    Ok(())
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
            println!("Activating generation {}...", gen_num);

            let generation = gen_manager
                .load_generation(gen_num)
                .map_err(|e| format!("Failed to load generation: {}", e))?;

            // For now, just report what we would do
            // 目前，只报告我们将要做什么
            println!(
                "Would activate configuration from: {}",
                generation.store_path.display_name()
            );
            println!();
            println!("Note: Full activation requires root privileges.");
            println!("In a real system, this would:");
            println!("  - Update /etc configuration files");
            println!("  - Restart affected services");
            println!("  - Update the current system profile");

            Ok(())
        }
        None => {
            Err("No configuration has been built yet. Run 'neve config build' first.".to_string())
        }
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

            println!("Rolled back to generation {}.", generation.number);
            println!();
            println!("Note: Full activation requires running 'neve config switch'.");

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

        let generation = gen_manager
            .switch_to(gen_num)
            .map_err(|e| format!("Failed to switch to generation {}: {}", gen_num, e))?;

        output::success(&format!("Switched to generation {}.", generation.number));
    } else {
        output::info("Switch cancelled.");
    }

    Ok(())
}
