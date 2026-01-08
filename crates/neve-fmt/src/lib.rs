//! Code formatter for Neve.
//! Neve 代码格式化器。
//!
//! This crate provides functionality for formatting Neve source code
//! according to a consistent style.
//! 本 crate 提供按照一致风格格式化 Neve 源代码的功能。

mod config;
mod format;
pub mod printer;

pub use config::FormatConfig;
pub use format::Formatter;

use neve_diagnostic::Diagnostic;
use neve_parser::parse;

/// Format Neve source code.
/// 格式化 Neve 源代码。
pub fn format(source: &str) -> Result<String, FormatError> {
    format_with_config(source, &FormatConfig::default())
}

/// Format Neve source code with custom configuration.
/// 使用自定义配置格式化 Neve 源代码。
pub fn format_with_config(source: &str, config: &FormatConfig) -> Result<String, FormatError> {
    let (ast, diagnostics) = parse(source);
    if !diagnostics.is_empty() {
        return Err(FormatError::Parse(diagnostics));
    }

    let formatter = Formatter::new(config.clone());
    Ok(formatter.format(&ast))
}

/// Check if source code is already formatted.
/// 检查源代码是否已格式化。
pub fn check(source: &str) -> Result<bool, FormatError> {
    let formatted = format(source)?;
    Ok(formatted == source)
}

/// Format errors.
/// 格式化错误。
#[derive(Debug, Clone)]
pub enum FormatError {
    /// Parse error. / 解析错误。
    Parse(Vec<Diagnostic>),
    /// I/O error. / I/O 错误。
    Io(String),
}

impl FormatError {
    /// Access diagnostics when formatting failed during parsing.
    /// 获取解析失败时的诊断信息。
    pub fn diagnostics(&self) -> Option<&[Diagnostic]> {
        match self {
            FormatError::Parse(diags) => Some(diags),
            _ => None,
        }
    }
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::Parse(diags) => {
                let summary = diags
                    .iter()
                    .map(|diag| diag.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                write!(f, "parse error: {}", summary)
            }
            FormatError::Io(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for FormatError {}
