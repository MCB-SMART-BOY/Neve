//! Derivation model for Neve package management.
//! Neve 包管理的派生模型。
//!
//! A derivation describes how to build a package: its sources, dependencies,
//! build instructions, and outputs. Derivations are content-addressed,
//! meaning their identity is determined by their contents.
//! 派生描述了如何构建一个包：它的源码、依赖、构建指令和输出。
//! 派生是内容寻址的，即其身份由其内容决定。

mod derivation;
mod hash;
mod output;
pub mod resolve;

pub use derivation::*;
pub use hash::*;
pub use output::*;
pub use resolve::{
    Dependency, MemoryRegistry, PackageId, PackageMetadata, PackageRegistry, Resolution,
    ResolveError, Resolver, Version, VersionConstraint, VersionParseError,
};
