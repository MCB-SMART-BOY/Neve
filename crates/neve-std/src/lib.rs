//! Standard library for Neve.
//! Neve 标准库。
//!
//! This crate provides the built-in functions and types for Neve.
//! 本 crate 提供 Neve 的内置函数和类型。

mod bytes;
mod fetch;
mod io;
mod list;
mod map;
mod math;
mod option;
mod path;
mod result;
mod set;
mod string;

use neve_eval::Value;

pub use io::set_script_args;

/// Returns true if the named builtin performs host effects (I/O, process execution, network).
/// Delegates to neve_common::is_effectful_builtin (single source of truth).
pub fn is_effectful_builtin(name: &str) -> bool {
    neve_common::is_effectful_builtin(name)
}

/// Initialize the standard library and return all built-in bindings.
/// 初始化标准库并返回所有内置绑定。
pub fn stdlib() -> Vec<(&'static str, Value)> {
    let mut bindings = Vec::new();
    bindings.extend(bytes::builtins());
    bindings.extend(fetch::builtins());
    bindings.extend(io::builtins());
    bindings.extend(list::builtins());
    bindings.extend(map::builtins());
    bindings.extend(math::builtins());
    bindings.extend(option::builtins());
    bindings.extend(path::builtins());
    bindings.extend(result::builtins());
    bindings.extend(set::builtins());
    bindings.extend(string::builtins());
    bindings
}
