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
        // 系统配置仅在 Linux 上支持。
    }

    let config_path = default_config_path();

    output::info(&format!(
        "Building system configuration from {}...",
        config_path.display()
    ));
    // 正在从 {} 构建系统配置...

    // Load the configuration module
    // 加载配置模块
    let module = if config_path.exists() {
        Module::load(&config_path).map_err(|e| format!("Failed to load configuration: {}", e))?
        // 加载配置失败：{}
    } else {
        output::warning("No configuration file found, using default configuration.");
        // 未找到配置文件，使用默认配置。
        Module::new("default")
    };

    // Convert to SystemConfig
    // 转换为 SystemConfig
    let mut system_config = module
        .to_system_config()
        .map_err(|e| format!("Failed to parse configuration: {}", e))?;
    // 解析配置失败：{}

    // Generate configuration files
    // 生成配置文件
    let output_dir = build_dir();
    let generator = Generator::new(output_dir.clone());
    let generated = generator
        .generate(&system_config)
        .map_err(|e| format!("Failed to generate configuration: {}", e))?;
    // 生成配置失败：{}

    output::info(&format!(
        "Generated {} configuration files.",
        generated.files.len()
    ));
    // 生成了 {} 个配置文件。

    // Create a new generation
    // 创建新的代
    let gen_manager = GenerationManager::new(generations_dir())
        .map_err(|e| format!("Failed to initialize generation manager: {}", e))?;
    // 初始化代管理器失败：{}

    // Create store path from derivation
    // 从派生创建存储路径
    let drv = generator.to_derivation(&system_config);
    let store_path = drv.drv_path();

    let metadata = GenerationMetadata::new()
        .name(&system_config.name)
        .description("Built by neve config build");
    // 由 neve config build 构建

    let generation = gen_manager
        .create_generation(&store_path, metadata)
        .map_err(|e| format!("Failed to create generation: {}", e))?;
    // 创建代失败：{}

    system_config.generation = generation.number;

    output::success(&format!("Created generation {}.", generation.number));
    // 已创建代 {}。
    output::success("Configuration built successfully.");
    // 配置构建成功。
    println!();
    output::info("To activate this configuration, run:");
    // 要激活此配置，请运行：
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
        // 系统配置仅在 Linux 上支持。
    }

    let gen_manager = GenerationManager::new(generations_dir())
        .map_err(|e| format!("Failed to initialize generation manager: {}", e))?;
    // 初始化代管理器失败：{}

    let current = gen_manager
        .current_generation()
        .map_err(|e| format!("Failed to get current generation: {}", e))?;
    // 获取当前代失败：{}

    match current {
        Some(gen_num) => {
            println!("Activating generation {}...", gen_num);
            // 正在激活代 {}...

            let generation = gen_manager
                .load_generation(gen_num)
                .map_err(|e| format!("Failed to load generation: {}", e))?;
            // 加载代失败：{}

            // For now, just report what we would do
            // 目前，只报告我们将要做什么
            println!(
                "Would activate configuration from: {}",
                generation.store_path.display_name()
            );
            // 将从以下位置激活配置：{}
            println!();
            println!("Note: Full activation requires root privileges.");
            // 注意：完整激活需要 root 权限。
            println!("In a real system, this would:");
            // 在真实系统中，这将：
            println!("  - Update /etc configuration files");
            // - 更新 /etc 配置文件
            println!("  - Restart affected services");
            // - 重启受影响的服务
            println!("  - Update the current system profile");
            // - 更新当前系统配置文件

            Ok(())
        }
        None => {
            Err("No configuration has been built yet. Run 'neve config build' first.".to_string())
            // 尚未构建配置。请先运行 'neve config build'。
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
        // 系统配置仅在 Linux 上支持。
    }

    let gen_manager = GenerationManager::new(generations_dir())
        .map_err(|e| format!("Failed to initialize generation manager: {}", e))?;
    // 初始化代管理器失败：{}

    let current = gen_manager
        .current_generation()
        .map_err(|e| format!("Failed to get current generation: {}", e))?;
    // 获取当前代失败：{}

    match current {
        Some(gen_num) if gen_num > 1 => {
            let prev_gen = gen_num - 1;
            println!(
                "Rolling back from generation {} to {}...",
                gen_num, prev_gen
            );
            // 正在从代 {} 回滚到 {}...

            let generation = gen_manager
                .switch_to(prev_gen)
                .map_err(|e| format!("Failed to switch to generation {}: {}", prev_gen, e))?;
            // 切换到代 {} 失败：{}

            println!("Rolled back to generation {}.", generation.number);
            // 已回滚到代 {}。
            println!();
            println!("Note: Full activation requires running 'neve config switch'.");
            // 注意：完整激活需要运行 'neve config switch'。

            Ok(())
        }
        Some(_) => Err("Already at generation 1, cannot rollback further.".to_string()),
        // 已经在代 1，无法进一步回滚。
        None => Err("No configuration has been built yet.".to_string()),
        // 尚未构建配置。
    }
}

/// List all configuration generations.
/// 列出所有配置代。
pub fn list_generations() -> Result<(), String> {
    let gen_manager = GenerationManager::new(generations_dir())
        .map_err(|e| format!("Failed to initialize generation manager: {}", e))?;
    // 初始化代管理器失败：{}

    let current = gen_manager
        .current_generation()
        .map_err(|e| format!("Failed to get current generation: {}", e))?;
    // 获取当前代失败：{}

    let generations = gen_manager
        .list_generations()
        .map_err(|e| format!("Failed to list generations: {}", e))?;
    // 列出代失败：{}

    if generations.is_empty() {
        println!("No configuration generations found.");
        // 未找到配置代。
        println!("Run 'neve config build' to create one.");
        // 运行 'neve config build' 创建一个。
        return Ok(());
    }

    println!("System configuration generations:");
    // 系统配置代：
    println!();

    for generation in generations.iter().rev() {
        let current_marker = if Some(generation.number) == current {
            " (current)"
            // （当前）
        } else {
            ""
        };
        let name = generation.metadata.name.as_deref().unwrap_or("unnamed");
        // 未命名

        println!("  {} - {}{}", generation.number, name, current_marker);

        if let Some(ref desc) = generation.metadata.description {
            println!("      {}", desc);
        }
    }

    Ok(())
}
