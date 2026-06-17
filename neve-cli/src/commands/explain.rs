//! The `neve explain` command — display extended error code documentation.
//! `neve explain` 命令 — 显示扩展的错误代码文档。
//!
//! Similar to `rustc --explain E0001`, this command prints detailed
//! explanations of error codes including descriptions, suggestions,
//! and extended help text.
//! 类似于 `rustc --explain E0001`，此命令打印错误代码的详细说明，
//! 包括描述、建议和扩展帮助文本。

use neve_diagnostic::explain;

/// Run the explain command.
/// 运行 explain 命令。
pub fn run(code: &str) -> Result<(), String> {
    explain(code).map_err(|e| {
        // Also show a helpful hint for querying all error codes
        format!("{}\n\nTip: use `neve doc diagnostics` to browse all error codes.", e)
    })
}
