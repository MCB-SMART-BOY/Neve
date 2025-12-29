//! The `neve config` commands.

use crate::output;
use crate::platform::{PlatformCapabilities, warn_system_config_unavailable};
use neve_config::{
    generate::Generator,
    generation::{GenerationManager, GenerationMetadata},
    module::Module,
};
use std::path::PathBuf;

/// Get the default configuration file path.
fn default_config_path() -> PathBuf {
    // Look for configuration in standard locations
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
    if let Some(path) = dirs_config_path()
        && path.exists()
    {
        return path;
    }

    PathBuf::from("./configuration.neve")
}

/// Get the user's config directory path.
fn dirs_config_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".config/neve/configuration.neve"))
}

/// Get the generations directory.
fn generations_dir() -> PathBuf {
    std::env::var("NEVE_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/lib/neve"))
}

/// Get the build output directory.
fn build_dir() -> PathBuf {
    std::env::var("NEVE_BUILD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("neve-build"))
}

/// Build system configuration.
pub fn build() -> Result<(), String> {
    // Check platform support
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
    let module = if config_path.exists() {
        Module::load(&config_path).map_err(|e| format!("Failed to load configuration: {}", e))?
    } else {
        output::warning("No configuration file found, using default configuration.");
        Module::new("default")
    };

    // Convert to SystemConfig
    let mut system_config = module
        .to_system_config()
        .map_err(|e| format!("Failed to parse configuration: {}", e))?;

    // Generate configuration files
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
    let gen_manager = GenerationManager::new(generations_dir())
        .map_err(|e| format!("Failed to initialize generation manager: {}", e))?;

    // Create store path from derivation
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
pub fn switch() -> Result<(), String> {
    // Check platform support
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
pub fn rollback() -> Result<(), String> {
    // Check platform support
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
        println!("No configuration generations found.");
        println!("Run 'neve config build' to create one.");
        return Ok(());
    }

    println!("System configuration generations:");
    println!();

    for generation in generations.iter().rev() {
        let current_marker = if Some(generation.number) == current {
            " (current)"
        } else {
            ""
        };
        let name = generation.metadata.name.as_deref().unwrap_or("unnamed");

        println!("  {} - {}{}", generation.number, name, current_marker);

        if let Some(ref desc) = generation.metadata.description {
            println!("      {}", desc);
        }
    }

    Ok(())
}
