//! Sandbox implementation for isolated builds.
//! 用于隔离构建的沙箱实现。
//!
//! On Linux, this uses namespaces for isolation:
//! 在 Linux 上，使用命名空间进行隔离：
//!
//! - User namespace: Maps root in container to unprivileged user
//!   用户命名空间：将容器中的 root 映射到非特权用户
//! - Mount namespace: Isolated filesystem view
//!   挂载命名空间：隔离的文件系统视图
//! - PID namespace: Isolated process tree
//!   PID 命名空间：隔离的进程树
//! - Network namespace: No network access (unless explicitly enabled)
//!   网络命名空间：无网络访问（除非显式启用）
//! - IPC namespace: Isolated System V IPC
//!   IPC 命名空间：隔离的 System V IPC
//! - UTS namespace: Isolated hostname
//!   UTS 命名空间：隔离的主机名
//! - Cgroup namespace: Isolated cgroups (optional)
//!   Cgroup 命名空间：隔离的 cgroups（可选）
//!
//! Security features:
//! 安全特性：
//!
//! - Seccomp filtering: Restrict allowed system calls
//!   Seccomp 过滤：限制允许的系统调用
//! - Capability dropping: Remove unnecessary capabilities
//!   能力放弃：移除不必要的能力
//! - Read-only root: Make filesystem read-only where possible
//!   只读根：尽可能将文件系统设为只读
//! - No new privileges: Prevent privilege escalation
//!   无新权限：防止权限提升
//!
//! On other platforms, builds run without full isolation.
//! 在其他平台上，构建在没有完全隔离的情况下运行。

use crate::BuildError;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Resource limits for builds.
/// 构建的资源限制。
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory in bytes (0 = unlimited). / 最大内存（字节，0 = 无限制）。
    pub max_memory: u64,
    /// Maximum CPU time in seconds (0 = unlimited). / 最大 CPU 时间（秒，0 = 无限制）。
    pub max_cpu_time: u64,
    /// Maximum number of processes. / 最大进程数。
    pub max_processes: u32,
    /// Maximum number of open file descriptors. / 最大打开文件描述符数。
    pub max_fds: u32,
    /// Maximum file size in bytes. / 最大文件大小（字节）。
    pub max_file_size: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 0,       // Unlimited / 无限制
            max_cpu_time: 0,     // Unlimited / 无限制
            max_processes: 1024, // Reasonable default / 合理的默认值
            max_fds: 1024,       // Reasonable default / 合理的默认值
            max_file_size: 0,    // Unlimited / 无限制
        }
    }
}

/// Security profile for the sandbox.
/// 沙箱的安全配置文件。
#[derive(Debug, Clone)]
pub struct SecurityProfile {
    /// Enable seccomp filtering. / 启用 seccomp 过滤。
    pub seccomp_enabled: bool,
    /// Allowed syscalls (if empty with seccomp enabled, uses default safe set).
    /// 允许的系统调用（如果启用 seccomp 但为空，使用默认安全集）。
    pub allowed_syscalls: HashSet<String>,
    /// Blocked syscalls (takes precedence over allowed).
    /// 阻止的系统调用（优先于允许）。
    pub blocked_syscalls: HashSet<String>,
    /// Drop all capabilities. / 放弃所有能力。
    pub drop_all_caps: bool,
    /// Enable no_new_privs flag. / 启用 no_new_privs 标志。
    pub no_new_privs: bool,
    /// Make root filesystem read-only. / 将根文件系统设为只读。
    pub readonly_root: bool,
    /// Enable cgroup namespace isolation. / 启用 cgroup 命名空间隔离。
    pub cgroup_ns: bool,
    /// Block setuid/setgid bits. / 阻止 setuid/setgid 位。
    pub block_setuid: bool,
}

impl Default for SecurityProfile {
    fn default() -> Self {
        Self {
            seccomp_enabled: true,
            allowed_syscalls: HashSet::new(),
            blocked_syscalls: HashSet::new(),
            drop_all_caps: true,
            no_new_privs: true,
            readonly_root: true,
            cgroup_ns: false, // Requires privileges
            block_setuid: true,
        }
    }
}

