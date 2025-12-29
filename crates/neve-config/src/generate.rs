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

/// A systemd service unit definition.
#[derive(Debug, Clone)]
pub struct ServiceUnit {
    /// Service name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Service type (simple, forking, oneshot, etc.).
    pub service_type: String,
    /// Command to execute.
    pub exec_start: String,
    /// Command to run before exec_start.
    pub exec_start_pre: Option<String>,
    /// Command to run after service stops.
    pub exec_stop: Option<String>,
    /// User to run as.
    pub user: Option<String>,
    /// Group to run as.
    pub group: Option<String>,
    /// Working directory.
    pub working_directory: Option<String>,
    /// Environment variables.
    pub environment: Vec<(String, String)>,
    /// Restart policy (always, on-failure, no).
    pub restart: String,
    /// Dependencies (After=).
    pub after: Vec<String>,
    /// Required dependencies (Requires=).
    pub requires: Vec<String>,
    /// Wanted by targets.
    pub wanted_by: Vec<String>,
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
    fn generate_etc(
        &self,
        config: &SystemConfig,
        generated: &mut GeneratedConfig,
    ) -> Result<(), ConfigError> {
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
    fn generate_services(
        &self,
        config: &SystemConfig,
        generated: &mut GeneratedConfig,
    ) -> Result<(), ConfigError> {
        let services_dir = self.output_dir.join("etc/systemd/system");
        fs::create_dir_all(&services_dir)?;

        // Generate systemd units for each service
        for service_name in &config.options.services {
            let unit = self.create_service_unit(service_name);
            let unit_content = self.render_service_unit(&unit);

            let unit_path = services_dir.join(format!("{}.service", service_name));
            fs::write(&unit_path, &unit_content)?;

            generated.files.push(GeneratedFile {
                source: unit_path,
                target: PathBuf::from(format!("/etc/systemd/system/{}.service", service_name)),
                mode: 0o644,
            });

            // Create symlink for multi-user.target.wants
            let wants_dir = services_dir.join("multi-user.target.wants");
            fs::create_dir_all(&wants_dir)?;

            #[cfg(unix)]
            {
                let link_path = wants_dir.join(format!("{}.service", service_name));
                let target = format!("/etc/systemd/system/{}.service", service_name);
                // Remove existing symlink if it exists
                let _ = fs::remove_file(&link_path);
                std::os::unix::fs::symlink(&target, &link_path)?;
            }
        }

        generated.services = config.options.services.clone();

        Ok(())
    }

    /// Create a service unit definition for a known service.
    fn create_service_unit(&self, name: &str) -> ServiceUnit {
        // Default service configuration
        let mut unit = ServiceUnit {
            name: name.to_string(),
            description: format!("{} service", name),
            service_type: "simple".to_string(),
            exec_start: format!("/usr/bin/{}", name),
            exec_start_pre: None,
            exec_stop: None,
            user: None,
            group: None,
            working_directory: None,
            environment: Vec::new(),
            restart: "on-failure".to_string(),
            after: vec!["network.target".to_string()],
            requires: Vec::new(),
            wanted_by: vec!["multi-user.target".to_string()],
        };

        // Customize known services
        match name {
            "sshd" | "ssh" => {
                unit.description = "OpenSSH Daemon".to_string();
                unit.exec_start = "/usr/bin/sshd -D".to_string();
                unit.restart = "always".to_string();
            }
            "docker" => {
                unit.description = "Docker Application Container Engine".to_string();
                unit.exec_start = "/usr/bin/dockerd".to_string();
                unit.after.push("containerd.service".to_string());
                unit.requires.push("containerd.service".to_string());
            }
            "nginx" => {
                unit.description = "Nginx HTTP Server".to_string();
                unit.exec_start = "/usr/bin/nginx -g 'daemon off;'".to_string();
                unit.exec_start_pre = Some("/usr/bin/nginx -t".to_string());
            }
            "postgresql" | "postgres" => {
                unit.description = "PostgreSQL Database Server".to_string();
                unit.exec_start = "/usr/bin/postgres -D /var/lib/postgresql/data".to_string();
                unit.user = Some("postgres".to_string());
                unit.group = Some("postgres".to_string());
            }
            "redis" => {
                unit.description = "Redis In-Memory Data Store".to_string();
                unit.exec_start = "/usr/bin/redis-server /etc/redis.conf".to_string();
                unit.user = Some("redis".to_string());
            }
            _ => {}
        }

        unit
    }

    /// Render a service unit to a string.
    fn render_service_unit(&self, unit: &ServiceUnit) -> String {
        let mut content = String::new();

        // [Unit] section
        content.push_str("[Unit]\n");
        content.push_str(&format!("Description={}\n", unit.description));
        if !unit.after.is_empty() {
            content.push_str(&format!("After={}\n", unit.after.join(" ")));
        }
        if !unit.requires.is_empty() {
            content.push_str(&format!("Requires={}\n", unit.requires.join(" ")));
        }
        content.push('\n');

        // [Service] section
        content.push_str("[Service]\n");
        content.push_str(&format!("Type={}\n", unit.service_type));
        if let Some(ref pre) = unit.exec_start_pre {
            content.push_str(&format!("ExecStartPre={}\n", pre));
        }
        content.push_str(&format!("ExecStart={}\n", unit.exec_start));
        if let Some(ref stop) = unit.exec_stop {
            content.push_str(&format!("ExecStop={}\n", stop));
        }
        if let Some(ref user) = unit.user {
            content.push_str(&format!("User={}\n", user));
        }
        if let Some(ref group) = unit.group {
            content.push_str(&format!("Group={}\n", group));
        }
        if let Some(ref wd) = unit.working_directory {
            content.push_str(&format!("WorkingDirectory={}\n", wd));
        }
        for (key, value) in &unit.environment {
            content.push_str(&format!("Environment=\"{}={}\"\n", key, value));
        }
        content.push_str(&format!("Restart={}\n", unit.restart));
        content.push('\n');

        // [Install] section
        content.push_str("[Install]\n");
        if !unit.wanted_by.is_empty() {
            content.push_str(&format!("WantedBy={}\n", unit.wanted_by.join(" ")));
        }

        content
    }

    /// Generate user configurations.
    fn generate_users(
        &self,
        config: &SystemConfig,
        generated: &mut GeneratedConfig,
    ) -> Result<(), ConfigError> {
        let etc_dir = self.output_dir.join("etc");
        fs::create_dir_all(&etc_dir)?;

        // Generate passwd entries
        let mut passwd_content = String::new();
        // System users
        passwd_content.push_str("root:x:0:0:root:/root:/bin/bash\n");
        passwd_content.push_str("nobody:x:65534:65534:Nobody:/nonexistent:/usr/sbin/nologin\n");

        // User-defined users (starting from UID 1000)
        let mut uid = 1000;
        for user in &config.options.users {
            let shell = user.shell.as_deref().unwrap_or("/bin/sh");
            let home = user.home.display();
            passwd_content.push_str(&format!(
                "{}:x:{}:{}:{}:{}:{}\n",
                user.name, uid, uid, user.name, home, shell
            ));
            uid += 1;
        }

        let passwd_path = etc_dir.join("passwd");
        fs::write(&passwd_path, &passwd_content)?;
        generated.files.push(GeneratedFile {
            source: passwd_path,
            target: PathBuf::from("/etc/passwd"),
            mode: 0o644,
        });

        // Generate group entries
        let mut group_content = String::new();
        group_content.push_str("root:x:0:\n");
        group_content.push_str("wheel:x:10:\n");
        group_content.push_str("users:x:100:\n");
        group_content.push_str("nobody:x:65534:\n");

        // Add user groups
        let mut gid = 1000;
        for user in &config.options.users {
            // Primary group
            group_content.push_str(&format!("{}:x:{}:\n", user.name, gid));
            gid += 1;
        }

        // Add users to supplementary groups
        let mut group_members: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for user in &config.options.users {
            for group in &user.groups {
                group_members
                    .entry(group.clone())
                    .or_default()
                    .push(user.name.clone());
            }
        }

        // Update group entries with members
        for (group, members) in &group_members {
            if !members.is_empty() {
                // For wheel and other predefined groups, we need to update them
                let members_str = members.join(",");
                if group == "wheel" {
                    group_content = group_content
                        .replace("wheel:x:10:\n", &format!("wheel:x:10:{}\n", members_str));
                } else if group == "docker" {
                    group_content.push_str(&format!("docker:x:999:{}\n", members_str));
                }
            }
        }

        let group_path = etc_dir.join("group");
        fs::write(&group_path, &group_content)?;
        generated.files.push(GeneratedFile {
            source: group_path,
            target: PathBuf::from("/etc/group"),
            mode: 0o644,
        });

        // Generate shadow entries (placeholder - actual passwords would be hashed)
        let mut shadow_content = String::new();
        shadow_content.push_str("root:!:19000:0:99999:7:::\n");

        for user in &config.options.users {
            // Locked account by default (! prefix)
            shadow_content.push_str(&format!("{}:!:19000:0:99999:7:::\n", user.name));
        }

        let shadow_path = etc_dir.join("shadow");
        fs::write(&shadow_path, &shadow_content)?;
        generated.files.push(GeneratedFile {
            source: shadow_path,
            target: PathBuf::from("/etc/shadow"),
            mode: 0o640,
        });

        // Create user home directory structure info
        let users_dir = self.output_dir.join("users");
        fs::create_dir_all(&users_dir)?;

        for user in &config.options.users {
            let user_dir = users_dir.join(&user.name);
            fs::create_dir_all(&user_dir)?;

            // User info for activation script
            let info = format!(
                "name={}\nhome={}\nshell={}\ngroups={}\n",
                user.name,
                user.home.display(),
                user.shell.as_deref().unwrap_or("/bin/sh"),
                user.groups.join(",")
            );
            fs::write(user_dir.join("info"), info)?;

            // User packages
            fs::write(user_dir.join("packages"), user.packages.join("\n") + "\n")?;
        }

        Ok(())
    }

    /// Generate environment configuration.
    fn generate_environment(
        &self,
        config: &SystemConfig,
        generated: &mut GeneratedConfig,
    ) -> Result<(), ConfigError> {
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
    fn generate_activation_script(
        &self,
        config: &SystemConfig,
        generated: &mut GeneratedConfig,
    ) -> Result<(), ConfigError> {
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
