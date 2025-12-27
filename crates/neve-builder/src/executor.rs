//! Build executor.

use crate::sandbox::{Sandbox, SandboxConfig};
use crate::{BuildError, BuilderConfig};
use neve_derive::{Derivation, Hash, StorePath};
use neve_store::Store;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Build executor.
pub struct BuildExecutor<'a> {
    store: &'a Store,
    config: &'a BuilderConfig,
}

impl<'a> BuildExecutor<'a> {
    /// Create a new build executor.
    pub fn new(store: &'a Store, config: &'a BuilderConfig) -> Self {
        Self { store, config }
    }

    /// Execute a derivation build.
    pub fn execute(&self, drv: &Derivation) -> Result<(HashMap<String, StorePath>, String), BuildError> {
        // Create temporary build directory
        let build_id = format!("{}-{}", drv.name, uuid_simple());
        let build_root = self.config.temp_dir.join(&build_id);
        fs::create_dir_all(&build_root)?;

        // Set up sandbox
        let sandbox_config = SandboxConfig::new(build_root.clone());
        let sandbox = Sandbox::new(sandbox_config)?;

        // Create tmp directory inside build
        fs::create_dir_all(sandbox.build_dir().join("tmp"))?;

        // Prepare environment
        let env = self.prepare_env(drv, &sandbox)?;

        // Set up input symlinks
        self.setup_inputs(drv, &sandbox)?;

        // Create output directories
        let output_dirs = self.create_output_dirs(drv, &sandbox)?;

        // Execute the builder
        let output = sandbox.execute(&drv.builder, &drv.args, &env)?;

        let log = format!(
            "=== stdout ===\n{}\n=== stderr ===\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        if !output.status.success() {
            if self.config.keep_failed {
                eprintln!("Build failed. Keeping build directory: {}", build_root.display());
            } else {
                let _ = sandbox.cleanup();
            }
            return Err(BuildError::BuildFailed(format!(
                "builder exited with status {}\n{}",
                output.status,
                log
            )));
        }

        // Collect outputs
        let outputs = self.collect_outputs(drv, &output_dirs)?;

        // Clean up
        if !self.config.keep_failed {
            let _ = sandbox.cleanup();
        }

        Ok((outputs, log))
    }

    /// Prepare environment variables for the build.
    fn prepare_env(&self, drv: &Derivation, sandbox: &Sandbox) -> Result<HashMap<String, String>, BuildError> {
        // Convert BTreeMap to HashMap
        let mut env: HashMap<String, String> = drv.env.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Standard build environment variables
        env.insert("NIX_BUILD_TOP".to_string(), sandbox.build_dir().to_string_lossy().into_owned());
        env.insert("TMPDIR".to_string(), sandbox.build_dir().join("tmp").to_string_lossy().into_owned());
        env.insert("TEMPDIR".to_string(), sandbox.build_dir().join("tmp").to_string_lossy().into_owned());
        env.insert("TMP".to_string(), sandbox.build_dir().join("tmp").to_string_lossy().into_owned());
        env.insert("TEMP".to_string(), sandbox.build_dir().join("tmp").to_string_lossy().into_owned());
        env.insert("HOME".to_string(), sandbox.build_dir().to_string_lossy().into_owned());
        env.insert("PWD".to_string(), sandbox.build_dir().to_string_lossy().into_owned());

        // Build info
        env.insert("NIX_BUILD_CORES".to_string(), self.config.cores.to_string());
        env.insert("name".to_string(), drv.name.clone());
        env.insert("version".to_string(), drv.version.clone());
        env.insert("system".to_string(), drv.system.clone());

        // Output paths
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
    fn setup_inputs(&self, drv: &Derivation, sandbox: &Sandbox) -> Result<(), BuildError> {
        let inputs_dir = sandbox.build_dir().join("inputs");
        fs::create_dir_all(&inputs_dir)?;

        // Link input derivation outputs
        for (input_drv_path, output_names) in &drv.input_drvs {
            let input_store_path = self.store.to_path(input_drv_path);
            
            for output_name in output_names {
                let link_name = format!("{}-{}", input_drv_path.name(), output_name);
                let link_path = inputs_dir.join(&link_name);
                
                // In a real implementation, we would link to the actual output path
                // For now, just link to the derivation file
                if input_store_path.exists() {
                    #[cfg(unix)]
                    std::os::unix::fs::symlink(&input_store_path, &link_path)?;
                    
                    #[cfg(not(unix))]
                    fs::copy(&input_store_path, &link_path)?;
                }
            }
        }

        // Link input sources
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
    fn create_output_dirs(&self, drv: &Derivation, sandbox: &Sandbox) -> Result<HashMap<String, std::path::PathBuf>, BuildError> {
        let mut output_dirs = HashMap::new();

        for name in drv.outputs.keys() {
            let out_dir = sandbox.output_dir().join(name);
            fs::create_dir_all(&out_dir)?;
            output_dirs.insert(name.clone(), out_dir);
        }

        Ok(output_dirs)
    }

    /// Collect outputs and register them in the store.
    fn collect_outputs(
        &self,
        drv: &Derivation,
        output_dirs: &HashMap<String, std::path::PathBuf>,
    ) -> Result<HashMap<String, StorePath>, BuildError> {
        let mut outputs = HashMap::new();

        for (name, output) in &drv.outputs {
            let out_dir = output_dirs.get(name)
                .ok_or_else(|| BuildError::BuildFailed(format!("missing output directory: {}", name)))?;

            // Validate output before collecting
            crate::output::validate_output(out_dir)?;

            // Compute hash of output
            let hash = hash_path(out_dir)?;
            
            // Verify hash if expected (for fixed-output derivations)
            if let Some(ref expected_hash) = output.expected_hash
                && hash != *expected_hash {
                    return Err(BuildError::OutputHashMismatch {
                        output: name.clone(),
                        expected: expected_hash.to_hex(),
                        actual: hash.to_hex(),
                    });
                }

            // Create store path name
            let store_name = if name == "out" {
                format!("{}-{}", drv.name, drv.version)
            } else {
                format!("{}-{}-{}", drv.name, drv.version, name)
            };

            // Add output to store
            let store_path = self.store.add_dir(out_dir, &store_name)?;

            outputs.insert(name.clone(), store_path);
        }

        Ok(outputs)
    }
}

/// Generate a simple unique ID.
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}{:x}", now.as_secs(), now.subsec_nanos())
}

/// Hash a path (file or directory).
fn hash_path(path: &Path) -> Result<Hash, BuildError> {
    use neve_derive::Hasher;
    
    let mut hasher = Hasher::new();
    hash_path_recursive(path, &mut hasher)?;
    Ok(hasher.finalize())
}

/// Recursively hash a path.
fn hash_path_recursive(path: &Path, hasher: &mut neve_derive::Hasher) -> Result<(), BuildError> {
    if path.is_file() {
        let content = fs::read(path)?;
        hasher.update(&content);
    } else if path.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .collect();
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

