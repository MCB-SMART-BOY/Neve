//! Parser for n3v3.
//! n3v3 语法解析器。
//!
//! This crate provides a recursive descent parser that converts
//! tokens into an abstract syntax tree.
//! 本 crate 提供递归下降解析器，将 token 序列转换为抽象语法树。
//!
//! ## Error Recovery 错误恢复
//!
//! The parser implements error recovery to continue parsing after
//! encountering errors, allowing multiple errors to be reported
//! in a single parse pass.
//! 解析器实现了错误恢复机制，在遇到错误后可以继续解析，
//! 从而在单次解析过程中报告多个错误。

mod parser;
mod recovery;

pub use parser::Parser;
pub use recovery::{DelimiterKind, DelimiterStack, RecoveryMode};

use n3v3_diagnostic::Diagnostic;
use n3v3_lexer::Lexer;
use n3v3_syntax::SourceFile;

/// Parse source code into an AST.
/// 将源代码解析为抽象语法树（AST）。
///
/// This is the main entry point for parsing n3v3 source code.
/// 这是解析 n3v3 源代码的主入口函数。
///
/// # Arguments 参数
/// * `source` - The source code to parse / 要解析的源代码
///
/// # Returns 返回值
/// A tuple containing the parsed source file and any diagnostics.
/// 返回一个元组，包含解析后的源文件和所有诊断信息。
pub fn parse(source: &str) -> (SourceFile, Vec<Diagnostic>) {
    // Strip shebang line if present so that executable n3v3 scripts starting
    // with #!/usr/bin/env n3v3 parse correctly.
    let source = if source.starts_with("#!") {
        source.find('\n').map_or("", |i| &source[i + 1..])
    } else {
        source
    };
    let lexer = Lexer::new(source);
    let (tokens, comments, mut diagnostics) = lexer.tokenize_with_trivia();

    let mut parser = Parser::new_with_comments(tokens, comments);
    let file = parser.parse_file();

    diagnostics.extend(parser.diagnostics());
    (file, diagnostics)
}
