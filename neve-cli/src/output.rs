//! Output formatting utilities.
//! 输出格式化工具。
//!
//! These functions provide colored terminal output for the CLI.
//! 这些函数为 CLI 提供彩色终端输出。

/// Print a success message in green.
/// 以绿色打印成功消息。
pub fn success(msg: &str) {
    println!("\x1b[32m{msg}\x1b[0m");
}

/// Print a warning message in yellow.
/// 以黄色打印警告消息。
pub fn warning(msg: &str) {
    eprintln!("\x1b[33mwarning:\x1b[0m {msg}");
}

/// Print an error message in red.
/// 以红色打印错误消息。
pub fn error(msg: &str) {
    eprintln!("\x1b[31merror:\x1b[0m {msg}");
}

/// Print an info message in blue.
/// 以蓝色打印信息消息。
pub fn info(msg: &str) {
    println!("\x1b[34minfo:\x1b[0m {msg}");
}
