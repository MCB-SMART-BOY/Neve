//! AST and syntax definitions for n3v3.
//! n3v3 的抽象语法树和语法定义。
//!
//! This crate defines the abstract syntax tree used by the parser
//! and subsequent compilation phases.
//! 本 crate 定义了解析器和后续编译阶段使用的抽象语法树。
//!
//! Public AST enums are intentionally non-exhaustive. External consumers must
//! include a wildcard arm when matching them so future syntax additions do not
//! silently break downstream builds.
//!
//! ```compile_fail
//! use n3v3_syntax::ItemKind;
//!
//! fn describe(item: ItemKind) -> &'static str {
//!     match item {
//!         ItemKind::Let(_) => "let",
//!         ItemKind::Fn(_) => "fn",
//!         ItemKind::TypeAlias(_) => "type alias",
//!         ItemKind::Struct(_) => "struct",
//!         ItemKind::Enum(_) => "enum",
//!         ItemKind::Trait(_) => "trait",
//!         ItemKind::Impl(_) => "impl",
//!         ItemKind::Import(_) => "import",
//!         ItemKind::ExprStmt(_) => "expression",
//!     }
//! }
//! ```

mod ast;
mod expr;
mod pattern;
mod types;

pub use ast::*;
pub use expr::*;
pub use pattern::*;
pub use types::*;
