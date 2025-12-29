//! The `neve eval` command.

use crate::output;
use neve_diagnostic::emit;
use neve_eval::AstEvaluator;
use neve_parser::parse;

pub fn run(expr: &str, verbose: bool) -> Result<(), String> {
    // Prepare source for parsing
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
fn prepare_source(expr: &str) -> String {
    let trimmed = expr.trim();
    
    if trimmed.is_empty() {
        return String::new();
    }
    
    // Check if it's already a valid item (starts with keyword)
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
        if trimmed.ends_with(';') {
            return trimmed.to_string();
        } else {
            return format!("{trimmed};");
        }
    }
    
    // For expressions, wrap in a block-based let binding
    // This handles expressions like `{ let x = 1; x * 2 }` or simple `1 + 2`
    // We wrap the expression: let __result__ = <expr>;
    // But if it's a block expression, it will work directly
    format!("let __result__ = {trimmed};")
}

fn eval_and_print(file: &neve_syntax::SourceFile, source: &str, verbose: bool) -> Result<(), String> {
    if verbose {
        output::info(&format!("AST: {file:?}"));
    }

    // Evaluate using the AST evaluator
    let mut evaluator = AstEvaluator::new();

    match evaluator.eval_file(file) {
        Ok(value) => {
            // Don't print Unit for statements that don't return values
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
