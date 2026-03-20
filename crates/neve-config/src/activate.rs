//! Configuration activation.
//! 配置激活。
//!
//! Handles switching between system configurations.
//! 处理系统配置之间的切换。

use crate::ConfigError;
use crate::generate::GeneratedConfig;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
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
        let mut tx = ActivationTransaction::new();

        let apply_result = (|| -> Result<(), ConfigError> {
            // Copy files
            // 复制文件
            for file in &generated.files {
                let target = resolve_target_under_root(&self.root, &file.target)?;

                if self.verbose {
                    println!(
                        "Installing {} -> {}",
                        file.source.display(),
                        target.display()
                    );
                }

                if !self.dry_run {
                    tx.install_file(&file.source, &target, file.mode)?;
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

                    result.script_output =
                        Some(String::from_utf8_lossy(&output.stdout).into_owned());
                }
            }

            // Enable services
            // 启用服务
            for service in &generated.services {
                if self.verbose {
                    println!("Enabling service: {}", service);
                }

                if !self.dry_run {
                    self.enable_service(service, &mut tx)?;
                }
                result.services_enabled += 1;
            }

            Ok(())
        })();

        if let Err(err) = apply_result {
            if !self.dry_run {
                if let Err(rollback_err) = tx.rollback() {
                    return Err(ConfigError::Activation(format!(
                        "activation failed: {}; rollback failed: {}",
                        err, rollback_err
                    )));
                }
                if self.verbose {
                    println!("Activation failed; changes rolled back.");
                }
            }
            return Err(err);
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
        match self.activate(to) {
            Ok(result) => Ok(result),
            Err(err) => {
                let Some(prev) = from else {
                    return Err(err);
                };
                if self.dry_run {
                    return Err(err);
                }
                if self.verbose {
                    println!("Switch failed, rolling back to previous configuration...");
                }
                match self.activate(prev) {
                    Ok(_) => Err(ConfigError::Activation(format!(
                        "switch failed and rolled back to previous configuration: {}",
                        err
                    ))),
                    Err(rollback_err) => Err(ConfigError::Activation(format!(
                        "switch failed: {}; rollback to previous configuration failed: {}",
                        err, rollback_err
                    ))),
                }
            }
        }
    }

    /// Test a configuration without activating.
    /// 测试配置但不激活。
    pub fn test(&self, generated: &GeneratedConfig) -> Result<TestResult, ConfigError> {
        let mut result = TestResult::new();

        // Check all files can be installed
        // 检查所有文件是否可以安装
        for file in &generated.files {
            let target = resolve_target_under_root(&self.root, &file.target)?;

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

impl Activator {
    /// Enable a systemd service by creating a wants symlink.
    /// 通过创建 wants 符号链接启用 systemd 服务。
    fn enable_service(
        &self,
        service: &str,
        tx: &mut ActivationTransaction,
    ) -> Result<(), ConfigError> {
        if !is_valid_service_name(service) {
            return Err(ConfigError::Activation(format!(
                "invalid service name: {}",
                service
            )));
        }

        let unit_rel = PathBuf::from(format!("/etc/systemd/system/{}.service", service));
        let wants_rel = PathBuf::from(format!(
            "/etc/systemd/system/multi-user.target.wants/{}.service",
            service
        ));
        let unit_path = resolve_target_under_root(&self.root, &unit_rel)?;
        let wants_path = resolve_target_under_root(&self.root, &wants_rel)?;

        if !unit_path.exists() {
            return Err(ConfigError::Activation(format!(
                "service unit file does not exist for '{}': {}",
                service,
                unit_path.display()
            )));
        }

        // Use a relative target inside multi-user.target.wants for portability
        // across chroots and alternate roots.
        // 在 multi-user.target.wants 中使用相对目标，便于 chroot 与自定义 root。
        let link_target = PathBuf::from(format!("../{}.service", service));
        tx.install_symlink(&wants_path, &link_target)
    }
}

/// Backup entry for a touched path during activation.
/// 激活过程中被修改路径的备份条目。
#[derive(Debug)]
enum PathBackup {
    File {
        path: PathBuf,
        bytes: Vec<u8>,
        mode: Option<u32>,
    },
    Symlink {
        path: PathBuf,
        target: PathBuf,
    },
}

/// File-system transaction used by activation for rollback.
/// 激活使用的文件系统事务（用于回滚）。
#[derive(Debug, Default)]
struct ActivationTransaction {
    created_paths: Vec<PathBuf>,
    created_set: HashSet<PathBuf>,
    backups: Vec<PathBackup>,
    backup_set: HashSet<PathBuf>,
}

impl ActivationTransaction {
    fn new() -> Self {
        Self::default()
    }

    fn install_file(&mut self, source: &Path, target: &Path, mode: u32) -> Result<(), ConfigError> {
        self.capture_before_write(target)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, target)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(target, fs::Permissions::from_mode(mode))?;
        }
        Ok(())
    }

    fn install_symlink(&mut self, path: &Path, target: &Path) -> Result<(), ConfigError> {
        self.capture_before_write(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Ok(meta) = fs::symlink_metadata(path) {
            if meta.is_dir() {
                return Err(ConfigError::Activation(format!(
                    "cannot overwrite directory with symlink: {}",
                    path.display()
                )));
            }
            fs::remove_file(path)?;
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, path)?;
        }
        #[cfg(not(unix))]
        {
            fs::write(path, target.to_string_lossy().as_bytes())?;
        }
        Ok(())
    }

    fn capture_before_write(&mut self, path: &Path) -> Result<(), ConfigError> {
        if self.created_set.contains(path) || self.backup_set.contains(path) {
            return Ok(());
        }
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    let target = fs::read_link(path)?;
                    self.backups.push(PathBackup::Symlink {
                        path: path.to_path_buf(),
                        target,
                    });
                } else if metadata.is_file() {
                    let bytes = fs::read(path)?;
                    #[cfg(unix)]
                    let mode = {
                        use std::os::unix::fs::PermissionsExt;
                        Some(metadata.permissions().mode())
                    };
                    #[cfg(not(unix))]
                    let mode = None;
                    self.backups.push(PathBackup::File {
                        path: path.to_path_buf(),
                        bytes,
                        mode,
                    });
                } else {
                    return Err(ConfigError::Activation(format!(
                        "unsupported target path type: {}",
                        path.display()
                    )));
                }
                self.backup_set.insert(path.to_path_buf());
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                self.created_set.insert(path.to_path_buf());
                self.created_paths.push(path.to_path_buf());
            }
            Err(err) => return Err(ConfigError::Io(err)),
        }
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), ConfigError> {
        for path in self.created_paths.iter().rev() {
            match fs::symlink_metadata(path) {
                Ok(meta) => {
                    if meta.is_dir() {
                        fs::remove_dir_all(path)?;
                    } else {
                        fs::remove_file(path)?;
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(ConfigError::Io(err)),
            }
        }

        for backup in self.backups.iter().rev() {
            match backup {
                PathBackup::File { path, bytes, mode } => {
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    match fs::symlink_metadata(path) {
                        Ok(meta) => {
                            if meta.is_dir() {
                                fs::remove_dir_all(path)?;
                            } else {
                                fs::remove_file(path)?;
                            }
                        }
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                        Err(err) => return Err(ConfigError::Io(err)),
                    }
                    fs::write(path, bytes)?;
                    #[cfg(unix)]
                    if let Some(saved_mode) = mode {
                        use std::os::unix::fs::PermissionsExt;
                        fs::set_permissions(path, fs::Permissions::from_mode(*saved_mode))?;
                    }
                }
                PathBackup::Symlink { path, target } => {
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    match fs::symlink_metadata(path) {
                        Ok(meta) => {
                            if meta.is_dir() {
                                fs::remove_dir_all(path)?;
                            } else {
                                fs::remove_file(path)?;
                            }
                        }
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                        Err(err) => return Err(ConfigError::Io(err)),
                    }
                    #[cfg(unix)]
                    {
                        std::os::unix::fs::symlink(target, path)?;
                    }
                    #[cfg(not(unix))]
                    {
                        fs::write(path, target.to_string_lossy().as_bytes())?;
                    }
                }
            }
        }

        Ok(())
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

/// Resolve a target path under the configured root and reject path traversal.
/// 在配置的根目录下解析目标路径并拒绝路径穿越。
fn resolve_target_under_root(root: &Path, target: &Path) -> Result<PathBuf, ConfigError> {
    let mut rel = PathBuf::new();
    for comp in target.components() {
        match comp {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(seg) => rel.push(seg),
            Component::ParentDir => {
                return Err(ConfigError::Activation(format!(
                    "invalid target path (parent traversal): {}",
                    target.display()
                )));
            }
            Component::Prefix(_) => {
                return Err(ConfigError::Activation(format!(
                    "invalid target path prefix: {}",
                    target.display()
                )));
            }
        }
    }
    Ok(root.join(rel))
}

/// Validate service unit name.
/// 验证服务单元名称。
fn is_valid_service_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '@'))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::GeneratedFile;
    use tempfile::tempdir;

    fn write_exec_script(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(path, content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    }

    #[test]
    fn test_activate_enables_service_symlink() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempdir()?;
        let src = root.path().join("demo.service.src");
        fs::write(&src, "[Service]\nExecStart=/bin/true\n")?;

        let mut generated = GeneratedConfig::new();
        generated.files.push(GeneratedFile {
            source: src,
            target: PathBuf::from("/etc/systemd/system/demo.service"),
            mode: 0o644,
        });
        generated.services.push("demo".to_string());

        let activator = Activator::new().root(root.path());
        let result = activator.activate(&generated)?;
        assert!(result.success);
        assert_eq!(result.services_enabled, 1);

        let wants = root
            .path()
            .join("etc/systemd/system/multi-user.target.wants/demo.service");
        assert!(fs::symlink_metadata(&wants).is_ok());
        #[cfg(unix)]
        {
            let link_target = fs::read_link(&wants)?;
            assert_eq!(link_target, PathBuf::from("../demo.service"));
        }

        Ok(())
    }

    #[test]
    fn test_activate_rolls_back_files_on_script_failure() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = tempdir()?;
        let etc_dir = root.path().join("etc");
        fs::create_dir_all(&etc_dir)?;

        let existing_target = etc_dir.join("existing.conf");
        fs::write(&existing_target, "old-content\n")?;

        let src_existing = root.path().join("existing.new");
        fs::write(&src_existing, "new-content\n")?;
        let src_new = root.path().join("new.conf.src");
        fs::write(&src_new, "new-file\n")?;

        let script = root.path().join("fail.sh");
        write_exec_script(&script, "#!/bin/sh\necho fail >&2\nexit 1\n")?;

        let mut generated = GeneratedConfig::new();
        generated.files.push(GeneratedFile {
            source: src_existing,
            target: PathBuf::from("/etc/existing.conf"),
            mode: 0o644,
        });
        generated.files.push(GeneratedFile {
            source: src_new,
            target: PathBuf::from("/etc/new.conf"),
            mode: 0o644,
        });
        generated.activation_script = Some(script);

        let activator = Activator::new().root(root.path());
        let err = activator
            .activate(&generated)
            .expect_err("activation should fail");
        let msg = err.to_string();
        assert!(msg.contains("activation script failed"));

        let existing_after = fs::read_to_string(&existing_target)?;
        assert_eq!(existing_after, "old-content\n");
        assert!(!root.path().join("etc/new.conf").exists());

        Ok(())
    }

    #[test]
    fn test_activate_rejects_invalid_service_name() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempdir()?;
        let mut generated = GeneratedConfig::new();
        generated.services.push("../bad".to_string());

        let activator = Activator::new().root(root.path());
        let err = activator
            .activate(&generated)
            .expect_err("activation should fail");
        assert!(err.to_string().contains("invalid service name"));
        Ok(())
    }
}
