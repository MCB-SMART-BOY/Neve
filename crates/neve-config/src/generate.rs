//! Configuration generation.
//!
//! Generates system files and derivations from configuration.

use crate::{ConfigError, SystemConfig};
use neve_derive::{Derivation, StorePath};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// Configuration generator.
pub struct Generator {
    /// Output directory for generated files.
    output_dir: PathBuf,
    /// The system architecture.
    system: String,
}

impl Generator {
    /// Create a new generator.
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            system: current_system(),
        }
    }

    /// Set the target system.
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = system.into();
        self
    }

    /// Generate configuration files.
    pub fn generate(&self, config: &SystemConfig) -> Result<GeneratedConfig, ConfigError> {
        fs::create_dir_all(&self.output_dir)?;
        
        let mut generated = GeneratedConfig::new();
        
        // Generate /etc files
        self.generate_etc(config, &mut generated)?;
        
        // Generate systemd units
        self.generate_services(config, &mut generated)?;
        
        // Generate user configurations
        self.generate_users(config, &mut generated)?;
        
        // Generate environment
        self.generate_environment(config, &mut generated)?;
        
        // Generate activation script
        self.generate_activation_script(config, &mut generated)?;
        
        Ok(generated)
    }

    /// Generate /etc configuration files.
    fn generate_etc(&self, config: &SystemConfig, generated: &mut GeneratedConfig) -> Result<(), ConfigError> {
        let etc_dir = self.output_dir.join("etc");
        fs::create_dir_all(&etc_dir)?;
        
        // /etc/hostname
        if let Some(ref hostname) = config.options.hostname {
            let path = etc_dir.join("hostname");
            fs::write(&path, format!("{}\n", hostname))?;
            generated.files.push(GeneratedFile {
                source: path,
                target: PathBuf::from("/etc/hostname"),
                mode: 0o644,
            });
        }
        
        // /etc/timezone
        if let Some(ref timezone) = config.options.timezone {
            let path = etc_dir.join("timezone");
            fs::write(&path, format!("{}\n", timezone))?;
            generated.files.push(GeneratedFile {
                source: path,
                target: PathBuf::from("/etc/timezone"),
                mode: 0o644,
            });
        }
        
        // /etc/locale.conf
        if let Some(ref locale) = config.options.locale {
            let path = etc_dir.join("locale.conf");
            fs::write(&path, format!("LANG={}\n", locale))?;
            generated.files.push(GeneratedFile {
                source: path,
                target: PathBuf::from("/etc/locale.conf"),
                mode: 0o644,
            });
        }
        
        Ok(())
    }

    /// Generate service configurations.
    fn generate_services(&self, config: &SystemConfig, generated: &mut GeneratedConfig) -> Result<(), ConfigError> {
        let services_dir = self.output_dir.join("services");
        fs::create_dir_all(&services_dir)?;
        
        // Generate a list of enabled services
        let enabled_path = services_dir.join("enabled");
        let content = config.options.services.join("\n");
        fs::write(&enabled_path, format!("{}\n", content))?;
        
        generated.services = config.options.services.clone();
        
        Ok(())
    }

    /// Generate user configurations.
    fn generate_users(&self, config: &SystemConfig, _generated: &mut GeneratedConfig) -> Result<(), ConfigError> {
        let users_dir = self.output_dir.join("users");
        fs::create_dir_all(&users_dir)?;
        
        for user in &config.options.users {
            let user_dir = users_dir.join(&user.name);
            fs::create_dir_all(&user_dir)?;
            
            // User info
            let info = format!(
                "name={}\nhome={}\nshell={}\ngroups={}\n",
                user.name,
                user.home.display(),
                user.shell.as_deref().unwrap_or("/bin/sh"),
                user.groups.join(",")
            );
            fs::write(user_dir.join("info"), info)?;
            
            // User packages
            fs::write(
                user_dir.join("packages"),
                user.packages.join("\n") + "\n"
            )?;
        }
        
        Ok(())
    }

    /// Generate environment configuration.
    fn generate_environment(&self, config: &SystemConfig, generated: &mut GeneratedConfig) -> Result<(), ConfigError> {
        let env_path = self.output_dir.join("environment");
        
        let mut content = String::new();
        for (key, value) in &config.options.environment {
            content.push_str(&format!("export {}=\"{}\"\n", key, value));
        }
        
        fs::write(&env_path, content)?;
        generated.files.push(GeneratedFile {
            source: env_path,
            target: PathBuf::from("/etc/profile.d/neve-env.sh"),
            mode: 0o644,
        });
        
        Ok(())
    }

    /// Generate the activation script.
    fn generate_activation_script(&self, config: &SystemConfig, generated: &mut GeneratedConfig) -> Result<(), ConfigError> {
        let script_path = self.output_dir.join("activate");
        
        let mut script = String::from("#!/bin/sh\n");
        script.push_str("# Neve system activation script\n\n");
        script.push_str(&format!("# Configuration: {}\n", config.name));
        script.push_str(&format!("# Generation: {}\n\n", config.generation));
        
        // Copy etc files
        script.push_str("echo 'Activating configuration...'\n\n");
        
        for file in &generated.files {
            script.push_str(&format!(
                "install -m {:o} {} {}\n",
                file.mode,
                file.source.display(),
                file.target.display()
            ));
        }
        
        // Enable services
        script.push_str("\n# Enable services\n");
        for service in &generated.services {
            script.push_str(&format!("# systemctl enable {}\n", service));
        }
        
        script.push_str("\necho 'Configuration activated.'\n");
        
        fs::write(&script_path, script)?;
        
        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
        }
        
        generated.activation_script = Some(script_path);
        
        Ok(())
    }

    /// Create a derivation for the configuration.
    pub fn to_derivation(&self, config: &SystemConfig) -> Derivation {
        let mut env = BTreeMap::new();
        
        if let Some(ref hostname) = config.options.hostname {
            env.insert("hostname".to_string(), hostname.clone());
        }
        if let Some(ref timezone) = config.options.timezone {
            env.insert("timezone".to_string(), timezone.clone());
        }
        
        env.insert("packages".to_string(), config.options.packages.join(" "));
        env.insert("services".to_string(), config.options.services.join(" "));
        
        Derivation::builder(&config.name, "1.0")
            .system(&self.system)
            .envs(env)
            .build()
    }
}

