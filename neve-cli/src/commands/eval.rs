//! The `neve eval` command.

use neve_parser::parse;
use neve_diagnostic::emit;
use neve_eval::AstEvaluator;

pub fn run(expr: &str, verbose: bool) -> Result<(), String> {
    // Wrap expression in a let binding so it can be parsed as an item
    let source = format!("let __result__ = {};", expr);
    
    let (file, diagnostics) = parse(&source);

    for diag in &diagnostics {
        emit(&source, "<eval>", diag);
    }

    if !diagnostics.is_empty() {
        return Err("parse error".to_string());
    }

    if verbose {
        println!("AST: {:?}", file);
    }

    // Evaluate using the AST evaluator
    let mut evaluator = AstEvaluator::new();
    
    match evaluator.eval_file(&file) {
        Ok(value) => {
            println!("{:?}", value);
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
            return Err("evaluation error".to_string());
        }
    }

    Ok(())
}
