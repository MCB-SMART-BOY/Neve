//! Lexical analysis for Neve.
//!
//! This crate provides the lexer that converts source code into tokens.

mod token;
mod lexer;

pub use token::{Token, TokenKind};
pub use lexer::Lexer;
