//! AST to HIR lowering.

use crate::{Module, Resolver};
use neve_syntax::SourceFile;

/// Lower an AST to HIR.
pub fn lower(file: &SourceFile) -> Module {
    let mut resolver = Resolver::new();
    resolver.resolve(file)
}
