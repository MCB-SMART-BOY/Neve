//! Build execution for Neve.
//! Neve 的构建执行。
//!
//! This crate provides functionality for building derivations:
//! 本 crate 提供构建派生的功能：
//!
//! - Sandboxed build environments / 沙箱构建环境
//! - Build execution / 构建执行
//! - Output collection and registration / 输出收集和注册
//! - Docker-based builds for cross-platform support / 基于 Docker 的跨平台构建支持

pub mod analytics;
pub mod docker;
pub mod executor;
pub mod output;
pub mod sandbox;

use neve_derive::{Derivation, StorePath};
use neve_store::Store;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during building.
/// 构建过程中可能发生的错误。
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("store error: {0}")]
    Store(#[from] neve_store::StoreError),

    #[error("fetch error: {0}")]
    Fetch(#[from] neve_fetch::FetchError),

    #[error("sandbox error: {0}")]
    Sandbox(String),

    #[error("build failed: {0}")]
    BuildFailed(String),

    #[error("missing input: {0}")]
    MissingInput(String),

    #[error("output hash mismatch for {output}: expected {expected}, got {actual}")]
    OutputHashMismatch {
        output: String,
        expected: String,
        actual: String,
    },
}

/// Build result.
/// 构建结果。
#[derive(Debug, Clone)]
pub struct BuildResult {
    /// The derivation that was built. / 被构建的派生。
    pub derivation: StorePath,
    /// Map from output name to store path. / 输出名称到存储路径的映射。
    pub outputs: HashMap<String, StorePath>,
    /// Build log. / 构建日志。
    pub log: String,
    /// Build duration in seconds. / 构建耗时（秒）。
    pub duration_secs: f64,
}

/// Build backend type.
/// 构建后端类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildBackend {
    /// Native sandbox (Linux namespaces) - full isolation.
    /// 原生沙箱（Linux 命名空间）- 完全隔离。
    #[default]
    Native,
    /// Docker-based sandbox - cross-platform isolation.
    /// 基于 Docker 的沙箱 - 跨平台隔离。
    Docker,
    /// Simple execution without isolation.
    /// 简单执行，无隔离。
    Simple,
}

/// Builder configuration.
/// 构建器配置。
#[derive(Debug, Clone)]
pub struct BuilderConfig {
    /// Number of parallel builds. / 并行构建数量。
    pub max_jobs: usize,
    /// Number of cores per build. / 每个构建使用的核心数。
    pub cores: usize,
    /// Temporary directory for builds. / 构建临时目录。
    pub temp_dir: PathBuf,
    /// Whether to use sandboxing. / 是否使用沙箱。
    pub sandbox: bool,
    /// Keep failed build directories for debugging. / 保留失败的构建目录以供调试。
    pub keep_failed: bool,
    /// Build timeout in seconds (0 = no timeout). / 构建超时（秒，0 表示无超时）。
    pub timeout: u64,
    /// Build backend to use. / 使用的构建后端。
    pub backend: BuildBackend,
}

impl Default for BuilderConfig {
    fn default() -> Self {
        // Determine the best backend for this platform
        let backend = if cfg!(target_os = "linux") && sandbox::sandbox_available() {
            BuildBackend::Native
        } else if docker::DockerExecutor::is_available() {
            BuildBackend::Docker
        } else {
            BuildBackend::Simple
        };

        Self {
            max_jobs: 1,
            cores: num_cpus(),
            temp_dir: std::env::temp_dir().join("neve-build"),
            sandbox: backend != BuildBackend::Simple,
            keep_failed: false,
            timeout: 0,
            backend,
        }
    }
}

/// Get number of CPUs.
/// 获取 CPU 数量。
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1)
}

/// The builder.
/// 构建器。
pub struct Builder {
    /// The store. / 存储。
    store: Store,
    /// Builder configuration. / 构建器配置。
    config: BuilderConfig,
}

impl Builder {
    /// Create a new builder.
    /// 创建新的构建器。
    pub fn new(store: Store) -> Self {
        Self {
            store,
            config: BuilderConfig::default(),
        }
    }

    /// Create a new builder with configuration.
    pub fn with_config(store: Store, config: BuilderConfig) -> Self {
        Self { store, config }
    }

    /// Get the store.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Get mutable store reference.
    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    /// Get the configuration.
    pub fn config(&self) -> &BuilderConfig {
        &self.config
    }

    /// Build a derivation.
    /// 构建一个派生。
    pub fn build(&mut self, drv: &Derivation) -> Result<BuildResult, BuildError> {
        let start = std::time::Instant::now();

        // Check if already built
        let drv_path = drv.drv_path();
        if let Some(outputs) = self.check_outputs_exist(drv) {
            return Ok(BuildResult {
                derivation: drv_path,
                outputs,
                log: String::new(),
                duration_secs: 0.0,
            });
        }

        // Ensure all inputs are available
        self.ensure_inputs(drv)?;

        // Execute the build
        let (outputs, log) = self.execute_build(drv)?;

        let duration = start.elapsed().as_secs_f64();

        Ok(BuildResult {
            derivation: drv_path,
            outputs,
            log,
            duration_secs: duration,
        })
    }

    /// Check if all outputs already exist.
    fn check_outputs_exist(&self, drv: &Derivation) -> Option<HashMap<String, StorePath>> {
        let mut outputs = HashMap::new();

        for (name, output) in &drv.outputs {
            if let Some(ref path) = output.path {
                if self.store.path_exists(path) {
                    outputs.insert(name.clone(), path.clone());
                } else {
                    return None;
                }
            } else {
                // Content-addressed output, can't check ahead of time
                return None;
            }
        }

        Some(outputs)
    }

    /// Ensure all inputs are available.
    fn ensure_inputs(&mut self, drv: &Derivation) -> Result<(), BuildError> {
        // Check input derivations
        for input_drv_path in drv.input_drvs.keys() {
            if !self.store.path_exists(input_drv_path) {
                return Err(BuildError::MissingInput(input_drv_path.display_name()));
            }

            // Read and build the input derivation if its outputs don't exist
            let input_drv = self.store.read_derivation(input_drv_path)?;
            if self.check_outputs_exist(&input_drv).is_none() {
                self.build(&input_drv)?;
            }
        }

        // Check input sources
        for input_src in &drv.input_srcs {
            if !self.store.path_exists(input_src) {
                return Err(BuildError::MissingInput(input_src.display_name()));
            }
        }

        Ok(())
    }

    /// Execute the build.
    fn execute_build(
        &mut self,
        drv: &Derivation,
    ) -> Result<(HashMap<String, StorePath>, String), BuildError> {
        use executor::BuildExecutor;

        let executor = BuildExecutor::new(&self.store, &self.config);
        executor.execute(drv)
    }
}
