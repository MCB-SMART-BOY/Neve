//! Output formatting utilities.

/// Print a success message.
#[allow(dead_code)]
pub fn success(msg: &str) {
    println!("\x1b[32m{}\x1b[0m", msg);
}

/// Print a warning message.
#[allow(dead_code)]
pub fn warning(msg: &str) {
    eprintln!("\x1b[33mwarning:\x1b[0m {}", msg);
}

/// Print an error message.
#[allow(dead_code)]
pub fn error(msg: &str) {
    eprintln!("\x1b[31merror:\x1b[0m {}", msg);
}

/// Print an info message.
#[allow(dead_code)]
pub fn info(msg: &str) {
    println!("\x1b[34minfo:\x1b[0m {}", msg);
}
