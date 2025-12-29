//! High-level Intermediate Representation for Neve.
//! Neve 的高级中间表示。
//!
//! HIR is a desugared representation of the AST after name resolution.
//! It is used as input to the type checker.
//! HIR 是经过名称解析后的脱糖 AST 表示，用作类型检查器的输入。

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
