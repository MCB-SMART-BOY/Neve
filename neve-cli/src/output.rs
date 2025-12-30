//! Output formatting utilities for enhanced CLI experience.
//! 用于增强 CLI 体验的输出格式化工具。
//!
//! Provides colored terminal output, progress indicators, spinners,
//! tables, and structured formatting for the Neve CLI.
//! 为 Neve CLI 提供彩色终端输出、进度指示器、spinner、
//! 表格和结构化格式化。

use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

// ANSI color codes / ANSI 颜色代码
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";

/// Check if the terminal supports colors.
/// 检查终端是否支持颜色。
pub fn supports_color() -> bool {
    // Check NO_COLOR environment variable
    // 检查 NO_COLOR 环境变量
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }

    // Check if stdout is a terminal
    // 检查 stdout 是否为终端
    #[cfg(unix)]
    {
        unsafe { libc::isatty(libc::STDOUT_FILENO) != 0 }
    }

    #[cfg(not(unix))]
    {
        true
    }
}

/// Apply color if supported.
/// 如果支持则应用颜色。
fn colorize(color: &str, text: &str) -> String {
    if supports_color() {
        format!("{}{}{}", color, text, RESET)
    } else {
        text.to_string()
    }
}

/// Print a success message in green with checkmark.
/// 以绿色打印带有勾号的成功消息。
pub fn success(msg: &str) {
    let prefix = if supports_color() { "✓" } else { "[OK]" };
    println!("{} {}", colorize(GREEN, prefix), msg);
}

/// Print a warning message in yellow.
/// 以黄色打印警告消息。
pub fn warning(msg: &str) {
    let prefix = if supports_color() { "⚠" } else { "[WARN]" };
    eprintln!("{} {}", colorize(YELLOW, prefix), msg);
}

/// Print an error message in red.
/// 以红色打印错误消息。
pub fn error(msg: &str) {
    let prefix = if supports_color() { "✗" } else { "[ERROR]" };
    eprintln!("{} {}", colorize(RED, prefix), msg);
}

/// Print an info message in blue.
/// 以蓝色打印信息消息。
pub fn info(msg: &str) {
    let prefix = if supports_color() { "ℹ" } else { "[INFO]" };
    println!("{} {}", colorize(BLUE, prefix), msg);
}

/// Print a debug message in dim text.
/// 以暗色文本打印调试消息。
pub fn debug(msg: &str) {
    if std::env::var("NEVE_DEBUG").is_ok() {
        println!("{}", colorize(DIM, &format!("[debug] {}", msg)));
    }
}

/// Print a header with bold formatting.
/// 以粗体格式打印标题。
pub fn header(msg: &str) {
    println!("\n{}", colorize(BOLD, msg));
    if supports_color() {
        println!("{}", "─".repeat(msg.chars().count()));
    } else {
        println!("{}", "-".repeat(msg.len()));
    }
}

/// Print a section with cyan color.
/// 以青色打印节标题。
pub fn section(msg: &str) {
    println!("\n{}", colorize(CYAN, msg));
}

/// Print a highlight message in magenta.
/// 以洋红色打印高亮消息。
pub fn highlight(msg: &str) {
    println!("{}", colorize(MAGENTA, msg));
}

/// Print a key-value pair.
/// 打印键值对。
pub fn kv(key: &str, value: &str) {
    println!("  {}: {}", colorize(BOLD, key), value);
}

/// Print a list item.
/// 打印列表项。
pub fn list_item(item: &str) {
    let bullet = if supports_color() { "•" } else { "-" };
    println!("  {} {}", bullet, item);
}

/// Print a numbered list item.
/// 打印编号列表项。
pub fn numbered_item(num: usize, item: &str) {
    println!("  {}. {}", colorize(DIM, &num.to_string()), item);
}

/// Status indicator for long-running operations.
/// 长时间运行操作的状态指示器。
pub struct Status {
    message: String,
    done: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Status {
    /// Create a new status indicator.
    /// 创建新的状态指示器。
    pub fn new(message: &str) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();
        let msg = message.to_string();

        let handle = if supports_color() {
            Some(thread::spawn(move || {
                let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                let mut i = 0;
                while !done_clone.load(Ordering::Relaxed) {
                    print!("\r{} {} ", colorize(CYAN, frames[i]), msg);
                    let _ = io::stdout().flush();
                    i = (i + 1) % frames.len();
                    thread::sleep(Duration::from_millis(80));
                }
            }))
        } else {
            print!("{} ... ", msg);
            let _ = io::stdout().flush();
            None
        };

        Self {
            message: message.to_string(),
            done,
            handle,
        }
    }

    /// Mark the operation as successful.
    /// 将操作标记为成功。
    pub fn success(self, msg: Option<&str>) {
        self.done.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle {
            let _ = h.join();
        }
        let final_msg = msg.unwrap_or(&self.message);
        if supports_color() {
            println!("\r{} {} ", colorize(GREEN, "✓"), final_msg);
        } else {
            println!("done");
        }
    }

