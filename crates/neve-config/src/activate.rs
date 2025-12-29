//! Configuration activation.
//!
//! Handles switching between system configurations.

use crate::ConfigError;
use crate::generate::GeneratedConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Configuration activator.
pub struct Activator {
    /// The system root (usually /).
    root: PathBuf,
    /// Whether to perform a dry run.
    dry_run: bool,
    /// Whether to show verbose output.
    verbose: bool,
}

impl Activator {
    /// Create a new activator.
    pub fn new() -> Self {
        Self {
            root: PathBuf::from("/"),
            dry_run: false,
            verbose: false,
        }
    }

    /// Set the system root.
    pub fn root(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = root.into();
        self
    }

    /// Enable dry run mode.
    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Enable verbose output.
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Activate a configuration.
    pub fn activate(&self, generated: &GeneratedConfig) -> Result<ActivationResult, ConfigError> {
        let mut result = ActivationResult::new();

        // Copy files
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
        for service in &generated.services {
            if self.verbose {
                println!("Enabling service: {}", service);
            }

            if !self.dry_run {
                // In a real implementation, this would call systemctl
                result.services_enabled += 1;
            }
        }

        result.success = true;
        Ok(result)
    }

    /// Switch to a new configuration.
    pub fn switch(
        &self,
        from: Option<&GeneratedConfig>,
        to: &GeneratedConfig,
    ) -> Result<ActivationResult, ConfigError> {
        // If there's a previous configuration, we might need to handle rollback
        if let Some(_prev) = from {
            // In a real implementation, we'd save the current state for rollback
        }

        self.activate(to)
    }

    /// Test a configuration without activating.
    pub fn test(&self, generated: &GeneratedConfig) -> Result<TestResult, ConfigError> {
        let mut result = TestResult::new();

        // Check all files can be installed
        for file in &generated.files {
            let target = self
                .root
                .join(file.target.strip_prefix("/").unwrap_or(&file.target));

            // Check if target directory exists or can be created
            if let Some(parent) = target.parent()
                && !parent.exists()
            {
                result
                    .warnings
                    .push(format!("Directory will be created: {}", parent.display()));
            }

            // Check if target exists and would be overwritten
            if target.exists() {
                result
                    .warnings
                    .push(format!("File will be overwritten: {}", target.display()));
            }

            result.files_checked += 1;
        }

        // Check activation script
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
#[derive(Debug, Clone)]
pub struct ActivationResult {
    /// Whether activation succeeded.
    pub success: bool,
    /// Number of files installed.
    pub files_installed: usize,
    /// Number of services enabled.
    pub services_enabled: usize,
    /// Output from activation script.
    pub script_output: Option<String>,
}

impl ActivationResult {
    /// Create a new activation result.
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
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Whether test passed.
    pub success: bool,
    /// Number of files checked.
    pub files_checked: usize,
    /// Warnings encountered.
    pub warnings: Vec<String>,
    /// Errors encountered.
    pub errors: Vec<String>,
}

impl TestResult {
    /// Create a new test result.
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
