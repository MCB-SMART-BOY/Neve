//! Lexical analysis for Neve.
//! Neve 词法分析模块。
//!
//! This crate provides the lexer that converts source code into tokens.
//! 本 crate 提供词法分析器，将源代码转换为 token 序列。

mod lexer;
mod token;

pub use lexer::Lexer;
pub use token::{Token, TokenKind};
