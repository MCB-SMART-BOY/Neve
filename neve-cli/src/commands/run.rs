//! The `neve run` command.

use std::fs;
use std::path::Path;
use neve_parser::parse;
use neve_diagnostic::emit;
use neve_eval::AstEvaluator;

pub fn run(file: &str, verbose: bool) -> Result<(), String> {
    let path = Path::new(file);
    let source = fs::read_to_string(path)
        .map_err(|e| format!("cannot read file '{}': {}", file, e))?;

    let (ast, diagnostics) = parse(&source);

    for diag in &diagnostics {
        emit(&source, file, diag);
    }

    if !diagnostics.is_empty() {
        return Err("parse error".to_string());
    }

    if verbose {
        println!("Parsed {} items", ast.items.len());
    }

    // Evaluate using the AST evaluator with base path for imports
    let mut evaluator = if let Some(parent) = path.parent() {
        AstEvaluator::new().with_base_path(parent.to_path_buf())
    } else {
        AstEvaluator::new()
    };
    
    match evaluator.eval_file(&ast) {
        Ok(value) => {
            // Only print non-unit values
            if !matches!(value, neve_eval::Value::Unit) {
                println!("{:?}", value);
            }
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
            return Err("evaluation error".to_string());
        }
    }

    Ok(())
}
