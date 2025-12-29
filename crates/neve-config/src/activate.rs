//! Configuration activation.
//! 配置激活。
//!
//! Handles switching between system configurations.
//! 处理系统配置之间的切换。

use crate::ConfigError;
use crate::generate::GeneratedConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Configuration activator.
/// 配置激活器。
pub struct Activator {
    /// The system root (usually /). / 系统根目录（通常是 /）。
    root: PathBuf,
    /// Whether to perform a dry run. / 是否执行试运行。
    dry_run: bool,
    /// Whether to show verbose output. / 是否显示详细输出。
    verbose: bool,
}

impl Activator {
    /// Create a new activator.
    /// 创建新的激活器。
    pub fn new() -> Self {
        Self {
            root: PathBuf::from("/"),
            dry_run: false,
            verbose: false,
        }
    }

    /// Set the system root.
    /// 设置系统根目录。
    pub fn root(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = root.into();
        self
    }

    /// Enable dry run mode.
    /// 启用试运行模式。
    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Enable verbose output.
    /// 启用详细输出。
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Activate a configuration.
    /// 激活配置。
    pub fn activate(&self, generated: &GeneratedConfig) -> Result<ActivationResult, ConfigError> {
        let mut result = ActivationResult::new();

        // Copy files
        // 复制文件
        for file in &generated.files {
            let target = self
                .root
                .join(file.target.strip_prefix("/").unwrap_or(&file.target));

            if self.verbose {
                println!(
                    "Installing {} -> {}",
                    file.source.display(),
                    target.display()
                );
            }

            if !self.dry_run {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&file.source, &target)?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&target, fs::Permissions::from_mode(file.mode))?;
                }
            }

            result.files_installed += 1;
        }

        // Run activation script
        // 运行激活脚本
        if let Some(ref script) = generated.activation_script {
            if self.verbose {
                println!("Running activation script: {}", script.display());
            }

            if !self.dry_run {
                let output = Command::new(script).env("NEVE_ROOT", &self.root).output()?;

                if !output.status.success() {
                    return Err(ConfigError::Activation(format!(
                        "activation script failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )));
                }

                result.script_output = Some(String::from_utf8_lossy(&output.stdout).into_owned());
            }
        }

        // Enable services
        // 启用服务
        for service in &generated.services {
            if self.verbose {
                println!("Enabling service: {}", service);
            }

            if !self.dry_run {
                // In a real implementation, this would call systemctl
                // 在实际实现中，这会调用 systemctl
                result.services_enabled += 1;
            }
        }

        result.success = true;
        Ok(result)
    }

    /// Switch to a new configuration.
    /// 切换到新配置。
    pub fn switch(
        &self,
        from: Option<&GeneratedConfig>,
        to: &GeneratedConfig,
    ) -> Result<ActivationResult, ConfigError> {
        // If there's a previous configuration, we might need to handle rollback
        // 如果有之前的配置，我们可能需要处理回滚
        if let Some(_prev) = from {
            // In a real implementation, we'd save the current state for rollback
            // 在实际实现中，我们会保存当前状态以便回滚
        }

        self.activate(to)
    }

    /// Test a configuration without activating.
    /// 测试配置但不激活。
    pub fn test(&self, generated: &GeneratedConfig) -> Result<TestResult, ConfigError> {
        let mut result = TestResult::new();

        // Check all files can be installed
        // 检查所有文件是否可以安装
        for file in &generated.files {
            let target = self
                .root
                .join(file.target.strip_prefix("/").unwrap_or(&file.target));

            // Check if target directory exists or can be created
            // 检查目标目录是否存在或可以创建
            if let Some(parent) = target.parent()
                && !parent.exists()
            {
                result
                    .warnings
                    .push(format!("Directory will be created: {}", parent.display()));
            }

            // Check if target exists and would be overwritten
            // 检查目标是否存在并将被覆盖
            if target.exists() {
                result
                    .warnings
                    .push(format!("File will be overwritten: {}", target.display()));
            }

            result.files_checked += 1;
        }

        // Check activation script
        // 检查激活脚本
        if let Some(ref script) = generated.activation_script
            && !script.exists()
        {
            result
                .errors
                .push(format!("Activation script not found: {}", script.display()));
        }

        result.success = result.errors.is_empty();
        Ok(result)
    }
}

impl Default for Activator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of activation.
/// 激活结果。
#[derive(Debug, Clone)]
pub struct ActivationResult {
    /// Whether activation succeeded. / 激活是否成功。
    pub success: bool,
    /// Number of files installed. / 已安装的文件数。
    pub files_installed: usize,
    /// Number of services enabled. / 已启用的服务数。
    pub services_enabled: usize,
    /// Output from activation script. / 激活脚本的输出。
    pub script_output: Option<String>,
}

impl ActivationResult {
    /// Create a new activation result.
    /// 创建新的激活结果。
    pub fn new() -> Self {
        Self {
            success: false,
            files_installed: 0,
            services_enabled: 0,
            script_output: None,
        }
    }
}

impl Default for ActivationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of configuration test.
/// 配置测试结果。
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Whether test passed. / 测试是否通过。
    pub success: bool,
    /// Number of files checked. / 已检查的文件数。
    pub files_checked: usize,
    /// Warnings encountered. / 遇到的警告。
    pub warnings: Vec<String>,
    /// Errors encountered. / 遇到的错误。
    pub errors: Vec<String>,
}

impl TestResult {
    /// Create a new test result.
    /// 创建新的测试结果。
    pub fn new() -> Self {
        Self {
            success: false,
            files_checked: 0,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }
}

impl Default for TestResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Rollback to a previous configuration.
/// 回滚到之前的配置。
pub fn rollback(generation: u64, generations_dir: &Path) -> Result<PathBuf, ConfigError> {
    let gen_path = generations_dir.join(format!("generation-{}", generation));

    if !gen_path.exists() {
        return Err(ConfigError::NotFound(format!(
            "generation {} not found",
            generation
        )));
    }

    Ok(gen_path)
}
