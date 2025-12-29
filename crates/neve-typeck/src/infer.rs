//! Type inference using Hindley-Milner algorithm.

use neve_common::Span;
use neve_hir::{Ty, TyKind};

/// Type variable counter for fresh variables.
pub struct InferContext {
    next_var: u32,
}

impl InferContext {
    pub fn new() -> Self {
        Self { next_var: 0 }
    }

    /// Create a fresh type variable.
    pub fn fresh_var(&mut self) -> Ty {
        let var = self.next_var;
        self.next_var += 1;
        Ty {
            kind: TyKind::Var(var),
            span: Span::DUMMY,
        }
    }
}

impl Default for InferContext {
    fn default() -> Self {
        Self::new()
    }
}
