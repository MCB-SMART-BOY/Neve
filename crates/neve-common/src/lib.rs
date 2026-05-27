//! Common utilities and data structures for Neve.
//! Neve 的通用工具和数据结构。
//!
//! This crate provides foundational types used across the Neve compiler:
//! 本 crate 提供 Neve 编译器中使用的基础类型：
//!
//! - `Span`: Source code location tracking / 源码位置跟踪
//! - `Interner`: String interning for efficient symbol handling / 字符串驻留，用于高效的符号处理
//! - `Arena`: Memory arena for AST allocation / 内存池，用于 AST 分配

mod int;
mod interner;
mod span;

pub use int::{
    Int, int_abs, int_from_f64, int_is_negative, int_is_zero, int_to_f64, int_to_i64, int_to_u32,
    int_to_usize, parse_int, parse_int_radix,
};
pub use interner::{Interner, Symbol};
pub use span::{BytePos, Span};

/// Kill a process by PID using the platform-appropriate mechanism.
/// This is the SINGLE SOURCE OF TRUTH for process termination across all crates.
/// Used by both neve-std (awaitTaskWithTimeout) and neve-eval (streaming timeout).
///
/// Unix: sends SIGKILL via libc::kill
/// Windows: uses taskkill /F /PID
pub fn kill_process(pid: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();
    }
}

/// Check if a builtin function name is effectful (touches the host).
/// This is the single source of truth for effect classification,
/// shared by neve-typeck and neve-std to prevent drift.
pub fn is_effectful_builtin(name: &str) -> bool {
    // Single-segment effectful builtins (v3.0 short aliases)
    if matches!(name, "print" | "println" | "read" | "write" | "cmd" | "env" | "exec" | "run" | "sh" | "ls" | "exists" | "pwd" | "home") {
        return true;
    }
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() >= 2 {
        match parts[0] {
            "io" => !matches!(
                parts[1],
                // Pure inspectors
                "processSuccess" | "processStdout" | "processCode" | "processStderr" |
                // Pure TTY inspectors (no host mutation)
                "isTTY" | "terminalSize" |
                // Pure constructors (no I/O)
                "command" | "commandWith" | "commandWithRedirects" |
                "pipeline" | "pipelineWithRedirects" |
                "redirectStdoutPath" | "redirectStderrPath" | "redirectStdinPath" |
                "taskCommand" | "taskPipeline" |
                "eventMap" | "eventFilter" |
                "reactive" | "liveCurrent" | "liveCancel" |
                "watchFile" | "every" |
                "hashString" | "currentSystem" |
                // Stream constructors and transforms (pure, no I/O)
                "streamLines" | "streamCommand" | "streamList" | "streamBytes" |
                "streamMap" | "streamFilter" | "streamTake" | "streamDrop" |
                "streamWithTimeout"
            ),
            "fetch" => true,
            _ => false,
        }
    } else {
        false
    }
}
