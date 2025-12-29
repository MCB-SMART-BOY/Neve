//! Build executor.
//! 构建执行器。
//!
//! Executes derivation builds in sandboxed environments.
//! 在沙箱环境中执行派生构建。

use crate::sandbox::{Sandbox, SandboxConfig};
use crate::{BuildError, BuilderConfig};
use neve_derive::{Derivation, Hash, StorePath};
use neve_store::Store;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Build executor.
/// 构建执行器。
pub struct BuildExecutor<'a> {
    /// Store reference. / 存储引用。
    store: &'a Store,
    /// Builder configuration. / 构建器配置。
    config: &'a BuilderConfig,
}

impl<'a> BuildExecutor<'a> {
    /// Create a new build executor.
    /// 创建新的构建执行器。
    pub fn new(store: &'a Store, config: &'a BuilderConfig) -> Self {
        Self { store, config }
    }

    /// Execute a derivation build.
    /// 执行派生构建。
    pub fn execute(
        &self,
        drv: &Derivation,
    ) -> Result<(HashMap<String, StorePath>, String), BuildError> {
        // Create temporary build directory
        // 创建临时构建目录
        let build_id = format!("{}-{}", drv.name, uuid_simple());
        let build_root = self.config.temp_dir.join(&build_id);
        fs::create_dir_all(&build_root)?;

        // Set up sandbox
        // 设置沙箱
        let sandbox_config = SandboxConfig::new(build_root.clone());
        let sandbox = Sandbox::new(sandbox_config)?;

        // Create tmp directory inside build
        // 在构建目录内创建 tmp 目录
        fs::create_dir_all(sandbox.build_dir().join("tmp"))?;

        // Prepare environment
        // 准备环境变量
        let env = self.prepare_env(drv, &sandbox)?;

        // Set up input symlinks
        // 设置输入符号链接
        self.setup_inputs(drv, &sandbox)?;

        // Create output directories
        // 创建输出目录
        let output_dirs = self.create_output_dirs(drv, &sandbox)?;

        // Execute the builder
        // 执行构建器
        let output = sandbox.execute(&drv.builder, &drv.args, &env)?;

        let log = format!(
            "=== stdout ===\n{}\n=== stderr ===\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        if !output.status.success() {
            if self.config.keep_failed {
                eprintln!(
                    "Build failed. Keeping build directory: {}",
                    build_root.display()
                );
            } else {
                let _ = sandbox.cleanup();
            }
            return Err(BuildError::BuildFailed(format!(
                "builder exited with status {}\n{}",
                output.status, log
            )));
        }

        // Collect outputs
        // 收集输出
        let outputs = self.collect_outputs(drv, &output_dirs)?;

        // Clean up
        // 清理
        if !self.config.keep_failed {
            let _ = sandbox.cleanup();
        }

        Ok((outputs, log))
    }

    /// Prepare environment variables for the build.
    /// 为构建准备环境变量。
    fn prepare_env(
        &self,
        drv: &Derivation,
        sandbox: &Sandbox,
    ) -> Result<HashMap<String, String>, BuildError> {
        // Convert BTreeMap to HashMap
        // 将 BTreeMap 转换为 HashMap
        let mut env: HashMap<String, String> = drv
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Standard build environment variables
        // 标准构建环境变量
        env.insert(
            "NIX_BUILD_TOP".to_string(),
            sandbox.build_dir().to_string_lossy().into_owned(),
        );
        env.insert(
            "TMPDIR".to_string(),
            sandbox
                .build_dir()
                .join("tmp")
                .to_string_lossy()
                .into_owned(),
        );
        env.insert(
            "TEMPDIR".to_string(),
            sandbox
                .build_dir()
                .join("tmp")
                .to_string_lossy()
                .into_owned(),
        );
        env.insert(
            "TMP".to_string(),
            sandbox
                .build_dir()
                .join("tmp")
                .to_string_lossy()
                .into_owned(),
        );
        env.insert(
            "TEMP".to_string(),
            sandbox
                .build_dir()
                .join("tmp")
                .to_string_lossy()
                .into_owned(),
        );
        env.insert(
            "HOME".to_string(),
            sandbox.build_dir().to_string_lossy().into_owned(),
        );
        env.insert(
            "PWD".to_string(),
            sandbox.build_dir().to_string_lossy().into_owned(),
        );

        // Build info
        // 构建信息
        env.insert("NIX_BUILD_CORES".to_string(), self.config.cores.to_string());
        env.insert("name".to_string(), drv.name.clone());
        env.insert("version".to_string(), drv.version.clone());
        env.insert("system".to_string(), drv.system.clone());

        // Output paths
        // 输出路径
        for name in drv.outputs.keys() {
            let out_dir = sandbox.output_dir().join(name);
            let var_name = if name == "out" {
                "out".to_string()
            } else {
                name.clone()
            };
            env.insert(var_name, out_dir.to_string_lossy().into_owned());
        }

        Ok(env)
    }

