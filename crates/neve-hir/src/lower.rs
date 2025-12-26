//! AST to HIR lowering.

use neve_syntax::SourceFile;
use crate::{Module, Resolver};

/// Lower an AST to HIR.
pub fn lower(file: &SourceFile) -> Module {
    let mut resolver = Resolver::new();
    resolver.resolve(file)
}
