//! Type checking for Neve.
//! Neve 类型检查。
//!
//! This crate implements Hindley-Milner type inference with extensions
//! for records, traits, and other Neve-specific features.
//! 本 crate 实现了 Hindley-Milner 类型推断，并扩展支持记录、特征
//! 和其他 Neve 特有的功能。
//!
//! ## Features 功能
//!
//! - Type inference with let-polymorphism / 带有 let 多态的类型推断
//! - Structural record types / 结构化记录类型
//! - Trait definitions and implementations / 特征定义和实现
//! - Associated types / 关联类型
//! - Generic parameters with bounds / 带约束的泛型参数

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
/// 对 HIR 模块进行类型检查。
///
/// Returns a list of diagnostics (errors and warnings) found during type checking.
/// 返回类型检查过程中发现的诊断信息（错误和警告）列表。
pub fn check(module: &Module) -> Vec<Diagnostic> {
    let mut checker = TypeChecker::new();
    checker.check(module);
    checker.diagnostics()
}
