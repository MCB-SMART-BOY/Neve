//! High-level Intermediate Representation for Neve.
//! Neve 的高级中间表示。
//!
//! HIR is a desugared representation of the AST after name resolution.
//! It is used as input to the type checker.
//! HIR 是经过名称解析后的脱糖 AST 表示，用作类型检查器的输入。

mod hir;
mod incremental;
mod lower;
mod module_diagnostics;
mod module_graph;
mod module_loader;
mod module_lowering;
mod module_paths;
mod resolve;

pub use hir::*;

pub use lower::lower;
pub use module_loader::{
    ImportResolveError, ModuleInfo, ModuleLoadError, ModuleLoader, Visibility,
};
pub use module_paths::{ModulePath, ModulePathKind};
pub use resolve::{
    Resolver, StdBuiltinImportBindings, resolve_std_builtin_import, std_builtin_exports,
    std_builtin_root_modules, supports_canonical_std_import,
};
