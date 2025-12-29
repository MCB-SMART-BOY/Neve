//! Interpreter/evaluator for Neve.
//! Neve 解释器/求值器。
//!
//! This crate implements a tree-walking interpreter for HIR.
//! 本 crate 实现了 HIR 的树遍历解释器。
//!
//! ## Architecture 架构
//!
//! The evaluator supports two modes:
//! 求值器支持两种模式：
//!
//! - **HIR Evaluation**: Evaluates lowered HIR (High-level IR) for optimized execution.
//!   **HIR 求值**：对降级后的 HIR（高级中间表示）进行求值，以实现优化执行。
//!
//! - **AST Evaluation**: Evaluates the AST directly, useful for REPL and quick prototyping.
//!   **AST 求值**：直接对 AST 进行求值，适用于 REPL 和快速原型开发。

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
