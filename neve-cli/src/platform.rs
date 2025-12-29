//! Platform detection and capability reporting.
//! 平台检测和功能报告。
//!
//! This module provides utilities for detecting platform-specific features
//! and displaying appropriate warnings to users on limited platforms.
//! 本模块提供检测平台特定功能的工具，并在受限平台上向用户显示适当的警告。

use std::fmt;

/// Platform capabilities for Neve.
/// Neve 的平台功能。
#[derive(Debug, Clone)]
pub struct PlatformCapabilities {
    /// The current operating system. / 当前操作系统。
    pub os: Os,
    /// The CPU architecture. / CPU 架构。
    pub arch: Arch,
    /// Whether native sandboxed builds are available. / 是否支持原生沙箱构建。
    pub native_sandbox: bool,
    /// Whether Docker is available for builds. / Docker 是否可用于构建。
    pub docker_available: bool,
    /// Whether system configuration is supported. / 是否支持系统配置。
    pub system_config: bool,
}

/// Operating system.
/// 操作系统。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    /// Linux operating system. / Linux 操作系统。
    Linux,
    /// macOS operating system. / macOS 操作系统。
    MacOS,
    /// Windows operating system. / Windows 操作系统。
    Windows,
    /// Other/unknown operating system. / 其他/未知操作系统。
    Other,
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Os::Linux => write!(f, "Linux"),
            Os::MacOS => write!(f, "macOS"),
            Os::Windows => write!(f, "Windows"),
            Os::Other => write!(f, "Unknown"),
        }
    }
}

/// CPU architecture.
/// CPU 架构。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    /// 64-bit x86 architecture. / 64 位 x86 架构。
    X86_64,
    /// 64-bit ARM architecture. / 64 位 ARM 架构。
    Aarch64,
    /// Other/unknown architecture. / 其他/未知架构。
    Other,
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arch::X86_64 => write!(f, "x86_64"),
            Arch::Aarch64 => write!(f, "aarch64"),
            Arch::Other => write!(f, "unknown"),
        }
    }
}

impl PlatformCapabilities {
    /// Detect capabilities for the current platform.
    /// 检测当前平台的功能。
    pub fn detect() -> Self {
        let os = detect_os();
        let arch = detect_arch();
        let native_sandbox = os == Os::Linux && check_namespace_support();
        let docker_available = check_docker();
        let system_config = os == Os::Linux;

        Self {
            os,
            arch,
            native_sandbox,
            docker_available,
            system_config,
        }
    }

    /// Get the system identifier string (e.g., "x86_64-linux").
    /// 获取系统标识符字符串（例如 "x86_64-linux"）。
    pub fn system_id(&self) -> String {
        let os_str = match self.os {
            Os::Linux => "linux",
            Os::MacOS => "darwin",
            Os::Windows => "windows",
            Os::Other => "unknown",
        };
        format!("{}-{}", self.arch, os_str)
    }

    /// Check if sandboxed builds are available (native or Docker).
    /// 检查沙箱构建是否可用（原生或 Docker）。
    pub fn can_sandbox_build(&self) -> bool {
        self.native_sandbox || self.docker_available
    }

    /// Get the recommended build backend.
    /// 获取推荐的构建后端。
    pub fn recommended_backend(&self) -> BuildBackend {
        if self.native_sandbox {
            BuildBackend::Native
        } else if self.docker_available {
            BuildBackend::Docker
        } else {
            BuildBackend::Simple
        }
    }

    /// Print platform information.
    /// 打印平台信息。
    pub fn print_info(&self) {
        println!("Platform: {} {}", self.os, self.arch);
        println!("System ID: {}", self.system_id());
        println!();
        println!("Capabilities:");
        println!(
            "  Native sandbox:     {}",
            if self.native_sandbox { "yes" } else { "no" }
        );
        println!(
            "  Docker available:   {}",
            if self.docker_available { "yes" } else { "no" }
        );
        println!(
            "  System config:      {}",
            if self.system_config { "yes" } else { "no" }
        );
        println!();
        println!("Recommended build backend: {}", self.recommended_backend());
    }
}

/// Build backend options.
/// 构建后端选项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildBackend {
    /// Native Linux namespace isolation (full isolation).
    /// 原生 Linux 命名空间隔离（完全隔离）。
    Native,
    /// Docker-based isolation (cross-platform).
    /// 基于 Docker 的隔离（跨平台）。
    Docker,
    /// Simple execution without isolation.
    /// 无隔离的简单执行。
    Simple,
}

