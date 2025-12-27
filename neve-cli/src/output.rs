//! Output formatting utilities.
//!
//! These functions provide colored terminal output for the CLI.

/// Print a success message in green.
pub fn success(msg: &str) {
    println!("\x1b[32m{msg}\x1b[0m");
}

/// Print a warning message in yellow.
pub fn warning(msg: &str) {
    eprintln!("\x1b[33mwarning:\x1b[0m {msg}");
}

/// Print an error message in red.
pub fn error(msg: &str) {
    eprintln!("\x1b[31merror:\x1b[0m {msg}");
}

/// Print an info message in blue.
pub fn info(msg: &str) {
    println!("\x1b[34minfo:\x1b[0m {msg}");
}
