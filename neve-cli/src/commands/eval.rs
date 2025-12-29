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

/// Prepare the source for parsing by wrapping trailing expressions in let bindings.
fn prepare_source(expr: &str) -> String {
    let trimmed = expr.trim();
    
    if trimmed.is_empty() {
        return String::new();
    }
    
    // If no semicolons, wrap the whole thing as a let binding
    if !trimmed.contains(';') {
        return format!("let __result__ = {trimmed};");
    }
    
    // Find the last semicolon
    if let Some(last_semi_pos) = trimmed.rfind(';') {
        let before = &trimmed[..=last_semi_pos];
        let after = trimmed[last_semi_pos + 1..].trim();
        
        // If there's content after the last semicolon, wrap it as a result
        if !after.is_empty() {
            // Check if the trailing part is already a definition (starts with keyword)
            let is_definition = after.starts_with("let ")
                || after.starts_with("fn ")
                || after.starts_with("type ")
                || after.starts_with("struct ")
                || after.starts_with("enum ")
                || after.starts_with("trait ")
                || after.starts_with("impl ")
                || after.starts_with("import ")
                || after.starts_with("pub ");
            
            if is_definition {
                // It's a definition, just add trailing semicolon if needed
                if after.ends_with(';') {
                    format!("{before}{after}")
                } else {
                    format!("{before}{after};")
                }
            } else {
                // It's an expression, wrap it
                format!("{before}let __result__ = {after};")
            }
        } else {
            // Nothing after the last semicolon
            before.to_string()
        }
    } else {
        // No semicolons found (shouldn't reach here due to earlier check)
        format!("let __result__ = {trimmed};")
    }
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
