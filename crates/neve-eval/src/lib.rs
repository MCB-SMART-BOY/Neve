//! Interpreter/evaluator for Neve — HIR tree-walking interpreter.
//! Neve 解释器/求值器 — HIR 树遍历解释器。

mod builtin;
mod env;
mod eval;
pub mod pattern;
pub mod value;

pub use builtin::builtins;
pub use env::Environment;
pub use eval::{EvalError, EvaluableModuleRef, Evaluator};
// Internal pattern analysis types — not re-exported (pub(crate) access only)
pub use value::{BuiltinFn, Value, stable_key};
