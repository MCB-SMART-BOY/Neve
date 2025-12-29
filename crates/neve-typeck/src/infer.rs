//! Type inference using Hindley-Milner algorithm.
//! 使用 Hindley-Milner 算法的类型推断。
//!
//! This module provides the inference context for generating fresh type variables.
//! 本模块提供用于生成新类型变量的推断上下文。

use neve_common::Span;
use neve_hir::{Ty, TyKind};

/// Type variable counter for fresh variables.
/// 用于生成新类型变量的计数器。
pub struct InferContext {
    /// Next type variable ID to assign.
    /// 下一个要分配的类型变量 ID。
    next_var: u32,
}

impl InferContext {
    pub fn new() -> Self {
        Self { next_var: 0 }
    }

    /// Create a fresh type variable.
    /// 创建一个新的类型变量。
    pub fn fresh_var(&mut self) -> Ty {
        let var = self.next_var;
        self.next_var += 1;
        Ty {
            kind: TyKind::Var(var),
            span: Span::DUMMY,
        }
    }
}

impl Default for InferContext {
    fn default() -> Self {
        Self::new()
    }
}
