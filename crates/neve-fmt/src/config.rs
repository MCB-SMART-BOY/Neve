//! Formatter configuration.
//! 格式化器配置。
//!
//! Defines configuration options for code formatting.
//! 定义代码格式化的配置选项。

/// Formatter configuration.
/// 格式化器配置。
#[derive(Debug, Clone)]
pub struct FormatConfig {
    /// Indentation width (in spaces). / 缩进宽度（空格数）。
    pub indent_width: usize,
    /// Maximum line width. / 最大行宽。
    pub max_width: usize,
    /// Use tabs instead of spaces. / 使用制表符代替空格。
    pub use_tabs: bool,
    /// Add trailing newline. / 添加尾随换行符。
    pub trailing_newline: bool,
    /// Space before function argument list. / 函数参数列表前的空格。
    pub space_before_parens: bool,
    /// Space inside braces for records. / 记录大括号内的空格。
    pub space_inside_braces: bool,
    /// Break long lists across multiple lines. / 将长列表拆分为多行。
    pub break_long_lists: bool,
    /// Add blank line between top-level items. / 在顶级项之间添加空行。
    pub blank_lines_between_items: bool,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            indent_width: 2,
            max_width: 100,
            use_tabs: false,
            trailing_newline: true,
            space_before_parens: false,
            space_inside_braces: true,
            break_long_lists: true,
            blank_lines_between_items: false,
        }
    }
}

impl FormatConfig {
    /// Create a new configuration with default settings.
    /// 使用默认设置创建新配置。
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indentation width.
    /// 设置缩进宽度。
    pub fn indent_width(mut self, width: usize) -> Self {
        self.indent_width = width;
        self
    }

    /// Set the maximum line width.
    /// 设置最大行宽。
    pub fn max_width(mut self, width: usize) -> Self {
        self.max_width = width;
        self
    }

    /// Use tabs instead of spaces.
    /// 使用制表符代替空格。
    pub fn use_tabs(mut self, use_tabs: bool) -> Self {
        self.use_tabs = use_tabs;
        self
    }

    /// Get the indentation string for one level.
    /// 获取一级缩进的字符串。
    pub fn indent_str(&self) -> String {
        if self.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(self.indent_width)
        }
    }
}
