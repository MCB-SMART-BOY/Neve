//! CLI command implementations.
//! CLI 命令实现。

// Cross-platform commands (language features)
// 跨平台命令（语言功能）
pub mod check;
pub mod doc;
pub mod eval;
pub mod fmt;
pub mod info;
pub mod repl;
pub mod run;

// Unix-only commands (package management)
// 仅限 Unix 的命令（软件包管理）
#[cfg(unix)]
pub mod build;
#[cfg(unix)]
pub mod config;
#[cfg(unix)]
pub mod install;
#[cfg(unix)]
pub mod remove;
#[cfg(unix)]
pub mod search;
#[cfg(unix)]
pub mod store;
#[cfg(unix)]
pub mod update;
