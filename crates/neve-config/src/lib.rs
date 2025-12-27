//! System configuration for Neve.
//!
//! This crate provides functionality for:
//! - Evaluating system configuration files
//! - Generating system configurations
//! - Activating and switching configurations
//! - Managing configuration generations

pub mod module;
pub mod generate;
pub mod activate;
pub mod generation;
pub mod flake;

use neve_derive::StorePath;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("evaluation error: {0}")]
    Eval(String),
    
    #[error("build error: {0}")]
    Build(#[from] neve_builder::BuildError),
    
    #[error("store error: {0}")]
    Store(#[from] neve_store::StoreError),
    
    #[error("module error: {0}")]
    Module(String),
    
    #[error("activation error: {0}")]
    Activation(String),
    
    #[error("configuration not found: {0}")]
    NotFound(String),
    
    #[error("invalid configuration: {0}")]
    Invalid(String),
    
    #[error("flake error: {0}")]
    Flake(String),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// A system configuration.
#[derive(Debug, Clone)]
pub struct SystemConfig {
    /// The configuration name.
    pub name: String,
    /// The store path of the built configuration.
    pub store_path: Option<StorePath>,
    /// The configuration generation number.
    pub generation: u64,
    /// Configuration options.
    pub options: ConfigOptions,
}

/// Configuration options.
#[derive(Debug, Clone, Default)]
pub struct ConfigOptions {
    /// Hostname.
    pub hostname: Option<String>,
    /// Timezone.
    pub timezone: Option<String>,
    /// Locale.
    pub locale: Option<String>,
    /// Enabled services.
    pub services: Vec<String>,
    /// System packages.
    pub packages: Vec<String>,
    /// User configurations.
    pub users: Vec<UserConfig>,
    /// Environment variables.
    pub environment: Vec<(String, String)>,
}

/// User configuration.
#[derive(Debug, Clone)]
pub struct UserConfig {
    /// Username.
    pub name: String,
    /// User's home directory.
    pub home: PathBuf,
    /// User's shell.
    pub shell: Option<String>,
    /// User's groups.
    pub groups: Vec<String>,
    /// User's packages.
    pub packages: Vec<String>,
}

impl SystemConfig {
    /// Create a new system configuration.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            store_path: None,
            generation: 0,
            options: ConfigOptions::default(),
        }
    }

    /// Set the hostname.
    pub fn hostname(mut self, hostname: impl Into<String>) -> Self {
        self.options.hostname = Some(hostname.into());
        self
    }

    /// Set the timezone.
    pub fn timezone(mut self, tz: impl Into<String>) -> Self {
        self.options.timezone = Some(tz.into());
        self
    }

    /// Add a service.
    pub fn service(mut self, service: impl Into<String>) -> Self {
        self.options.services.push(service.into());
        self
    }

    /// Add a package.
    pub fn package(mut self, package: impl Into<String>) -> Self {
        self.options.packages.push(package.into());
        self
    }

    /// Add a user.
    pub fn user(mut self, user: UserConfig) -> Self {
        self.options.users.push(user);
        self
    }
}

impl UserConfig {
    /// Create a new user configuration.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            home: PathBuf::from(format!("/home/{}", name)),
            name,
            shell: None,
            groups: Vec::new(),
            packages: Vec::new(),
        }
    }

    /// Set the shell.
    pub fn shell(mut self, shell: impl Into<String>) -> Self {
        self.shell = Some(shell.into());
        self
    }

    /// Add a group.
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.groups.push(group.into());
        self
    }

    /// Add a package.
    pub fn package(mut self, package: impl Into<String>) -> Self {
        self.packages.push(package.into());
        self
    }
}

