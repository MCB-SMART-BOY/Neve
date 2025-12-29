//! Pretty printer for formatted output.
//! 用于格式化输出的美观打印器。
//!
//! Provides a low-level API for building formatted output with proper
//! indentation and line breaking.
//! 提供用于构建带有适当缩进和换行的格式化输出的底层 API。

use crate::FormatConfig;

/// Pretty printer for building formatted output.
/// 用于构建格式化输出的美观打印器。
pub struct Printer {
    /// Formatting configuration. / 格式化配置。
    config: FormatConfig,
    /// Output buffer. / 输出缓冲区。
    output: String,
    /// Current indentation level. / 当前缩进级别。
    indent_level: usize,
    /// Current line width in characters. / 当前行宽度（字符数）。
    current_line_width: usize,
    /// Whether we're at the start of a line. / 是否在行首。
    at_line_start: bool,
}

impl Printer {
    /// Create a new printer.
    /// 创建新的打印器。
    pub fn new(config: FormatConfig) -> Self {
        Self {
            config,
            output: String::new(),
            indent_level: 0,
            current_line_width: 0,
            at_line_start: true,
        }
    }

    /// Get the formatted output.
    /// 获取格式化后的输出。
    pub fn finish(mut self) -> String {
        if self.config.trailing_newline && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.output
    }

    /// Write a string.
    /// 写入字符串。
    pub fn write(&mut self, s: &str) {
        if self.at_line_start && !s.is_empty() {
            self.write_indent();
            self.at_line_start = false;
        }
        self.output.push_str(s);
        self.current_line_width += s.len();
    }

    /// Write a string and a newline.
    /// 写入字符串和换行符。
    pub fn writeln(&mut self, s: &str) {
        self.write(s);
        self.newline();
    }

    /// Write a newline.
    /// 写入换行符。
    pub fn newline(&mut self) {
        self.output.push('\n');
        self.current_line_width = 0;
        self.at_line_start = true;
    }

    /// Write a space.
    /// 写入空格。
    pub fn space(&mut self) {
        self.write(" ");
    }

    /// Increase indentation level.
    /// 增加缩进级别。
    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease indentation level.
    /// 减少缩进级别。
    pub fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// Write indentation at the current level.
    /// 在当前级别写入缩进。
    fn write_indent(&mut self) {
        let indent = self.config.indent_str().repeat(self.indent_level);
        self.output.push_str(&indent);
        self.current_line_width = indent.len();
    }

    /// Check if adding text would exceed max width.
    /// 检查添加文本是否会超过最大宽度。
    pub fn would_exceed_width(&self, text_len: usize) -> bool {
        self.current_line_width + text_len > self.config.max_width
    }

    /// Get current indentation level.
    /// 获取当前缩进级别。
    pub fn current_indent(&self) -> usize {
        self.indent_level
    }

    /// Get config reference.
    /// 获取配置引用。
    pub fn config(&self) -> &FormatConfig {
        &self.config
    }
}
