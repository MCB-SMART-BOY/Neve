//! Interpreter/evaluator for Neve.
//!
//! This crate implements a tree-walking interpreter for HIR.

mod env;
mod eval;
mod builtin;
pub mod ast_eval;
pub mod value;

pub use value::{Value, BuiltinFn, AstClosure};
pub use env::Environment;
pub use eval::{Evaluator, EvalError};
pub use ast_eval::{AstEvaluator, AstEnv};
