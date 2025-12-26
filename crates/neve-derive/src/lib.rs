//! Derivation model for Neve package management.
//!
//! A derivation describes how to build a package: its sources, dependencies,
//! build instructions, and outputs. Derivations are content-addressed,
//! meaning their identity is determined by their contents.

mod derivation;
mod hash;
mod output;
pub mod resolve;

pub use derivation::*;
pub use hash::*;
pub use output::*;
pub use resolve::{
    Resolver, Resolution, ResolveError,
    PackageId, Version, VersionConstraint, VersionParseError,
    Dependency, PackageMetadata, PackageRegistry, MemoryRegistry,
};
