//! Standard library for Neve.
//!
//! This crate provides the built-in functions and types for Neve.

mod list;
mod map;
mod set;
mod string;
mod option;
mod result;
mod path;

use neve_eval::Value;

/// Initialize the standard library and return all built-in bindings.
pub fn stdlib() -> Vec<(&'static str, Value)> {
    let mut bindings = Vec::new();
    bindings.extend(list::builtins());
    bindings.extend(map::builtins());
    bindings.extend(set::builtins());
    bindings.extend(string::builtins());
    bindings.extend(option::builtins());
    bindings.extend(result::builtins());
    bindings.extend(path::builtins());
    bindings
}
