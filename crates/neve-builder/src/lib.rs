//! Build execution for Neve.
//!
//! This crate provides functionality for building derivations:
//! - Sandboxed build environments
//! - Build execution
//! - Output collection and registration

pub mod sandbox;
pub mod executor;
pub mod output;

use neve_derive::{Derivation, StorePath};
use neve_store::Store;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during building.
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
#[derive(Debug, Clone)]
pub struct BuildResult {
    /// The derivation that was built.
    pub derivation: StorePath,
    /// Map from output name to store path.
    pub outputs: HashMap<String, StorePath>,
    /// Build log.
    pub log: String,
    /// Build duration in seconds.
    pub duration_secs: f64,
}

/// Builder configuration.
#[derive(Debug, Clone)]
pub struct BuilderConfig {
    /// Number of parallel builds.
    pub max_jobs: usize,
    /// Number of cores per build.
    pub cores: usize,
    /// Temporary directory for builds.
    pub temp_dir: PathBuf,
    /// Whether to use sandboxing.
    pub sandbox: bool,
    /// Keep failed build directories for debugging.
    pub keep_failed: bool,
    /// Build timeout in seconds (0 = no timeout).
    pub timeout: u64,
}

impl Default for BuilderConfig {
    fn default() -> Self {
        Self {
            max_jobs: 1,
            cores: num_cpus(),
            temp_dir: std::env::temp_dir().join("neve-build"),
            sandbox: cfg!(target_os = "linux"),
            keep_failed: false,
            timeout: 0,
        }
    }
}

/// Get number of CPUs.
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1)
}

/// The builder.
pub struct Builder {
    /// The store.
    store: Store,
    /// Builder configuration.
    config: BuilderConfig,
}

impl Builder {
    /// Create a new builder.
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
    fn execute_build(&mut self, drv: &Derivation) -> Result<(HashMap<String, StorePath>, String), BuildError> {
        use executor::BuildExecutor;
        
        let executor = BuildExecutor::new(&self.store, &self.config);
        executor.execute(drv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_config_default() {
        let config = BuilderConfig::default();
        assert!(config.cores >= 1);
        assert_eq!(config.max_jobs, 1);
    }
}
