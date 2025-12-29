//! Parser for Neve.
//!
//! This crate provides a recursive descent parser that converts
//! tokens into an abstract syntax tree.
//!
//! ## Error Recovery
//!
//! The parser implements error recovery to continue parsing after
//! encountering errors, allowing multiple errors to be reported
//! in a single parse pass.

mod parser;
mod recovery;

pub use parser::Parser;
pub use recovery::{DelimiterKind, DelimiterStack, RecoveryMode};

use neve_diagnostic::Diagnostic;
use neve_lexer::Lexer;
use neve_syntax::SourceFile;

/// Parse source code into an AST.
pub fn parse(source: &str) -> (SourceFile, Vec<Diagnostic>) {
    let lexer = Lexer::new(source);
    let (tokens, mut diagnostics) = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    let file = parser.parse_file();

    diagnostics.extend(parser.diagnostics());
    (file, diagnostics)
}
