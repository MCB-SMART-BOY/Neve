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

/// Resource limits for builds.
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory in bytes (0 = unlimited).
    pub max_memory: u64,
    /// Maximum CPU time in seconds (0 = unlimited).
    pub max_cpu_time: u64,
    /// Maximum number of processes.
    pub max_processes: u32,
    /// Maximum number of open file descriptors.
    pub max_fds: u32,
    /// Maximum file size in bytes.
    pub max_file_size: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 0,          // Unlimited
            max_cpu_time: 0,        // Unlimited
            max_processes: 1024,    // Reasonable default
            max_fds: 1024,          // Reasonable default
            max_file_size: 0,       // Unlimited
        }
    }
}

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
    /// Additional read-write paths to mount.
    pub rw_paths: Vec<PathBuf>,
    /// Allowed network access.
    pub network: bool,
    /// Environment variables.
    pub env: HashMap<String, String>,
    /// Resource limits.
    pub limits: ResourceLimits,
    /// Allowed syscalls (empty = all allowed).
    pub allowed_syscalls: Vec<String>,
    /// Build log file path.
    pub log_file: Option<PathBuf>,
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
            rw_paths: Vec::new(),
            network: false,
            env: HashMap::new(),
            limits: ResourceLimits::default(),
            allowed_syscalls: Vec::new(),
            log_file: None,
        }
    }

    /// Add a read-only path.
    pub fn add_ro_path(&mut self, path: PathBuf) {
        self.ro_paths.push(path);
    }
    
    /// Add a read-write path.
    pub fn add_rw_path(&mut self, path: PathBuf) {
        self.rw_paths.push(path);
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
    
    /// Set resource limits.
    pub fn with_limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = limits;
        self
    }
    
    /// Set memory limit in bytes.
    pub fn with_memory_limit(mut self, bytes: u64) -> Self {
        self.limits.max_memory = bytes;
        self
    }
    
    /// Set CPU time limit in seconds.
    pub fn with_cpu_limit(mut self, seconds: u64) -> Self {
        self.limits.max_cpu_time = seconds;
        self
    }
    
    /// Set build log file.
    pub fn with_log_file(mut self, path: PathBuf) -> Self {
        self.log_file = Some(path);
        self
    }
}

/// A sandbox for isolated builds.
pub struct Sandbox {
    config: SandboxConfig,
    /// Whether the sandbox is currently active (has an ongoing build).
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
    
    /// Check if the sandbox is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }
    
    /// Enter the sandbox (mark as active before build).
    pub fn enter(&mut self) -> Result<(), BuildError> {
        if self.active {
            return Err(BuildError::Sandbox("sandbox is already active".into()));
        }
        self.active = true;
        Ok(())
    }
    
    /// Leave the sandbox (mark as inactive after build).
    pub fn leave(&mut self) {
        self.active = false;
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
                
                // Apply resource limits using rlimit
                apply_resource_limits(&self.config.limits);
                
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

/// Apply resource limits using setrlimit.
#[cfg(target_os = "linux")]
fn apply_resource_limits(limits: &ResourceLimits) {
    use nix::sys::resource::{setrlimit, Resource};
    
    // Set memory limit (address space)
    if limits.max_memory > 0 {
        let _ = setrlimit(Resource::RLIMIT_AS, limits.max_memory, limits.max_memory);
    }
    
    // Set CPU time limit
    if limits.max_cpu_time > 0 {
        let _ = setrlimit(Resource::RLIMIT_CPU, limits.max_cpu_time, limits.max_cpu_time);
    }
    
    // Set max processes
    if limits.max_processes > 0 {
        let _ = setrlimit(
            Resource::RLIMIT_NPROC, 
            limits.max_processes as u64, 
            limits.max_processes as u64
        );
    }
    
    // Set max file descriptors
    if limits.max_fds > 0 {
        let _ = setrlimit(
            Resource::RLIMIT_NOFILE, 
            limits.max_fds as u64, 
            limits.max_fds as u64
        );
    }
    
    // Set max file size
    if limits.max_file_size > 0 {
        let _ = setrlimit(Resource::RLIMIT_FSIZE, limits.max_file_size, limits.max_file_size);
    }
}

#[cfg(not(target_os = "linux"))]
fn apply_resource_limits(_limits: &ResourceLimits) {
    // Resource limits not supported on non-Linux platforms
}

/// Build phase for structured build execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildPhase {
    /// Unpacking sources.
    Unpack,
    /// Patching sources.
    Patch,
    /// Configuration (e.g., ./configure).
    Configure,
    /// Building (e.g., make).
    Build,
    /// Checking/testing.
    Check,
    /// Installation.
    Install,
    /// Post-installation fixups.
    Fixup,
    /// Distribution phase.
    Dist,
}

impl BuildPhase {
    /// Get phase name.
    pub fn name(&self) -> &'static str {
        match self {
            BuildPhase::Unpack => "unpack",
            BuildPhase::Patch => "patch",
            BuildPhase::Configure => "configure",
            BuildPhase::Build => "build",
            BuildPhase::Check => "check",
            BuildPhase::Install => "install",
            BuildPhase::Fixup => "fixup",
            BuildPhase::Dist => "dist",
        }
    }
    
    /// Get all phases in order.
    pub fn all() -> &'static [BuildPhase] {
        &[
            BuildPhase::Unpack,
            BuildPhase::Patch,
            BuildPhase::Configure,
            BuildPhase::Build,
            BuildPhase::Check,
            BuildPhase::Install,
            BuildPhase::Fixup,
            BuildPhase::Dist,
        ]
    }
}

/// Build hook that can be executed at various phases.
pub struct BuildHook {
    /// Phase to execute at.
    pub phase: BuildPhase,
    /// Whether to run before or after the phase.
    pub before: bool,
    /// The script to execute.
    pub script: String,
}

impl BuildHook {
    /// Create a pre-phase hook.
    pub fn pre(phase: BuildPhase, script: impl Into<String>) -> Self {
        Self {
            phase,
            before: true,
            script: script.into(),
        }
    }
    
    /// Create a post-phase hook.
    pub fn post(phase: BuildPhase, script: impl Into<String>) -> Self {
        Self {
            phase,
            before: false,
            script: script.into(),
        }
    }
}

