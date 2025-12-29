//! Interpreter/evaluator for Neve.
//!
//! This crate implements a tree-walking interpreter for HIR.

pub mod ast_eval;
mod builtin;
mod env;
mod eval;
pub mod value;

pub use ast_eval::{AstEnv, AstEvaluator};
pub use builtin::builtins;
pub use env::Environment;
pub use eval::{EvalError, Evaluator};
pub use value::{AstClosure, BuiltinFn, Value};
