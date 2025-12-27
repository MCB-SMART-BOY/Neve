//! High-level Intermediate Representation for Neve.
//!
//! HIR is a desugared representation of the AST after name resolution.
//! It is used as input to the type checker.

mod hir;
mod lower;
mod resolve;

pub use hir::*;
pub use lower::lower;
pub use resolve::Resolver;

