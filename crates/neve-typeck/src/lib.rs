//! Type checking for Neve.
//!
//! This crate implements Hindley-Milner type inference with extensions
//! for records, traits, and other Neve-specific features.

mod check;
pub mod errors;
mod infer;
mod traits;
mod unify;

pub use check::TypeChecker;
pub use errors::format_type;
pub use traits::{
    ConstraintSolver, ImplId, ImplInfo, ImplMethod, MethodResolution, TraitBound, TraitConstraint,
    TraitId, TraitInfo, TraitMethod, TraitResolver, UnsatisfiedConstraint,
};

use neve_diagnostic::Diagnostic;
use neve_hir::Module;

/// Type check a HIR module.
pub fn check(module: &Module) -> Vec<Diagnostic> {
    let mut checker = TypeChecker::new();
    checker.check(module);
    checker.diagnostics()
}
