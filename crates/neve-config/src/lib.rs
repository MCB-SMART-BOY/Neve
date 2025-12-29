//! System configuration for Neve.
//! Neve 的系统配置。
//!
//! This crate provides functionality for:
//! 本 crate 提供以下功能：
//!
//! - Evaluating system configuration files / 评估系统配置文件
//! - Generating system configurations / 生成系统配置
//! - Activating and switching configurations / 激活和切换配置
//! - Managing configuration generations / 管理配置代

pub mod activate;
pub mod flake;
pub mod generate;
pub mod generation;
pub mod module;

use neve_derive::StorePath;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during configuration.
/// 配置过程中可能发生的错误。
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
/// 系统配置。
#[derive(Debug, Clone)]
pub struct SystemConfig {
    /// The configuration name. / 配置名称。
    pub name: String,
    /// The store path of the built configuration. / 构建后配置的存储路径。
    pub store_path: Option<StorePath>,
    /// The configuration generation number. / 配置代号。
    pub generation: u64,
    /// Configuration options. / 配置选项。
    pub options: ConfigOptions,
}

/// Configuration options.
/// 配置选项。
#[derive(Debug, Clone, Default)]
pub struct ConfigOptions {
    /// Hostname. / 主机名。
    pub hostname: Option<String>,
    /// Timezone. / 时区。
    pub timezone: Option<String>,
    /// Locale. / 语言区域。
    pub locale: Option<String>,
    /// Enabled services. / 启用的服务。
    pub services: Vec<String>,
    /// System packages. / 系统包。
    pub packages: Vec<String>,
    /// User configurations. / 用户配置。
    pub users: Vec<UserConfig>,
    /// Environment variables. / 环境变量。
    pub environment: Vec<(String, String)>,
}

/// User configuration.
/// 用户配置。
#[derive(Debug, Clone)]
pub struct UserConfig {
    /// Username. / 用户名。
    pub name: String,
    /// User's home directory. / 用户主目录。
    pub home: PathBuf,
    /// User's shell. / 用户 shell。
    pub shell: Option<String>,
    /// User's groups. / 用户组。
    pub groups: Vec<String>,
    /// User's packages. / 用户包。
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
