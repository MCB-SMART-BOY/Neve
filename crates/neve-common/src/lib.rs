//! Common utilities and data structures for Neve.
//!
//! This crate provides foundational types used across the Neve compiler:
//! - `Span`: Source code location tracking
//! - `Interner`: String interning for efficient symbol handling
//! - `Arena`: Memory arena for AST allocation

mod interner;
mod span;

pub use interner::{Interner, Symbol};
pub use span::{BytePos, Span};
