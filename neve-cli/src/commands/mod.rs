//! CLI command implementations.

// Cross-platform commands (language features)
pub mod check;
pub mod eval;
pub mod fmt;
pub mod info;
pub mod repl;
pub mod run;

// Unix-only commands (package management)
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
