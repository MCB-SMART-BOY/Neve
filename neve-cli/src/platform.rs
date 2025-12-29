//! Platform detection and capability reporting.
//!
//! This module provides utilities for detecting platform-specific features
//! and displaying appropriate warnings to users on limited platforms.

use std::fmt;

/// Platform capabilities for Neve.
#[derive(Debug, Clone)]
pub struct PlatformCapabilities {
    /// The current operating system.
    pub os: Os,
    /// The CPU architecture.
    pub arch: Arch,
    /// Whether native sandboxed builds are available.
    pub native_sandbox: bool,
    /// Whether Docker is available for builds.
    pub docker_available: bool,
    /// Whether system configuration is supported.
    pub system_config: bool,
}

/// Operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    MacOS,
    Windows,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
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
    pub fn can_sandbox_build(&self) -> bool {
        self.native_sandbox || self.docker_available
    }

    /// Get the recommended build backend.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildBackend {
    /// Native Linux namespace isolation (full isolation).
    Native,
    /// Docker-based isolation (cross-platform).
    Docker,
    /// Simple execution without isolation.
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
fn detect_os() -> Os {
    match std::env::consts::OS {
        "linux" => Os::Linux,
        "macos" => Os::MacOS,
        "windows" => Os::Windows,
        _ => Os::Other,
    }
}

/// Detect the CPU architecture.
fn detect_arch() -> Arch {
    match std::env::consts::ARCH {
        "x86_64" => Arch::X86_64,
        "aarch64" => Arch::Aarch64,
        _ => Arch::Other,
    }
}

/// Check if Linux namespace support is available.
#[cfg(target_os = "linux")]
fn check_namespace_support() -> bool {
    // Check if unprivileged user namespaces are enabled
    std::fs::read_to_string("/proc/sys/kernel/unprivileged_userns_clone")
        .map(|s| s.trim() == "1")
        .unwrap_or_else(|_| {
            // On some systems, the file doesn't exist but user namespaces work
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
fn check_docker() -> bool {
    std::process::Command::new("docker")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Warn about limited sandbox support on non-Linux platforms.
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
        assert!(!caps.system_id().is_empty());
    }

    #[test]
    fn test_os_display() {
        assert_eq!(format!("{}", Os::Linux), "Linux");
        assert_eq!(format!("{}", Os::MacOS), "macOS");
        assert_eq!(format!("{}", Os::Windows), "Windows");
    }
}
