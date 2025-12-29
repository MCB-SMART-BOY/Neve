//! High-level Intermediate Representation for Neve.
//!
//! HIR is a desugared representation of the AST after name resolution.
//! It is used as input to the type checker.

mod hir;
mod lower;
mod module_loader;
mod resolve;

pub use hir::*;
pub use lower::lower;
pub use module_loader::{
    ImportResolveError, ModuleInfo, ModuleLoadError, ModuleLoader, ModulePath, ModulePathKind,
    Visibility,
};
pub use resolve::Resolver;
