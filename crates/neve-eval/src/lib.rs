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
//! - **AST Compatibility Evaluation**: Evaluates the AST directly as an explicit compatibility path.
//!   **AST 兼容求值**：作为显式兼容路径直接对 AST 进行求值。
//!
//! ## Pattern Matching Optimization 模式匹配优化
//!
//! The `pattern` module provides pattern analysis for optimization:
//! `pattern` 模块提供用于优化的模式分析：
//!
//! - Specificity scoring for pattern ordering / 用于模式排序的特异性评分
//! - Fast-path detection for common patterns / 常见模式的快速路径检测
//! - Match expression analysis hints / 匹配表达式分析提示

mod ast_eval;
mod builtin;
mod env;
mod eval;
pub mod compat {
    //! Explicit AST compatibility surface.
    //! 显式 AST 兼容层接口。
    //!
    //! **Deprecated since v3.18.0**: This compatibility path will be removed
    //! in v4.0. The canonical pipeline uses the HIR evaluator (`neve_eval::Evaluator`).
    //! Use `neve_eval::Evaluator` with `neve_frontend` for all new code.
    //!
    //! **自 v3.18.0 起已废弃**: 此兼容路径将在 v4.0 中移除。
    //! 规范管线使用 HIR 求值器 (`neve_eval::Evaluator`)。
    //! 所有新代码请使用 `neve_eval::Evaluator` 配合 `neve_frontend`。

    pub use crate::ast_eval::{AstEnv, AstEvaluator};
}
pub mod pattern;
pub mod value;

pub use builtin::builtins;
pub use env::Environment;
pub use eval::{EvalError, EvaluableModuleRef, Evaluator};
pub use pattern::{MatchHints, Specificity, analyze_match, is_irrefutable, pattern_specificity};
pub use value::{AstClosure, BuiltinFn, Value, stable_key};