    /// Set up input paths in the sandbox.
    /// 在沙箱中设置输入路径。
    fn setup_inputs(&self, drv: &Derivation, sandbox: &Sandbox) -> Result<(), BuildError> {
        let inputs_dir = sandbox.build_dir().join("inputs");
        fs::create_dir_all(&inputs_dir)?;

        // Link input derivation outputs
        // 链接输入派生的输出
        for (input_drv_path, output_names) in &drv.input_drvs {
            let input_store_path = self.store.to_path(input_drv_path);

            for output_name in output_names {
                let link_name = format!("{}-{}", input_drv_path.name(), output_name);
                let link_path = inputs_dir.join(&link_name);

                // In a real implementation, we would link to the actual output path
                // 在实际实现中，我们会链接到实际的输出路径
                // For now, just link to the derivation file
                // 目前，只链接到派生文件
                if input_store_path.exists() {
                    #[cfg(unix)]
                    std::os::unix::fs::symlink(&input_store_path, &link_path)?;

                    #[cfg(not(unix))]
                    fs::copy(&input_store_path, &link_path)?;
                }
            }
        }

        // Link input sources
        // 链接输入源
        for input_src in &drv.input_srcs {
            let src_path = self.store.to_path(input_src);
            let link_path = inputs_dir.join(input_src.name());

            if src_path.exists() {
                #[cfg(unix)]
                std::os::unix::fs::symlink(&src_path, &link_path)?;

                #[cfg(not(unix))]
                {
                    if src_path.is_dir() {
                        copy_dir_recursive(&src_path, &link_path)?;
                    } else {
                        fs::copy(&src_path, &link_path)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Create output directories.
    /// 创建输出目录。
    fn create_output_dirs(
        &self,
        drv: &Derivation,
        sandbox: &Sandbox,
    ) -> Result<HashMap<String, std::path::PathBuf>, BuildError> {
        let mut output_dirs = HashMap::new();

        for name in drv.outputs.keys() {
            let out_dir = sandbox.output_dir().join(name);
            fs::create_dir_all(&out_dir)?;
            output_dirs.insert(name.clone(), out_dir);
        }

        Ok(output_dirs)
    }

    /// Collect outputs and register them in the store.
    /// 收集输出并将其注册到存储中。
    fn collect_outputs(
        &self,
        drv: &Derivation,
        output_dirs: &HashMap<String, std::path::PathBuf>,
    ) -> Result<HashMap<String, StorePath>, BuildError> {
        let mut outputs = HashMap::new();

        for (name, output) in &drv.outputs {
            let out_dir = output_dirs.get(name).ok_or_else(|| {
                BuildError::BuildFailed(format!("missing output directory: {}", name))
            })?;

            // Validate output before collecting
            // 在收集之前验证输出
            crate::output::validate_output(out_dir)?;

            // Compute hash of output
            // 计算输出的哈希
            let hash = hash_path(out_dir)?;

            // Verify hash if expected (for fixed-output derivations)
            // 如果有预期哈希则验证（用于固定输出派生）
            if let Some(ref expected_hash) = output.expected_hash
                && hash != *expected_hash
            {
                return Err(BuildError::OutputHashMismatch {
                    output: name.clone(),
                    expected: expected_hash.to_hex(),
                    actual: hash.to_hex(),
                });
            }

            // Create store path name
            // 创建存储路径名称
            let store_name = if name == "out" {
                format!("{}-{}", drv.name, drv.version)
            } else {
                format!("{}-{}-{}", drv.name, drv.version, name)
            };

            // Add output to store
            // 将输出添加到存储
            let store_path = self.store.add_dir(out_dir, &store_name)?;

            outputs.insert(name.clone(), store_path);
        }

        Ok(outputs)
    }
}

/// Generate a simple unique ID.
/// 生成简单的唯一 ID。
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}{:x}", now.as_secs(), now.subsec_nanos())
}

/// Hash a path (file or directory).
/// 哈希路径（文件或目录）。
fn hash_path(path: &Path) -> Result<Hash, BuildError> {
    use neve_derive::Hasher;

    let mut hasher = Hasher::new();
    hash_path_recursive(path, &mut hasher)?;
    Ok(hasher.finalize())
}

/// Recursively hash a path.
/// 递归哈希路径。
fn hash_path_recursive(path: &Path, hasher: &mut neve_derive::Hasher) -> Result<(), BuildError> {
    if path.is_file() {
        let content = fs::read(path)?;
        hasher.update(&content);
    } else if path.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(path)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let name = entry.file_name();
            hasher.update(name.as_encoded_bytes());
            hash_path_recursive(&entry.path(), hasher)?;
        }
    } else if path.is_symlink() {
        let target = fs::read_link(path)?;
        hasher.update(target.as_os_str().as_encoded_bytes());
    }

    Ok(())
}

/// Recursively copy a directory.
/// 递归复制目录。
#[cfg(not(unix))]
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), BuildError> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
