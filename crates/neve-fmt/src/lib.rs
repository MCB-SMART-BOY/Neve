//! Code formatter for Neve.
//!
//! This crate provides functionality for formatting Neve source code
//! according to a consistent style.

mod format;
mod config;
pub mod printer;

pub use format::Formatter;
pub use config::FormatConfig;

use neve_lexer::Lexer;
use neve_parser::Parser;

/// Format Neve source code.
pub fn format(source: &str) -> Result<String, FormatError> {
    format_with_config(source, &FormatConfig::default())
}

/// Format Neve source code with custom configuration.
pub fn format_with_config(source: &str, config: &FormatConfig) -> Result<String, FormatError> {
    let lexer = Lexer::new(source);
    let (tokens, errors) = lexer.tokenize();
    
    if !errors.is_empty() {
        return Err(FormatError::Parse(format!("Lexer errors: {:?}", errors)));
    }
    
    let mut parser = Parser::new(tokens);
    let ast = parser.parse_file();
    
    let formatter = Formatter::new(config.clone());
    Ok(formatter.format(&ast))
}

/// Check if source code is already formatted.
pub fn check(source: &str) -> Result<bool, FormatError> {
    let formatted = format(source)?;
    Ok(formatted == source)
}

/// Format errors.
#[derive(Debug, Clone)]
pub enum FormatError {
    /// Parse error.
    Parse(String),
    /// I/O error.
    Io(String),
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::Parse(msg) => write!(f, "parse error: {}", msg),
            FormatError::Io(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for FormatError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple() {
        let source = "let x=1";
        let formatted = format(source).unwrap();
        assert!(formatted.contains("let x = 1"));
    }

    #[test]
    fn test_check() {
        let source = "let x = 1\n";
        let result = check(source);
        assert!(result.is_ok());
    }
}
