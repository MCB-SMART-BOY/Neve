//! Interpreter/evaluator for n3v3 — HIR tree-walking interpreter.
//! n3v3 解释器/求值器 — HIR 树遍历解释器。

mod builtin;
mod diagnostics;
mod env;
mod eval;
pub mod pattern;
pub mod value;

pub use builtin::builtins;
pub use diagnostics::eval_error_to_diagnostic;
pub use env::Environment;
pub use eval::{EvalError, EvaluableModuleRef, Evaluator};
// Internal pattern analysis types — not re-exported (pub(crate) access only)
pub use value::{BuiltinFn, Value, stable_key};