impl fmt::Display for BuildBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildBackend::Native => write!(f, "native (Linux namespaces)"),
            BuildBackend::Docker => write!(f, "docker"),
            BuildBackend::Simple => write!(f, "simple (no isolation)"),
        }
    }
}

/// Detect the current operating system.
/// 检测当前操作系统。
fn detect_os() -> Os {
    match std::env::consts::OS {
        "linux" => Os::Linux,
        "macos" => Os::MacOS,
        "windows" => Os::Windows,
        _ => Os::Other,
    }
}

/// Detect the CPU architecture.
/// 检测 CPU 架构。
fn detect_arch() -> Arch {
    match std::env::consts::ARCH {
        "x86_64" => Arch::X86_64,
        "aarch64" => Arch::Aarch64,
        _ => Arch::Other,
    }
}

/// Check if Linux namespace support is available.
/// 检查 Linux 命名空间支持是否可用。
#[cfg(target_os = "linux")]
fn check_namespace_support() -> bool {
    // Check if unprivileged user namespaces are enabled
    // 检查是否启用了非特权用户命名空间
    std::fs::read_to_string("/proc/sys/kernel/unprivileged_userns_clone")
        .map(|s| s.trim() == "1")
        .unwrap_or_else(|_| {
            // On some systems, the file doesn't exist but user namespaces work
            // 在某些系统上，该文件不存在但用户命名空间仍可工作
            std::fs::read_to_string("/proc/sys/user/max_user_namespaces")
                .map(|s| s.trim().parse::<u32>().unwrap_or(0) > 0)
                .unwrap_or(false)
        })
}

#[cfg(not(target_os = "linux"))]
fn check_namespace_support() -> bool {
    false
}

/// Check if Docker is available.
/// 检查 Docker 是否可用。
fn check_docker() -> bool {
    std::process::Command::new("docker")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Warn about limited sandbox support on non-Linux platforms.
/// 警告非 Linux 平台上有限的沙箱支持。
pub fn warn_limited_sandbox() {
    let caps = PlatformCapabilities::detect();

    if !caps.native_sandbox {
        eprintln!("\x1b[33mwarning:\x1b[0m Native sandboxed builds are only available on Linux.");

        if caps.docker_available {
            eprintln!("         Using Docker backend for isolated builds.");
            eprintln!("         Use --backend native to force native mode (less isolation).");
        } else {
            eprintln!("         Builds will run with limited isolation.");
            eprintln!(
                "         Install Docker for better isolation on {}.",
                caps.os
            );
        }
        eprintln!();
    }
}

/// Warn about system configuration not being available.
/// 警告系统配置不可用。
pub fn warn_system_config_unavailable() {
    let caps = PlatformCapabilities::detect();

    if !caps.system_config {
        eprintln!("\x1b[33mwarning:\x1b[0m System configuration is only available on Linux.");
        eprintln!("         This feature manages /etc, services, and system state.");
        eprintln!("         On {}, you can still use Neve for:", caps.os);
        eprintln!("           - Package definitions and builds (with Docker)");
        eprintln!("           - User-level environment management");
        eprintln!("           - Development environment configuration");
        eprintln!();
    }
}

/// Print a note about cross-platform usage.
/// 打印关于跨平台使用的说明。
pub fn print_cross_platform_note() {
    let caps = PlatformCapabilities::detect();

    if caps.os != Os::Linux {
        println!();
        println!("\x1b[34mNote:\x1b[0m Running on {} {}.", caps.os, caps.arch);
        println!("      Language features (eval, check, fmt, repl, lsp) work fully.");

        if caps.docker_available {
            println!("      Package builds will use Docker for isolation.");
        } else {
            println!("      Install Docker for sandboxed package builds.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_capabilities() {
        let caps = PlatformCapabilities::detect();
        // Just ensure detection doesn't panic
        // 只是确保检测不会 panic
        assert!(!caps.system_id().is_empty());
    }

    #[test]
    fn test_os_display() {
        assert_eq!(format!("{}", Os::Linux), "Linux");
        assert_eq!(format!("{}", Os::MacOS), "macOS");
        assert_eq!(format!("{}", Os::Windows), "Windows");
    }
}
