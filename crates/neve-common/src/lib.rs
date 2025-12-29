//! Common utilities and data structures for Neve.
//! Neve 的通用工具和数据结构。
//!
//! This crate provides foundational types used across the Neve compiler:
//! 本 crate 提供 Neve 编译器中使用的基础类型：
//!
//! - `Span`: Source code location tracking / 源码位置跟踪
//! - `Interner`: String interning for efficient symbol handling / 字符串驻留，用于高效的符号处理
//! - `Arena`: Memory arena for AST allocation / 内存池，用于 AST 分配

mod interner;
mod span;

pub use interner::{Interner, Symbol};
pub use span::{BytePos, Span};
