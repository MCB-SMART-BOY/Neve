//! The `neve eval` command.
//! `neve eval` 命令。

use crate::output;
use neve_diagnostic::emit;
use neve_eval::AstEvaluator;
use neve_parser::parse;

/// Run the eval command.
/// 运行 eval 命令。
pub fn run(expr: &str, verbose: bool) -> Result<(), String> {
    // Prepare source for parsing
    // 准备用于解析的源码
    // Strategy: if there's content after the last semicolon that looks like an expression,
    // wrap it in a let binding so it becomes a valid item
    let source = prepare_source(expr);

    let (file, diagnostics) = parse(&source);

    for diag in &diagnostics {
        emit(&source, "<eval>", diag);
    }

    if !diagnostics.is_empty() {
        return Err("parse error".to_string());
    }

    eval_and_print(&file, &source, verbose)
}

/// Prepare the source for parsing by wrapping expressions appropriately.
/// 通过适当包装表达式来准备用于解析的源码。
fn prepare_source(expr: &str) -> String {
    let trimmed = expr.trim();

    if trimmed.is_empty() {
        return String::new();
    }

    // Check if it's already a valid item (starts with keyword)
    // 检查是否已经是有效的项（以关键字开头）
    let is_item = trimmed.starts_with("let ")
        || trimmed.starts_with("fn ")
        || trimmed.starts_with("type ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("enum ")
        || trimmed.starts_with("trait ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("pub ");

    if is_item {
        // It's already an item, just ensure it ends with semicolon
        // 已经是一个项，只需确保以分号结尾
        if trimmed.ends_with(';') {
            return trimmed.to_string();
        } else {
            return format!("{trimmed};");
        }
    }

    // For expressions, wrap in a block-based let binding
    // 对于表达式，包装在基于块的 let 绑定中
    // This handles expressions like `{ let x = 1; x * 2 }` or simple `1 + 2`
    // 这处理像 `{ let x = 1; x * 2 }` 或简单的 `1 + 2` 这样的表达式
    // We wrap the expression: let __result__ = <expr>;
    // 我们包装表达式：let __result__ = <expr>;
    // But if it's a block expression, it will work directly
    // 但如果是块表达式，它将直接工作
    format!("let __result__ = {trimmed};")
}

/// Evaluate and print the result.
/// 求值并打印结果。
fn eval_and_print(
    file: &neve_syntax::SourceFile,
    source: &str,
    verbose: bool,
) -> Result<(), String> {
    if verbose {
        output::info(&format!("AST: {file:?}"));
    }

    // Evaluate using the AST evaluator
    // 使用 AST 求值器进行求值
    let mut evaluator = AstEvaluator::new();

    match evaluator.eval_file(file) {
        Ok(value) => {
            // Don't print Unit for statements that don't return values
            // 对于不返回值的语句，不打印 Unit
            if !matches!(value, neve_eval::Value::Unit) || source.starts_with("let __result__") {
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
