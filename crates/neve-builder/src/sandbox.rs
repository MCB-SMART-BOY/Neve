//! Sandbox implementation for isolated builds.
//!
//! On Linux, this uses namespaces for isolation:
//! - User namespace: Maps root in container to unprivileged user
//! - Mount namespace: Isolated filesystem view
//! - PID namespace: Isolated process tree
//! - Network namespace: No network access (unless explicitly enabled)
//! - IPC namespace: Isolated System V IPC
//! - UTS namespace: Isolated hostname
//!
//! On other platforms, builds run without full isolation.

use crate::BuildError;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Sandbox configuration.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Root directory for the sandbox.
    pub root: PathBuf,
    /// Store directory (read-only mount).
    pub store_dir: PathBuf,
    /// Build directory (read-write).
    pub build_dir: PathBuf,
    /// Output directory (read-write).
    pub output_dir: PathBuf,
    /// Additional read-only paths to mount.
    pub ro_paths: Vec<PathBuf>,
    /// Allowed network access.
    pub network: bool,
    /// Environment variables.
    pub env: HashMap<String, String>,
}

impl SandboxConfig {
    /// Create a new sandbox configuration.
    pub fn new(root: PathBuf) -> Self {
        Self {
            store_dir: PathBuf::from("/neve/store"),
            build_dir: root.join("build"),
            output_dir: root.join("output"),
            root,
            ro_paths: Vec::new(),
            network: false,
            env: HashMap::new(),
        }
    }

    /// Add a read-only path.
    pub fn add_ro_path(&mut self, path: PathBuf) {
        self.ro_paths.push(path);
    }

    /// Enable network access.
    pub fn with_network(mut self) -> Self {
        self.network = true;
        self
    }
    
    /// Add an environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

/// A sandbox for isolated builds.
pub struct Sandbox {
    config: SandboxConfig,
    #[allow(dead_code)]
    active: bool,
}

impl Sandbox {
    /// Create a new sandbox.
    pub fn new(config: SandboxConfig) -> Result<Self, BuildError> {
        // Create sandbox directories
        std::fs::create_dir_all(&config.root)?;
        std::fs::create_dir_all(&config.build_dir)?;
        std::fs::create_dir_all(&config.output_dir)?;
        
        // Create tmp directory inside build dir
        std::fs::create_dir_all(config.build_dir.join("tmp"))?;
        
        Ok(Self {
            config,
            active: false,
        })
    }

    /// Get the sandbox configuration.
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Get the build directory.
    pub fn build_dir(&self) -> &Path {
        &self.config.build_dir
    }

    /// Get the output directory.
    pub fn output_dir(&self) -> &Path {
        &self.config.output_dir
    }

