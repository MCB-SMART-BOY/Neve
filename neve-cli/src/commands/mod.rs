//! CLI command implementations.
//! CLI 命令实现。

// Cross-platform commands (language features)
// 跨平台命令（语言功能）
pub mod check;
mod diagnostics;
pub mod doc;
pub mod eval;
pub mod fmt;
pub mod info;
pub mod init;
pub mod module_graph;
pub mod repl;
pub mod run;
pub mod test;

// Unix-only commands (package management)
// 仅限 Unix 的命令（软件包管理）
#[cfg(unix)]
pub mod build;
#[cfg(unix)]
pub mod config;
#[cfg(unix)]
pub mod install;
pub mod lsp;
#[cfg(unix)]
pub mod registry;
#[cfg(unix)]
pub mod registry_publish;
#[cfg(unix)]
pub mod registry_serve;
#[cfg(unix)]
pub mod remove;
#[cfg(unix)]
pub mod search;
pub mod setup_helix;
#[cfg(unix)]
pub mod store;
#[cfg(unix)]
pub mod update;