/// Generated configuration.
#[derive(Debug, Clone)]
pub struct GeneratedConfig {
    /// Generated files.
    pub files: Vec<GeneratedFile>,
    /// Enabled services.
    pub services: Vec<String>,
    /// Activation script path.
    pub activation_script: Option<PathBuf>,
    /// Store path (after registration).
    pub store_path: Option<StorePath>,
}

impl GeneratedConfig {
    /// Create a new generated configuration.
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            services: Vec::new(),
            activation_script: None,
            store_path: None,
        }
    }
}

impl Default for GeneratedConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// A generated file.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// Source path (in build directory).
    pub source: PathBuf,
    /// Target path (on system).
    pub target: PathBuf,
    /// File mode.
    pub mode: u32,
}

/// Get the current system architecture.
fn current_system() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    format!("{}-{}", arch, os)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_generator() {
        let dir = env::temp_dir().join(format!("neve-gen-test-{}", std::process::id()));
        
        let config = SystemConfig::new("test")
            .hostname("test-host")
            .timezone("UTC")
            .service("sshd");
        
        let generator = Generator::new(dir.clone());
        let generated = generator.generate(&config).unwrap();
        
        assert!(!generated.files.is_empty());
        assert!(generated.activation_script.is_some());
        assert_eq!(generated.services, vec!["sshd"]);
        
        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_to_derivation() {
        let dir = env::temp_dir().join("neve-drv-test");
        let config = SystemConfig::new("my-config")
            .hostname("my-host")
            .package("vim")
            .service("sshd");
        
        let generator = Generator::new(dir);
        let drv = generator.to_derivation(&config);
        
        assert_eq!(drv.name, "my-config");
        assert!(drv.env.contains_key("hostname"));
        assert!(drv.env.contains_key("packages"));
    }
}
