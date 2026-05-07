//! Event system, reactive, temporal — new additions live here.

use neve_eval::value::Value;
use std::rc::Rc;

pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![]
}
