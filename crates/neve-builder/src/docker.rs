//! Docker-based build backend for cross-platform sandboxed builds.
//!
//! This module provides a Docker-based build execution environment that
//! enables sandboxed, reproducible builds on platforms that don't support
//! Linux namespaces (macOS, Windows).

use crate::BuildError;
use neve_derive::Derivation;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Default Docker image for builds.
pub const DEFAULT_BUILD_IMAGE: &str = "neve-build:latest";

/// Dockerfile template for creating the build image.
pub const BUILD_DOCKERFILE: &str = r#"
FROM alpine:latest

# Install basic build tools
RUN apk add --no-cache \
    bash \
    coreutils \
    findutils \
    diffutils \
    patch \
    sed \
    grep \
    gawk \
    gzip \
    bzip2 \
    xz \
    tar \
    make \
    gcc \
    g++ \
    musl-dev \
    curl \
    wget \
    git

# Create standard directories
RUN mkdir -p /neve/store /build /output /tmp

# Set up environment
ENV PATH="/neve/store/bin:/usr/local/bin:/usr/bin:/bin"
ENV HOME="/build"
ENV TMPDIR="/tmp"

WORKDIR /build
"#;

/// Docker build configuration.
#[derive(Debug, Clone)]
pub struct DockerConfig {
    /// Docker image to use for builds.
    pub image: String,
    /// Whether to pull the image if not available.
    pub auto_pull: bool,
    /// Whether to build the image if not available.
    pub auto_build: bool,
    /// Additional volumes to mount (host_path -> container_path).
    pub extra_volumes: HashMap<String, String>,
    /// Memory limit (e.g., "2g").
    pub memory_limit: Option<String>,
    /// CPU limit (e.g., "2").
    pub cpu_limit: Option<String>,
    /// Network mode ("none", "bridge", etc.).
    pub network_mode: String,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            image: DEFAULT_BUILD_IMAGE.to_string(),
            auto_pull: true,
            auto_build: true,
            extra_volumes: HashMap::new(),
            memory_limit: None,
            cpu_limit: None,
            network_mode: "none".to_string(),
        }
    }
}

/// Docker build executor.
pub struct DockerExecutor {
    config: DockerConfig,
    store_dir: PathBuf,
    temp_dir: PathBuf,
}

impl DockerExecutor {
    /// Create a new Docker executor.
    pub fn new(store_dir: PathBuf, temp_dir: PathBuf) -> Self {
        Self {
            config: DockerConfig::default(),
            store_dir,
            temp_dir,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(store_dir: PathBuf, temp_dir: PathBuf, config: DockerConfig) -> Self {
        Self {
            config,
            store_dir,
            temp_dir,
        }
    }

    /// Check if Docker is available.
    pub fn is_available() -> bool {
        Command::new("docker")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Ensure the build image exists.
    pub fn ensure_image(&self) -> Result<(), BuildError> {
        // Check if image exists
        let output = Command::new("docker")
            .args(["image", "inspect", &self.config.image])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if output.map(|s| s.success()).unwrap_or(false) {
            return Ok(());
        }

        // Image doesn't exist, try to build it
        if self.config.auto_build {
            self.build_image()?;
        } else {
            return Err(BuildError::Sandbox(format!(
                "Docker image '{}' not found. Run 'neve docker build-image' to create it.",
                self.config.image
            )));
        }

        Ok(())
    }

    /// Build the Docker image.
    pub fn build_image(&self) -> Result<(), BuildError> {
        eprintln!("Building Docker image '{}'...", self.config.image);

        // Create a temporary directory for the Dockerfile
        let dockerfile_dir = self.temp_dir.join("docker-build");
        std::fs::create_dir_all(&dockerfile_dir)?;

        let dockerfile_path = dockerfile_dir.join("Dockerfile");
        std::fs::write(&dockerfile_path, BUILD_DOCKERFILE)?;

        let output = Command::new("docker")
            .args([
                "build",
                "-t",
                &self.config.image,
                "-f",
                dockerfile_path.to_str().unwrap(),
                dockerfile_dir.to_str().unwrap(),
            ])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()?;

        // Clean up
        let _ = std::fs::remove_dir_all(&dockerfile_dir);

        if !output.status.success() {
            return Err(BuildError::Sandbox(
                "Failed to build Docker image".to_string(),
            ));
        }

        eprintln!("Docker image '{}' built successfully.", self.config.image);
        Ok(())
    }

    /// Execute a build in Docker.
    pub fn execute(
        &self,
        drv: &Derivation,
        build_dir: &Path,
        output_dir: &Path,
    ) -> Result<std::process::Output, BuildError> {
        // Ensure image exists
        self.ensure_image()?;

        // Prepare volumes
        let mut volumes = vec![
            format!("{}:/neve/store:ro", self.store_dir.display()),
            format!("{}:/build:rw", build_dir.display()),
            format!("{}:/output:rw", output_dir.display()),
        ];

        for (host, container) in &self.config.extra_volumes {
            volumes.push(format!("{}:{}:ro", host, container));
        }

        // Build docker run arguments
        let mut args = vec!["run".to_string(), "--rm".to_string()];

        // Add volumes
        for vol in &volumes {
            args.push("-v".to_string());
            args.push(vol.clone());
        }

        // Add network mode
        args.push("--network".to_string());
        args.push(self.config.network_mode.clone());

        // Add resource limits
        if let Some(ref mem) = self.config.memory_limit {
            args.push("--memory".to_string());
            args.push(mem.clone());
        }

        if let Some(ref cpu) = self.config.cpu_limit {
            args.push("--cpus".to_string());
            args.push(cpu.clone());
        }

        // Add environment variables
        for (key, value) in &drv.env {
            args.push("-e".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Standard environment
        args.push("-e".to_string());
        args.push("HOME=/build".to_string());
        args.push("-e".to_string());
        args.push("TMPDIR=/tmp".to_string());
        args.push("-e".to_string());
        args.push("out=/output".to_string());
        args.push("-e".to_string());
        args.push(format!("NIX_BUILD_CORES={}", num_cpus::get()));

        // Add working directory
        args.push("-w".to_string());
        args.push("/build".to_string());

        // Add image name
        args.push(self.config.image.clone());

        // Add command
        args.push(drv.builder.clone());
        for arg in &drv.args {
            args.push(arg.clone());
        }

        // Execute
        let output = Command::new("docker")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        Ok(output)
    }
}

/// Get the number of CPUs (for builds).
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_config_default() {
        let config = DockerConfig::default();
        assert_eq!(config.image, DEFAULT_BUILD_IMAGE);
        assert_eq!(config.network_mode, "none");
    }

    #[test]
    fn test_docker_available() {
        // This test just ensures the function doesn't panic
        let _ = DockerExecutor::is_available();
    }
}
