//! Type checking for Neve.
//!
//! This crate implements Hindley-Milner type inference with extensions
//! for records, traits, and other Neve-specific features.

mod infer;
mod unify;
mod check;
mod traits;
pub mod errors;

pub use check::TypeChecker;
pub use traits::{
    TraitResolver, TraitId, ImplId, TraitInfo, ImplInfo,
    TraitMethod, ImplMethod, TraitBound, TraitConstraint,
    ConstraintSolver, MethodResolution, UnsatisfiedConstraint,
};
pub use errors::format_type;

use neve_diagnostic::Diagnostic;
use neve_hir::Module;

/// Type check a HIR module.
pub fn check(module: &Module) -> Vec<Diagnostic> {
    let mut checker = TypeChecker::new();
    checker.check(module);
    checker.diagnostics()
}
