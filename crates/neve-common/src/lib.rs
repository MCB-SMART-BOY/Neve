//! Common utilities and data structures for Neve.
//!
//! This crate provides foundational types used across the Neve compiler:
//! - `Span`: Source code location tracking
//! - `Interner`: String interning for efficient symbol handling
//! - `Arena`: Memory arena for AST allocation

mod span;
mod interner;

pub use span::{Span, BytePos};
pub use interner::{Interner, Symbol};
