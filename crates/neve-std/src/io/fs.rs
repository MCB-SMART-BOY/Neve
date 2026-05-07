//! File system operations — new additions live here.
//! Legacy functions remain in io/mod.rs.

use neve_eval::value::Value;
use std::rc::Rc;

/// New builtins for the fs submodule.
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![]
}