    /// Execute a command in the sandbox.
    #[cfg(target_os = "linux")]
    pub fn execute(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<std::process::Output, BuildError> {
        // Check if we can use namespace isolation
        if namespace_available() {
            self.execute_with_namespaces(program, args, env)
        } else {
            self.execute_simple(program, args, env)
        }
    }

    /// Execute with full namespace isolation (Linux).
    #[cfg(target_os = "linux")]
    fn execute_with_namespaces(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<std::process::Output, BuildError> {
        use nix::sched::{CloneFlags, unshare};
        use nix::unistd::{fork, ForkResult, chroot, chdir, sethostname};
        use nix::mount::{mount, MsFlags, umount2, MntFlags};
        use nix::sys::wait::waitpid;
        use std::os::unix::process::ExitStatusExt;
        
        // Create a new root for the sandbox
        let newroot = self.config.root.join("rootfs");
        std::fs::create_dir_all(&newroot)?;
        
        // Set up directory structure
        let dirs = ["bin", "usr/bin", "lib", "lib64", "etc", "tmp", "proc", "dev", "build", "output", "neve/store"];
        for dir in dirs {
            std::fs::create_dir_all(newroot.join(dir))?;
        }
        
        // Clone flags for namespace isolation
        let mut clone_flags = CloneFlags::CLONE_NEWUSER
            | CloneFlags::CLONE_NEWNS
            | CloneFlags::CLONE_NEWPID
            | CloneFlags::CLONE_NEWIPC
            | CloneFlags::CLONE_NEWUTS;
        
        if !self.config.network {
            clone_flags |= CloneFlags::CLONE_NEWNET;
        }
        
        // Fork a child process
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                // Wait for the child
                let status = waitpid(child, None)
                    .map_err(|e| BuildError::Sandbox(format!("waitpid failed: {}", e)))?;
                
                // Clean up
                let _ = std::fs::remove_dir_all(&newroot);
                
                use nix::sys::wait::WaitStatus;
                match status {
                    WaitStatus::Exited(_, code) => {
                        Ok(std::process::Output {
                            status: std::process::ExitStatus::from_raw(code),
                            stdout: Vec::new(),
                            stderr: Vec::new(),
                        })
                    }
                    _ => Err(BuildError::Sandbox("child process did not exit normally".into())),
                }
            }
            Ok(ForkResult::Child) => {
                // Enter new namespaces
                if let Err(e) = unshare(clone_flags) {
                    eprintln!("Failed to unshare: {}", e);
                    std::process::exit(1);
                }
                
                // Write UID/GID mappings
                let uid = nix::unistd::getuid();
                let gid = nix::unistd::getgid();
                
                // Map current user to root in the namespace
                let _ = std::fs::write("/proc/self/uid_map", format!("0 {} 1\n", uid));
                let _ = std::fs::write("/proc/self/setgroups", "deny\n");
                let _ = std::fs::write("/proc/self/gid_map", format!("0 {} 1\n", gid));
                
                // Set up mounts - bind mount essential directories
                let mount_opts: Option<&str> = None;
                
                // Make all mounts private
                if let Err(e) = mount::<str, str, str, str>(
                    None, "/", None, MsFlags::MS_PRIVATE | MsFlags::MS_REC, None
                ) {
                    eprintln!("Failed to make mounts private: {}", e);
                    std::process::exit(1);
                }
                
                // Bind mount the new root
                if let Err(e) = mount(
                    Some(&newroot), &newroot, mount_opts,
                    MsFlags::MS_BIND | MsFlags::MS_REC, mount_opts
                ) {
                    eprintln!("Failed to bind mount newroot: {}", e);
                    std::process::exit(1);
                }
                
                // Bind mount necessary paths
                let bind_mounts = [
                    ("/bin", "bin"),
                    ("/usr", "usr"),
                    ("/lib", "lib"),
                ];
                
                for (src, dst) in bind_mounts {
                    let src_path = Path::new(src);
                    let dst_path = newroot.join(dst);
                    if src_path.exists() {
                        let _ = mount(
                            Some(src_path), &dst_path, mount_opts,
                            MsFlags::MS_BIND | MsFlags::MS_RDONLY | MsFlags::MS_REC, mount_opts
                        );
                    }
                }
                
                // Mount /lib64 if it exists
                if Path::new("/lib64").exists() {
                    let _ = mount(
                        Some(Path::new("/lib64")), &newroot.join("lib64"), mount_opts,
                        MsFlags::MS_BIND | MsFlags::MS_RDONLY, mount_opts
                    );
                }
                
                // Bind mount the store as read-only
                if self.config.store_dir.exists() {
                    let _ = mount(
                        Some(&self.config.store_dir), &newroot.join("neve/store"), mount_opts,
                        MsFlags::MS_BIND | MsFlags::MS_RDONLY, mount_opts
                    );
                }
                
                // Bind mount build and output directories
                if let Err(e) = mount(
                    Some(&self.config.build_dir), &newroot.join("build"), mount_opts,
                    MsFlags::MS_BIND, mount_opts
                ) {
                    eprintln!("Failed to mount build dir: {}", e);
                }
                
                if let Err(e) = mount(
                    Some(&self.config.output_dir), &newroot.join("output"), mount_opts,
                    MsFlags::MS_BIND, mount_opts
                ) {
                    eprintln!("Failed to mount output dir: {}", e);
                }
                
                // Pivot root
                let old_root = newroot.join("old_root");
                std::fs::create_dir_all(&old_root).ok();
                
                if nix::unistd::pivot_root(&newroot, &old_root).is_err() {
                    // Fall back to chroot if pivot_root fails
                    if let Err(e) = chroot(&newroot) {
                        eprintln!("Failed to chroot: {}", e);
                        std::process::exit(1);
                    }
                } else {
                    // Unmount old root
                    if let Err(e) = umount2("/old_root", MntFlags::MNT_DETACH) {
                        eprintln!("Failed to unmount old root: {}", e);
                    }
                    std::fs::remove_dir("/old_root").ok();
                }
                
                // Change to build directory
                if let Err(e) = chdir("/build") {
                    eprintln!("Failed to chdir: {}", e);
                    std::process::exit(1);
                }
                
                // Mount proc
                let _ = mount::<str, str, str, str>(
                    Some("proc"), "/proc", Some("proc"),
                    MsFlags::empty(), None
                );
                
                // Set hostname
                let _ = sethostname("neve-build");
                
                // Set up environment and exec
                let mut cmd = std::process::Command::new(program);
                cmd.args(args);
                cmd.env_clear();
                
                // Default environment
                cmd.env("PATH", "/bin:/usr/bin");
                cmd.env("HOME", "/build");
                cmd.env("TMPDIR", "/tmp");
                cmd.env("NIX_BUILD_TOP", "/build");
                cmd.env("out", "/output");
                
                // User-specified environment
                for (key, value) in env {
                    cmd.env(key, value);
                }
                for (key, value) in &self.config.env {
                    cmd.env(key, value);
                }
                
                // Execute
                let status = cmd.status();
                match status {
                    Ok(s) => std::process::exit(s.code().unwrap_or(1)),
                    Err(e) => {
                        eprintln!("Failed to execute {}: {}", program, e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                Err(BuildError::Sandbox(format!("fork failed: {}", e)))
            }
        }
    }

    /// Execute without namespace isolation (fallback).
    #[cfg(target_os = "linux")]
    fn execute_simple(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<std::process::Output, BuildError> {
        use std::process::Command;
        
        let mut cmd = Command::new(program);
        cmd.args(args)
            .current_dir(&self.config.build_dir)
            .env_clear();
        
        // Default environment
        cmd.env("HOME", &self.config.build_dir);
        cmd.env("TMPDIR", self.config.build_dir.join("tmp"));
        cmd.env("PATH", "/bin:/usr/bin");
        cmd.env("NIX_BUILD_TOP", &self.config.build_dir);
        cmd.env("out", &self.config.output_dir);
        
        // User environment
        for (key, value) in env {
            cmd.env(key, value);
        }
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }
        
        let output = cmd.output()?;
        Ok(output)
    }

    /// Execute a command in the sandbox (non-Linux).
    #[cfg(not(target_os = "linux"))]
    pub fn execute(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<std::process::Output, BuildError> {
        use std::process::Command;
        
        let mut cmd = Command::new(program);
        cmd.args(args)
            .current_dir(&self.config.build_dir)
            .env_clear();
        
        // Default environment
        cmd.env("HOME", &self.config.build_dir);
        cmd.env("TMPDIR", self.config.build_dir.join("tmp"));
        cmd.env("out", &self.config.output_dir);
        
        // User environment
        for (key, value) in env {
            cmd.env(key, value);
        }
        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }
        
        let output = cmd.output()?;
        Ok(output)
    }

    /// Clean up the sandbox.
    pub fn cleanup(self) -> Result<(), BuildError> {
        // Remove sandbox directories
        if self.config.root.exists() {
            std::fs::remove_dir_all(&self.config.root)?;
        }
        Ok(())
    }
}

/// Check if sandboxing with namespaces is available on this system.
pub fn sandbox_available() -> bool {
    namespace_available()
}

/// Check if Linux namespaces are available.
#[cfg(target_os = "linux")]
fn namespace_available() -> bool {
    // Check if unprivileged user namespaces are enabled
    std::fs::read_to_string("/proc/sys/kernel/unprivileged_userns_clone")
        .map(|s| s.trim() == "1")
        .unwrap_or_else(|_| {
            // On some systems, the file doesn't exist but user namespaces work
            // Try to detect by checking if we can read user_namespaces max
            std::fs::read_to_string("/proc/sys/user/max_user_namespaces")
                .map(|s| s.trim().parse::<u32>().unwrap_or(0) > 0)
                .unwrap_or(false)
        })
}

#[cfg(not(target_os = "linux"))]
fn namespace_available() -> bool {
    false
}

/// Sandbox isolation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// No isolation, run directly.
    None,
    /// Basic isolation (chroot, env clearing).
    Basic,
    /// Full isolation with namespaces.
    Full,
}

impl IsolationLevel {
    /// Get the best available isolation level.
    pub fn best_available() -> Self {
        if namespace_available() {
            IsolationLevel::Full
        } else {
            IsolationLevel::Basic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_sandbox_config() {
        let root = env::temp_dir().join("neve-sandbox-test");
        let config = SandboxConfig::new(root.clone());
        
        assert_eq!(config.build_dir, root.join("build"));
        assert_eq!(config.output_dir, root.join("output"));
        assert!(!config.network);
    }

    #[test]
    fn test_sandbox_create() {
        let root = env::temp_dir().join(format!("neve-sandbox-test-{}", std::process::id()));
        let config = SandboxConfig::new(root.clone());
        
        let sandbox = Sandbox::new(config).unwrap();
        assert!(sandbox.build_dir().exists());
        assert!(sandbox.output_dir().exists());
        
        sandbox.cleanup().unwrap();
        assert!(!root.exists());
    }
    
    #[test]
    fn test_isolation_level() {
        let level = IsolationLevel::best_available();
        // Should be at least Basic
        assert!(level == IsolationLevel::Full || level == IsolationLevel::Basic);
    }
}