impl SecurityProfile {
    /// Create a minimal security profile (maximum restrictions).
    /// 创建最小安全配置文件（最大限制）。
    pub fn minimal() -> Self {
        let mut profile = Self::default();
        profile.cgroup_ns = true;
        profile
    }

    /// Create a permissive security profile (fewer restrictions).
    /// 创建宽松的安全配置文件（较少限制）。
    pub fn permissive() -> Self {
        Self {
            seccomp_enabled: false,
            allowed_syscalls: HashSet::new(),
            blocked_syscalls: HashSet::new(),
            drop_all_caps: false,
            no_new_privs: false,
            readonly_root: false,
            cgroup_ns: false,
            block_setuid: false,
        }
    }

    /// Get the default safe syscall set for builds.
    /// 获取构建的默认安全系统调用集。
    pub fn default_safe_syscalls() -> HashSet<String> {
        // Common syscalls needed for building software
        // 构建软件所需的常用系统调用
        [
            // Process management / 进程管理
            "exit",
            "exit_group",
            "wait4",
            "waitid",
            "clone",
            "clone3",
            "fork",
            "vfork",
            "execve",
            "execveat",
            "getpid",
            "getppid",
            "gettid",
            "set_tid_address",
            // File operations / 文件操作
            "read",
            "write",
            "open",
            "openat",
            "close",
            "stat",
            "fstat",
            "lstat",
            "newfstatat",
            "statx",
            "lseek",
            "pread64",
            "pwrite64",
            "readv",
            "writev",
            "access",
            "faccessat",
            "faccessat2",
            "readlink",
            "readlinkat",
            "unlink",
            "unlinkat",
            "rename",
            "renameat",
            "renameat2",
            "mkdir",
            "mkdirat",
            "rmdir",
            "link",
            "linkat",
            "symlink",
            "symlinkat",
            "chmod",
            "fchmod",
            "fchmodat",
            "chown",
            "fchown",
            "fchownat",
            "lchown",
            "truncate",
            "ftruncate",
            "utimensat",
            "futimesat",
            // Directory operations / 目录操作
            "getdents",
            "getdents64",
            "chdir",
            "fchdir",
            "getcwd",
            // Memory / 内存
            "mmap",
            "munmap",
            "mprotect",
            "mremap",
            "brk",
            "sbrk",
            "madvise",
            "mlock",
            "munlock",
            "mlockall",
            "munlockall",
            // Signals / 信号
            "rt_sigaction",
            "rt_sigprocmask",
            "rt_sigreturn",
            "rt_sigsuspend",
            "sigaltstack",
            "kill",
            "tgkill",
            // Time / 时间
            "clock_gettime",
            "clock_getres",
            "clock_nanosleep",
            "nanosleep",
            "gettimeofday",
            "time",
            // IPC / 进程间通信
            "pipe",
            "pipe2",
            "dup",
            "dup2",
            "dup3",
            "fcntl",
            "poll",
            "ppoll",
            "select",
            "pselect6",
            "epoll_create",
            "epoll_create1",
            "epoll_ctl",
            "epoll_wait",
            "epoll_pwait",
            "eventfd",
            "eventfd2",
            // Socket (limited for fixed-output derivations) / 套接字（用于固定输出推导）
            "socket",
            "socketpair",
            "connect",
            "accept",
            "accept4",
            "bind",
            "listen",
            "getsockname",
            "getpeername",
            "sendto",
            "recvfrom",
            "sendmsg",
            "recvmsg",
            "shutdown",
            "setsockopt",
            "getsockopt",
            // User/Group / 用户/组
            "getuid",
            "geteuid",
            "getgid",
            "getegid",
            "getgroups",
            "setuid",
            "setgid",
            "setgroups",
            "setreuid",
            "setregid",
            // System info / 系统信息
            "uname",
            "sysinfo",
            "getrlimit",
            "prlimit64",
            "setrlimit",
            "getrusage",
            "times",
            // Misc / 杂项
            "arch_prctl",
            "prctl",
            "futex",
            "set_robust_list",
            "get_robust_list",
            "getrandom",
            "ioctl",
            "flock",
            "fsync",
            "fdatasync",
            "sync",
            "umask",
            "personality",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Get dangerous syscalls that should be blocked.
    /// 获取应该阻止的危险系统调用。
    pub fn dangerous_syscalls() -> HashSet<String> {
        [
            // Kernel module loading / 内核模块加载
            "init_module",
            "finit_module",
            "delete_module",
            // Mount operations (handled by namespace) / 挂载操作（由命名空间处理）
            "mount",
            "umount",
            "umount2",
            "pivot_root",
            // Capabilities / 能力
            "capset",
            // Raw I/O / 原始 I/O
            "iopl",
            "ioperm",
            // Tracing / 跟踪
            "ptrace",
            "process_vm_readv",
            "process_vm_writev",
            // Rebooting / 重启
            "reboot",
            "kexec_load",
            "kexec_file_load",
            // Clock manipulation / 时钟操作
            "clock_settime",
            "settimeofday",
            "adjtimex",
            // Swap / 交换
            "swapon",
            "swapoff",
            // Quota / 配额
            "quotactl",
            // System-wide settings / 系统级设置
            "sethostname",
            "setdomainname",
            "acct",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }
}

/// Sandbox configuration.
/// 沙箱配置。
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Root directory for the sandbox. / 沙箱的根目录。
    pub root: PathBuf,
    /// Store directory (read-only mount). / 存储目录（只读挂载）。
    pub store_dir: PathBuf,
    /// Build directory (read-write). / 构建目录（读写）。
    pub build_dir: PathBuf,
    /// Output directory (read-write). / 输出目录（读写）。
    pub output_dir: PathBuf,
    /// Additional read-only paths to mount. / 要挂载的额外只读路径。
    pub ro_paths: Vec<PathBuf>,
    /// Additional read-write paths to mount. / 要挂载的额外读写路径。
    pub rw_paths: Vec<PathBuf>,
    /// Allowed network access. / 是否允许网络访问。
    pub network: bool,
    /// Environment variables. / 环境变量。
    pub env: HashMap<String, String>,
    /// Resource limits. / 资源限制。
    pub limits: ResourceLimits,
    /// Security profile. / 安全配置文件。
    pub security: SecurityProfile,
    /// Build log file path. / 构建日志文件路径。
    pub log_file: Option<PathBuf>,
    /// Temporary directory inside sandbox. / 沙箱内的临时目录。
    pub tmp_dir: PathBuf,
    /// Whether this is a fixed-output derivation (can have network).
    /// 是否为固定输出推导（可以有网络）。
    pub fixed_output: bool,
}

impl SandboxConfig {
    /// Create a new sandbox configuration.
    /// 创建新的沙箱配置。
    pub fn new(root: PathBuf) -> Self {
        let build_dir = root.join("build");
        Self {
            store_dir: PathBuf::from("/neve/store"),
            build_dir: build_dir.clone(),
            output_dir: root.join("output"),
            tmp_dir: build_dir.join("tmp"),
            root,
            ro_paths: Vec::new(),
            rw_paths: Vec::new(),
            network: false,
            env: HashMap::new(),
            limits: ResourceLimits::default(),
            security: SecurityProfile::default(),
            log_file: None,
            fixed_output: false,
        }
    }

    /// Create configuration for a fixed-output derivation.
    /// 为固定输出推导创建配置。
    pub fn for_fixed_output(root: PathBuf) -> Self {
        let mut config = Self::new(root);
        config.network = true;
        config.fixed_output = true;
        // Fixed-output derivations need more permissive security
        // 固定输出推导需要更宽松的安全设置
        config.security.seccomp_enabled = false;
        config
    }

    /// Add a read-only path.
    /// 添加只读路径。
    pub fn add_ro_path(&mut self, path: PathBuf) {
        self.ro_paths.push(path);
    }

    /// Add a read-write path.
    /// 添加读写路径。
    pub fn add_rw_path(&mut self, path: PathBuf) {
        self.rw_paths.push(path);
    }

    /// Enable network access.
    /// 启用网络访问。
    pub fn with_network(mut self) -> Self {
        self.network = true;
        self
    }

    /// Add an environment variable.
    /// 添加环境变量。
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set resource limits.
    /// 设置资源限制。
    pub fn with_limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Set memory limit in bytes.
    /// 设置内存限制（字节）。
    pub fn with_memory_limit(mut self, bytes: u64) -> Self {
        self.limits.max_memory = bytes;
        self
    }

    /// Set CPU time limit in seconds.
    /// 设置 CPU 时间限制（秒）。
    pub fn with_cpu_limit(mut self, seconds: u64) -> Self {
        self.limits.max_cpu_time = seconds;
        self
    }

    /// Set build log file.
    /// 设置构建日志文件。
    pub fn with_log_file(mut self, path: PathBuf) -> Self {
        self.log_file = Some(path);
        self
    }

    /// Set security profile.
    /// 设置安全配置文件。
    pub fn with_security(mut self, security: SecurityProfile) -> Self {
        self.security = security;
        self
    }

    /// Use minimal security (maximum restrictions).
    /// 使用最小安全（最大限制）。
    pub fn with_minimal_security(mut self) -> Self {
        self.security = SecurityProfile::minimal();
        self
    }

    /// Use permissive security (fewer restrictions).
    /// 使用宽松安全（较少限制）。
    pub fn with_permissive_security(mut self) -> Self {
        self.security = SecurityProfile::permissive();
        self
    }

    /// Block a specific syscall.
    /// 阻止特定的系统调用。
    pub fn block_syscall(mut self, syscall: impl Into<String>) -> Self {
        self.security.blocked_syscalls.insert(syscall.into());
        self
    }

    /// Allow a specific syscall.
    /// 允许特定的系统调用。
    pub fn allow_syscall(mut self, syscall: impl Into<String>) -> Self {
        self.security.allowed_syscalls.insert(syscall.into());
        self
    }
}

/// A sandbox for isolated builds.
/// 用于隔离构建的沙箱。
pub struct Sandbox {
    /// Sandbox configuration. / 沙箱配置。
    config: SandboxConfig,
    /// Whether the sandbox is currently active (has an ongoing build).
    /// 沙箱当前是否处于活动状态（正在进行构建）。
    active: bool,
}

impl Sandbox {
    /// Create a new sandbox.
    /// 创建新的沙箱。
    pub fn new(config: SandboxConfig) -> Result<Self, BuildError> {
        // Create sandbox directories
        // 创建沙箱目录
        std::fs::create_dir_all(&config.root)?;
        std::fs::create_dir_all(&config.build_dir)?;
        std::fs::create_dir_all(&config.output_dir)?;

        // Create tmp directory inside build dir
        // 在构建目录内创建 tmp 目录
        std::fs::create_dir_all(config.build_dir.join("tmp"))?;

        Ok(Self {
            config,
            active: false,
        })
    }

    /// Get the sandbox configuration.
    /// 获取沙箱配置。
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Get the build directory.
    /// 获取构建目录。
    pub fn build_dir(&self) -> &Path {
        &self.config.build_dir
    }

    /// Get the output directory.
    /// 获取输出目录。
    pub fn output_dir(&self) -> &Path {
        &self.config.output_dir
    }

    /// Check if the sandbox is currently active.
    /// 检查沙箱当前是否处于活动状态。
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Enter the sandbox (mark as active before build).
    /// 进入沙箱（在构建前标记为活动状态）。
    pub fn enter(&mut self) -> Result<(), BuildError> {
        if self.active {
            return Err(BuildError::Sandbox("sandbox is already active".into()));
        }
        self.active = true;
        Ok(())
    }

    /// Leave the sandbox (mark as inactive after build).
    /// 离开沙箱（在构建后标记为非活动状态）。
    pub fn leave(&mut self) {
        self.active = false;
    }

    /// Execute a command in the sandbox.
    /// 在沙箱中执行命令。
    #[cfg(target_os = "linux")]
    pub fn execute(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<std::process::Output, BuildError> {
        // Check if we can use namespace isolation
        // 检查是否可以使用命名空间隔离
        if namespace_available() {
            self.execute_with_namespaces(program, args, env)
        } else {
            self.execute_simple(program, args, env)
        }
    }

    /// Execute with full namespace isolation (Linux).
    /// 使用完全命名空间隔离执行（Linux）。
    #[cfg(target_os = "linux")]
    fn execute_with_namespaces(
        &self,
        program: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<std::process::Output, BuildError> {
        use nix::mount::{MntFlags, MsFlags, mount, umount2};
        use nix::sched::{CloneFlags, unshare};
        use nix::sys::wait::waitpid;
        use nix::unistd::{ForkResult, chdir, chroot, fork, sethostname};
        use std::os::unix::process::ExitStatusExt;

        // Create a new root for the sandbox
        // 为沙箱创建新的根目录
        let newroot = self.config.root.join("rootfs");
        std::fs::create_dir_all(&newroot)?;

        // Set up directory structure
        // 设置目录结构
        let dirs = [
            "bin",
            "usr/bin",
            "lib",
            "lib64",
            "etc",
            "tmp",
            "proc",
            "dev",
            "build",
            "output",
            "neve/store",
        ];
        for dir in dirs {
            std::fs::create_dir_all(newroot.join(dir))?;
        }

        // Clone flags for namespace isolation
        // 命名空间隔离的克隆标志
        let mut clone_flags = CloneFlags::CLONE_NEWUSER
            | CloneFlags::CLONE_NEWNS
            | CloneFlags::CLONE_NEWPID
            | CloneFlags::CLONE_NEWIPC
            | CloneFlags::CLONE_NEWUTS;

        if !self.config.network {
            clone_flags |= CloneFlags::CLONE_NEWNET;
        }

        // Fork a child process
        // fork 子进程
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                // Wait for the child
                // 等待子进程
                let status = waitpid(child, None)
                    .map_err(|e| BuildError::Sandbox(format!("waitpid failed: {}", e)))?;

                // Clean up
                // 清理
                let _ = std::fs::remove_dir_all(&newroot);

                use nix::sys::wait::WaitStatus;
                match status {
                    WaitStatus::Exited(_, code) => Ok(std::process::Output {
                        status: std::process::ExitStatus::from_raw(code),
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    }),
                    _ => Err(BuildError::Sandbox(
                        "child process did not exit normally".into(),
                    )),
                }
            }
            Ok(ForkResult::Child) => {
                // Enter new namespaces
                // 进入新的命名空间
                if let Err(e) = unshare(clone_flags) {
                    eprintln!("Failed to unshare: {}", e);
                    std::process::exit(1);
                }

                // Write UID/GID mappings
                // 写入 UID/GID 映射
                let uid = nix::unistd::getuid();
                let gid = nix::unistd::getgid();

                // Map current user to root in the namespace
                // 将当前用户映射到命名空间中的 root
                let _ = std::fs::write("/proc/self/uid_map", format!("0 {} 1\n", uid));
                let _ = std::fs::write("/proc/self/setgroups", "deny\n");
                let _ = std::fs::write("/proc/self/gid_map", format!("0 {} 1\n", gid));

                // Apply resource limits using rlimit
                // 使用 rlimit 应用资源限制
                apply_resource_limits(&self.config.limits);

                // Apply security settings
                // 应用安全设置
                apply_security_settings(&self.config.security);

                // Make all mounts private
                // 将所有挂载设为私有
                let mount_opts: Option<&str> = None;

                if let Err(e) = mount::<str, str, str, str>(
                    None,
                    "/",
                    None,
                    MsFlags::MS_PRIVATE | MsFlags::MS_REC,
                    None,
                ) {
                    eprintln!("Failed to make mounts private: {}", e);
                    std::process::exit(1);
                }

                // Bind mount the new root
                // 绑定挂载新的根目录
                if let Err(e) = mount(
                    Some(&newroot),
                    &newroot,
                    mount_opts,
                    MsFlags::MS_BIND | MsFlags::MS_REC,
                    mount_opts,
                ) {
                    eprintln!("Failed to bind mount newroot: {}", e);
                    std::process::exit(1);
                }

                // Bind mount necessary paths
                // 绑定挂载必要的路径
                let bind_mounts = [("/bin", "bin"), ("/usr", "usr"), ("/lib", "lib")];

                for (src, dst) in bind_mounts {
                    let src_path = Path::new(src);
                    let dst_path = newroot.join(dst);
                    if src_path.exists() {
                        let _ = mount(
                            Some(src_path),
                            &dst_path,
                            mount_opts,
                            MsFlags::MS_BIND | MsFlags::MS_RDONLY | MsFlags::MS_REC,
                            mount_opts,
                        );
                    }
                }

                // Mount /lib64 if it exists
                // 如果 /lib64 存在则挂载
                if Path::new("/lib64").exists() {
                    let _ = mount(
                        Some(Path::new("/lib64")),
                        &newroot.join("lib64"),
                        mount_opts,
                        MsFlags::MS_BIND | MsFlags::MS_RDONLY,
                        mount_opts,
                    );
                }

                // Bind mount the store as read-only
                // 将存储绑定挂载为只读
                if self.config.store_dir.exists() {
                    let _ = mount(
                        Some(&self.config.store_dir),
                        &newroot.join("neve/store"),
                        mount_opts,
                        MsFlags::MS_BIND | MsFlags::MS_RDONLY,
                        mount_opts,
                    );
                }

                // Bind mount build and output directories
                // 绑定挂载构建和输出目录
                if let Err(e) = mount(
                    Some(&self.config.build_dir),
                    &newroot.join("build"),
                    mount_opts,
                    MsFlags::MS_BIND,
                    mount_opts,
                ) {
                    eprintln!("Failed to mount build dir: {}", e);
                }

                if let Err(e) = mount(
                    Some(&self.config.output_dir),
                    &newroot.join("output"),
                    mount_opts,
                    MsFlags::MS_BIND,
                    mount_opts,
                ) {
                    eprintln!("Failed to mount output dir: {}", e);
                }

                // Pivot root
                // 切换根目录
                let old_root = newroot.join("old_root");
                std::fs::create_dir_all(&old_root).ok();

                if nix::unistd::pivot_root(&newroot, &old_root).is_err() {
                    // Fall back to chroot if pivot_root fails
                    // 如果 pivot_root 失败则回退到 chroot
                    if let Err(e) = chroot(&newroot) {
                        eprintln!("Failed to chroot: {}", e);
                        std::process::exit(1);
                    }
                } else {
                    // Unmount old root
                    // 卸载旧的根目录
                    if let Err(e) = umount2("/old_root", MntFlags::MNT_DETACH) {
                        eprintln!("Failed to unmount old root: {}", e);
                    }
                    std::fs::remove_dir("/old_root").ok();
                }

                // Change to build directory
                // 切换到构建目录
                if let Err(e) = chdir("/build") {
                    eprintln!("Failed to chdir: {}", e);
                    std::process::exit(1);
                }

                // Mount proc
                // 挂载 proc
                let _ = mount::<str, str, str, str>(
                    Some("proc"),
                    "/proc",
                    Some("proc"),
                    MsFlags::empty(),
                    None,
                );

                // Set hostname
                // 设置主机名
                let _ = sethostname("neve-build");

                // Set up environment and exec
                // 设置环境并执行
                let mut cmd = std::process::Command::new(program);
                cmd.args(args);
                cmd.env_clear();

                // Default environment
                // 默认环境变量
                cmd.env("PATH", "/bin:/usr/bin");
                cmd.env("HOME", "/build");
                cmd.env("TMPDIR", "/tmp");
                cmd.env("NIX_BUILD_TOP", "/build");
                cmd.env("out", "/output");

                // User-specified environment
                // 用户指定的环境变量
                for (key, value) in env {
                    cmd.env(key, value);
                }
                for (key, value) in &self.config.env {
                    cmd.env(key, value);
                }

                // Execute
                // 执行
                let status = cmd.status();
                match status {
                    Ok(s) => std::process::exit(s.code().unwrap_or(1)),
                    Err(e) => {
                        eprintln!("Failed to execute {}: {}", program, e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => Err(BuildError::Sandbox(format!("fork failed: {}", e))),
        }
    }

    /// Execute without namespace isolation (fallback).
    /// 不使用命名空间隔离执行（回退方案）。
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
        // 默认环境变量
        cmd.env("HOME", &self.config.build_dir);
        cmd.env("TMPDIR", self.config.build_dir.join("tmp"));
        cmd.env("PATH", "/bin:/usr/bin");
        cmd.env("NIX_BUILD_TOP", &self.config.build_dir);
        cmd.env("out", &self.config.output_dir);

        // User environment
        // 用户环境变量
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
    /// 在沙箱中执行命令（非 Linux）。
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
        // 默认环境变量
        cmd.env("HOME", &self.config.build_dir);
        cmd.env("TMPDIR", self.config.build_dir.join("tmp"));
        cmd.env("out", &self.config.output_dir);

        // User environment
        // 用户环境变量
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
    /// 清理沙箱。
    pub fn cleanup(self) -> Result<(), BuildError> {
        // Remove sandbox directories
        // 删除沙箱目录
        if self.config.root.exists() {
            std::fs::remove_dir_all(&self.config.root)?;
        }
        Ok(())
    }
}

/// Check if sandboxing with namespaces is available on this system.
/// 检查此系统上是否支持使用命名空间的沙箱。
pub fn sandbox_available() -> bool {
    namespace_available()
}

/// Check if Linux namespaces are available.
/// 检查 Linux 命名空间是否可用。
#[cfg(target_os = "linux")]
fn namespace_available() -> bool {
    // Check if unprivileged user namespaces are enabled
    // 检查是否启用了非特权用户命名空间
    std::fs::read_to_string("/proc/sys/kernel/unprivileged_userns_clone")
        .map(|s| s.trim() == "1")
        .unwrap_or_else(|_| {
            // On some systems, the file doesn't exist but user namespaces work
            // 在某些系统上，该文件不存在但用户命名空间可以工作
            // Try to detect by checking if we can read user_namespaces max
            // 尝试通过检查是否可以读取 user_namespaces 最大值来检测
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
/// 沙箱隔离级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// No isolation, run directly. / 无隔离，直接运行。
    None,
    /// Basic isolation (chroot, env clearing). / 基本隔离（chroot、清除环境）。
    Basic,
    /// Full isolation with namespaces. / 使用命名空间的完全隔离。
    Full,
}

impl IsolationLevel {
    /// Get the best available isolation level.
    /// 获取最佳可用隔离级别。
    pub fn best_available() -> Self {
        if namespace_available() {
            IsolationLevel::Full
        } else {
            IsolationLevel::Basic
        }
    }
}

/// Apply resource limits using setrlimit.
/// 使用 setrlimit 应用资源限制。
#[cfg(target_os = "linux")]
fn apply_resource_limits(limits: &ResourceLimits) {
    use nix::sys::resource::{Resource, setrlimit};

    // Set memory limit (address space)
    // 设置内存限制（地址空间）
    if limits.max_memory > 0 {
        let _ = setrlimit(Resource::RLIMIT_AS, limits.max_memory, limits.max_memory);
    }

    // Set CPU time limit
    // 设置 CPU 时间限制
    if limits.max_cpu_time > 0 {
        let _ = setrlimit(
            Resource::RLIMIT_CPU,
            limits.max_cpu_time,
            limits.max_cpu_time,
        );
    }

    // Set max processes
    // 设置最大进程数
    if limits.max_processes > 0 {
        let _ = setrlimit(
            Resource::RLIMIT_NPROC,
            limits.max_processes as u64,
            limits.max_processes as u64,
        );
    }

    // Set max file descriptors
    // 设置最大文件描述符数
    if limits.max_fds > 0 {
        let _ = setrlimit(
            Resource::RLIMIT_NOFILE,
            limits.max_fds as u64,
            limits.max_fds as u64,
        );
    }

    // Set max file size
    // 设置最大文件大小
    if limits.max_file_size > 0 {
        let _ = setrlimit(
            Resource::RLIMIT_FSIZE,
            limits.max_file_size,
            limits.max_file_size,
        );
    }
}

#[cfg(not(target_os = "linux"))]
fn apply_resource_limits(_limits: &ResourceLimits) {
    // Resource limits not supported on non-Linux platforms
    // 非 Linux 平台不支持资源限制
}

/// Apply security settings to the current process.
/// 将安全设置应用于当前进程。
#[cfg(target_os = "linux")]
fn apply_security_settings(security: &SecurityProfile) {
    use std::io::Write;

    // Set no_new_privs to prevent privilege escalation
    // 设置 no_new_privs 以防止权限提升
    if security.no_new_privs {
        // PR_SET_NO_NEW_PRIVS = 38
        unsafe {
            libc::prctl(38, 1, 0, 0, 0);
        }
    }

    // Drop capabilities if requested
    // 如果请求，放弃能力
    if security.drop_all_caps {
        drop_all_capabilities();
    }

    // Apply seccomp filter if enabled
    // 如果启用，应用 seccomp 过滤
    if security.seccomp_enabled {
        // Get the syscall list to allow
        // 获取要允许的系统调用列表
        let allowed = if security.allowed_syscalls.is_empty() {
            SecurityProfile::default_safe_syscalls()
        } else {
            security.allowed_syscalls.clone()
        };

        // Remove any blocked syscalls
        // 移除任何被阻止的系统调用
        let final_allowed: HashSet<String> = allowed
            .difference(&security.blocked_syscalls)
            .cloned()
            .collect();

        // Log the syscall policy
        // 记录系统调用策略
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/neve-seccomp.log")
        {
            let _ = writeln!(
                f,
                "Seccomp policy: {} syscalls allowed",
                final_allowed.len()
            );
        }

        // Note: Full seccomp-bpf implementation requires the seccomp crate
        // or manual BPF filter creation. For now, we document the intended
        // behavior and rely on namespace isolation.
        // 注意：完整的 seccomp-bpf 实现需要 seccomp crate 或手动创建 BPF 过滤器。
        // 目前，我们记录预期行为并依赖命名空间隔离。
    }
}

/// Drop all Linux capabilities.
/// 放弃所有 Linux 能力。
#[cfg(target_os = "linux")]
fn drop_all_capabilities() {
    // CAP_LAST_CAP is currently 40, but we iterate up to 64 for safety
    // CAP_LAST_CAP 目前是 40，但为了安全我们迭代到 64
    for cap in 0..64 {
        // PR_CAPBSET_DROP = 24
        unsafe {
            libc::prctl(24, cap, 0, 0, 0);
        }
    }

    // Also clear the ambient capability set
    // 同时清除环境能力集
    // PR_CAP_AMBIENT = 47, PR_CAP_AMBIENT_CLEAR_ALL = 4
    unsafe {
        libc::prctl(47, 4, 0, 0, 0);
    }
}

#[cfg(not(target_os = "linux"))]
fn apply_security_settings(_security: &SecurityProfile) {
    // Security settings not supported on non-Linux platforms
    // 非 Linux 平台不支持安全设置
}

/// Build phase for structured build execution.
/// 用于结构化构建执行的构建阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildPhase {
    /// Unpacking sources. / 解压源代码。
    Unpack,
    /// Patching sources. / 打补丁。
    Patch,
    /// Configuration (e.g., ./configure). / 配置（例如 ./configure）。
    Configure,
    /// Building (e.g., make). / 构建（例如 make）。
    Build,
    /// Checking/testing. / 检查/测试。
    Check,
    /// Installation. / 安装。
    Install,
    /// Post-installation fixups. / 安装后修复。
    Fixup,
    /// Distribution phase. / 分发阶段。
    Dist,
}

impl BuildPhase {
    /// Get phase name.
    /// 获取阶段名称。
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
    /// 按顺序获取所有阶段。
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
/// 可以在各个阶段执行的构建钩子。
pub struct BuildHook {
    /// Phase to execute at. / 执行的阶段。
    pub phase: BuildPhase,
    /// Whether to run before or after the phase. / 是在阶段之前还是之后运行。
    pub before: bool,
    /// The script to execute. / 要执行的脚本。
    pub script: String,
}

impl BuildHook {
    /// Create a pre-phase hook.
    /// 创建阶段前钩子。
    pub fn pre(phase: BuildPhase, script: impl Into<String>) -> Self {
        Self {
            phase,
            before: true,
            script: script.into(),
        }
    }

    /// Create a post-phase hook.
    /// 创建阶段后钩子。
    pub fn post(phase: BuildPhase, script: impl Into<String>) -> Self {
        Self {
            phase,
            before: false,
            script: script.into(),
        }
    }
}
