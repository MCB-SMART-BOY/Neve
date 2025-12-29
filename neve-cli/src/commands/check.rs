//! The `neve check` command.

use crate::output;
use neve_diagnostic::emit;
use neve_hir::lower;
use neve_parser::parse;
use neve_typeck::check;
use std::fs;

pub fn run(file: &str, verbose: bool) -> Result<(), String> {
    let source =
        fs::read_to_string(file).map_err(|e| format!("cannot read file '{}': {}", file, e))?;

    // Parse
    let (ast, parse_diagnostics) = parse(&source);

    for diag in &parse_diagnostics {
        emit(&source, file, diag);
    }

    if !parse_diagnostics.is_empty() {
        output::error(&format!("{} parse error(s) found", parse_diagnostics.len()));
        return Err("parse error".to_string());
    }

    if verbose {
        output::info(&format!("Parsed {} items", ast.items.len()));
    }

    // Lower to HIR
    let hir = lower(&ast);

    if verbose {
        output::info(&format!("Lowered to {} HIR items", hir.items.len()));
    }

    // Type check
    let type_diagnostics = check(&hir);

    for diag in &type_diagnostics {
        emit(&source, file, diag);
    }

    if !type_diagnostics.is_empty() {
        output::error(&format!("{} type error(s) found", type_diagnostics.len()));
        return Err("type error".to_string());
    }

    output::success("OK - No errors found");
    Ok(())
}