    /// Mark the operation as failed.
    /// 将操作标记为失败。
    pub fn fail(self, msg: Option<&str>) {
        self.done.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle {
            let _ = h.join();
        }
        let final_msg = msg.unwrap_or(&self.message);
        if supports_color() {
            eprintln!("\r{} {} ", colorize(RED, "✗"), final_msg);
        } else {
            eprintln!("failed");
        }
    }
}

/// Progress bar for operations with known length.
/// 已知长度操作的进度条。
pub struct ProgressBar {
    total: usize,
    current: usize,
    width: usize,
    message: String,
}

impl ProgressBar {
    /// Create a new progress bar.
    /// 创建新的进度条。
    pub fn new(total: usize, message: &str) -> Self {
        Self {
            total,
            current: 0,
            width: 40,
            message: message.to_string(),
        }
    }

    /// Update progress.
    /// 更新进度。
    pub fn update(&mut self, current: usize) {
        self.current = current.min(self.total);
        self.render();
    }

    /// Increment progress by 1.
    /// 将进度增加 1。
    pub fn inc(&mut self) {
        self.current = (self.current + 1).min(self.total);
        self.render();
    }

    /// Render the progress bar.
    /// 渲染进度条。
    fn render(&self) {
        let percent = if self.total > 0 {
            (self.current * 100) / self.total
        } else {
            0
        };
        let filled = (self.current * self.width) / self.total.max(1);
        let empty = self.width - filled;

        if supports_color() {
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
            print!(
                "\r{} {} {} [{}/{}] {}%",
                colorize(CYAN, "⏳"),
                self.message,
                colorize(GREEN, &bar),
                self.current,
                self.total,
                percent
            );
        } else {
            let bar = format!("{}{}", "#".repeat(filled), "-".repeat(empty));
            print!(
                "\r{} [{}] {}/{} {}%",
                self.message, bar, self.current, self.total, percent
            );
        }
        let _ = io::stdout().flush();
    }

    /// Complete the progress bar.
    /// 完成进度条。
    pub fn finish(self) {
        println!();
    }

    /// Complete with a success message.
    /// 以成功消息完成。
    pub fn finish_with_message(self, msg: &str) {
        if supports_color() {
            println!("\r{} {}", colorize(GREEN, "✓"), msg);
        } else {
            println!("\r{} [OK]", msg);
        }
    }
}

/// Simple table for displaying structured data.
/// 用于显示结构化数据的简单表格。
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    column_widths: Vec<usize>,
}

impl Table {
    /// Create a new table with headers.
    /// 创建带有标题的新表格。
    pub fn new(headers: Vec<&str>) -> Self {
        let headers: Vec<String> = headers.into_iter().map(|s| s.to_string()).collect();
        let column_widths = headers.iter().map(|h| h.len()).collect();
        Self {
            headers,
            rows: Vec::new(),
            column_widths,
        }
    }

    /// Add a row to the table.
    /// 向表格添加一行。
    pub fn add_row(&mut self, row: Vec<&str>) {
        let row: Vec<String> = row.into_iter().map(|s| s.to_string()).collect();
        for (i, cell) in row.iter().enumerate() {
            if i < self.column_widths.len() {
                self.column_widths[i] = self.column_widths[i].max(cell.len());
            }
        }
        self.rows.push(row);
    }

    /// Print the table.
    /// 打印表格。
    pub fn print(&self) {
        // Print header
        // 打印标题
        let header_line: String = self
            .headers
            .iter()
            .enumerate()
            .map(|(i, h)| format!("{:width$}", h, width = self.column_widths[i]))
            .collect::<Vec<_>>()
            .join("  ");
        println!("{}", colorize(BOLD, &header_line));

        // Print separator
        // 打印分隔符
        let separator: String = self
            .column_widths
            .iter()
            .map(|w| {
                if supports_color() {
                    "─".repeat(*w)
                } else {
                    "-".repeat(*w)
                }
            })
            .collect::<Vec<_>>()
            .join("  ");
        println!("{}", colorize(DIM, &separator));

        // Print rows
        // 打印行
        for row in &self.rows {
            let line: String = row
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let width = self.column_widths.get(i).copied().unwrap_or(0);
                    format!("{:width$}", cell, width = width)
                })
                .collect::<Vec<_>>()
                .join("  ");
            println!("{}", line);
        }
    }
}

/// Format a duration for display.
/// 格式化持续时间以供显示。
pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Format a byte size for display (binary units).
/// 格式化字节大小以供显示（二进制单位）。
pub fn format_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;

    if bytes < KIB {
        format!("{} B", bytes)
    } else if bytes < MIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else if bytes < GIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    }
}

/// Prompt the user for confirmation.
/// 提示用户确认。
pub fn confirm(prompt: &str) -> bool {
    print!("{} [y/N] ", prompt);
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let input = input.trim().to_lowercase();
        input == "y" || input == "yes"
    } else {
        false
    }
}

/// Prompt the user for input.
/// 提示用户输入。
pub fn prompt(message: &str) -> Option<String> {
    print!("{}: ", message);
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let input = input.trim().to_string();
        if input.is_empty() { None } else { Some(input) }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.00 KiB");
        assert_eq!(format_size(1536), "1.50 KiB");
        assert_eq!(format_size(1048576), "1.00 MiB");
        assert_eq!(format_size(1073741824), "1.00 GiB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3700), "1h 1m");
    }
}
