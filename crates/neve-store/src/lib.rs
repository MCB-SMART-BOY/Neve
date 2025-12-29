//! Content-addressed store for Neve.
//! Neve 的内容寻址存储。
//!
//! The store is where all build outputs and derivations are kept.
//! Paths in the store are content-addressed, meaning their names
//! include a hash of their contents.
//! 存储是保存所有构建输出和派生的地方。
//! 存储中的路径是内容寻址的，即路径名包含其内容的哈希值。

pub mod cache;
mod db;
pub mod gc;
mod path;
mod store;

pub use cache::*;
pub use db::*;
pub use gc::*;
pub use path::*;
pub use store::*;
