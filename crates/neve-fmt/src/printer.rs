//! Pretty printer for formatted output.

use crate::FormatConfig;

/// Pretty printer for building formatted output.
pub struct Printer {
    config: FormatConfig,
    output: String,
    indent_level: usize,
    current_line_width: usize,
    at_line_start: bool,
}

impl Printer {
    /// Create a new printer.
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
    pub fn finish(mut self) -> String {
        if self.config.trailing_newline && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.output
    }

    /// Write a string.
    pub fn write(&mut self, s: &str) {
        if self.at_line_start && !s.is_empty() {
            self.write_indent();
            self.at_line_start = false;
        }
        self.output.push_str(s);
        self.current_line_width += s.len();
    }

    /// Write a string and a newline.
    pub fn writeln(&mut self, s: &str) {
        self.write(s);
        self.newline();
    }

    /// Write a newline.
    pub fn newline(&mut self) {
        self.output.push('\n');
        self.current_line_width = 0;
        self.at_line_start = true;
    }

    /// Write a space.
    pub fn space(&mut self) {
        self.write(" ");
    }

    /// Increase indentation level.
    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease indentation level.
    pub fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// Write indentation at the current level.
    fn write_indent(&mut self) {
        let indent = self.config.indent_str().repeat(self.indent_level);
        self.output.push_str(&indent);
        self.current_line_width = indent.len();
    }

    /// Check if adding text would exceed max width.
    pub fn would_exceed_width(&self, text_len: usize) -> bool {
        self.current_line_width + text_len > self.config.max_width
    }

    /// Get current indentation level.
    pub fn current_indent(&self) -> usize {
        self.indent_level
    }

    /// Get config reference.
    pub fn config(&self) -> &FormatConfig {
        &self.config
    }
}

