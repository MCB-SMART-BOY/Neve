//! Formatter configuration.

/// Formatter configuration.
#[derive(Debug, Clone)]
pub struct FormatConfig {
    /// Indentation width (in spaces).
    pub indent_width: usize,
    /// Maximum line width.
    pub max_width: usize,
    /// Use tabs instead of spaces.
    pub use_tabs: bool,
    /// Add trailing newline.
    pub trailing_newline: bool,
    /// Space before function argument list.
    pub space_before_parens: bool,
    /// Space inside braces for records.
    pub space_inside_braces: bool,
    /// Break long lists across multiple lines.
    pub break_long_lists: bool,
    /// Add blank line between top-level items.
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indentation width.
    pub fn indent_width(mut self, width: usize) -> Self {
        self.indent_width = width;
        self
    }

    /// Set the maximum line width.
    pub fn max_width(mut self, width: usize) -> Self {
        self.max_width = width;
        self
    }

    /// Use tabs instead of spaces.
    pub fn use_tabs(mut self, use_tabs: bool) -> Self {
        self.use_tabs = use_tabs;
        self
    }

    /// Get the indentation string for one level.
    pub fn indent_str(&self) -> String {
        if self.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(self.indent_width)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = FormatConfig::default();
        assert_eq!(config.indent_width, 2);
        assert_eq!(config.max_width, 100);
        assert!(!config.use_tabs);
    }

    #[test]
    fn test_indent_str() {
        let config = FormatConfig::new().indent_width(4);
        assert_eq!(config.indent_str(), "    ");
        
        let config = FormatConfig::new().use_tabs(true);
        assert_eq!(config.indent_str(), "\t");
    }
}
