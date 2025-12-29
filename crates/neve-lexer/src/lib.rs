//! Lexical analysis for Neve.
//!
//! This crate provides the lexer that converts source code into tokens.

mod lexer;
mod token;

pub use lexer::Lexer;
pub use token::{Token, TokenKind};
