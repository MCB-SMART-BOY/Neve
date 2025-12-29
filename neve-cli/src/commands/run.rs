//! The `neve run` command.
//! `neve run` 命令。

use crate::output;
use neve_diagnostic::emit;
use neve_eval::AstEvaluator;
use neve_parser::parse;
use std::fs;
use std::path::Path;

/// Run a Neve file.
/// 运行 Neve 文件。
pub fn run(file: &str, verbose: bool) -> Result<(), String> {
    let path = Path::new(file);
    let source = fs::read_to_string(path).map_err(|e| format!("cannot read file '{file}': {e}"))?;

    let (ast, diagnostics) = parse(&source);

    for diag in &diagnostics {
        emit(&source, file, diag);
    }

    if !diagnostics.is_empty() {
        return Err("parse error".to_string());
    }

    if verbose {
        output::info(&format!("Parsed {} items", ast.items.len()));
    }

    // Evaluate using the AST evaluator with base path for imports
    // 使用带有导入基础路径的 AST 求值器进行求值
    let mut evaluator = if let Some(parent) = path.parent() {
        AstEvaluator::new().with_base_path(parent.to_path_buf())
    } else {
        AstEvaluator::new()
    };

    match evaluator.eval_file(&ast) {
        Ok(value) => {
            // Only print non-unit values
            // 只打印非 unit 值
            if !matches!(value, neve_eval::Value::Unit) {
                output::success(&format!("{value:?}"));
            }
        }
        Err(e) => {
            output::error(&format!("{e:?}"));
            return Err("evaluation error".to_string());
        }
    }

    Ok(())
}
