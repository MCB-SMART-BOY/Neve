//! AST to HIR lowering.
//! AST 到 HIR 的降级转换。

use crate::{Module, Resolver};
use neve_syntax::SourceFile;

/// Lower an AST to HIR.
/// 将 AST 降级为 HIR。
pub fn lower(file: &SourceFile) -> Module {
    let mut resolver = Resolver::new();
    resolver.resolve(file)
}
