//! Standard library for Neve.
//! Neve 标准库。
//!
//! This crate provides the built-in functions and types for Neve.
//! 本 crate 提供 Neve 的内置函数和类型。

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

use neve_eval::{AstEnv, Value};
use std::collections::HashMap;
use std::rc::Rc;

/// Initialize the standard library and return all built-in bindings.
/// 初始化标准库并返回所有内置绑定。
pub fn stdlib() -> Vec<(&'static str, Value)> {
    let mut bindings = Vec::new();
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

/// Build module overrides for the stdlib (`std.*`) import paths.
/// 构建标准库模块覆盖（`std.*` 导入路径）。
pub fn std_module_overrides() -> HashMap<Vec<String>, Rc<AstEnv>> {
    let mut module_bindings: HashMap<Vec<String>, Vec<(String, Value)>> = HashMap::new();

    for (name, value) in stdlib() {
        let segments: Vec<&str> = name.split('.').collect();
        if segments.len() < 2 {
            continue;
        }

        let (module_segments, item_name) = segments.split_at(segments.len() - 1);
        let mut module_path = Vec::with_capacity(module_segments.len() + 1);
        module_path.push("std".to_string());
        module_path.extend(module_segments.iter().map(|seg| (*seg).to_string()));

        module_bindings
            .entry(module_path)
            .or_default()
            .push((item_name[0].to_string(), value));
    }

    let mut overrides: HashMap<Vec<String>, Rc<AstEnv>> = HashMap::new();
    let mut std_env = AstEnv::new();

    for (path, bindings) in module_bindings {
        let mut env = AstEnv::new();
        for (name, value) in bindings {
            env.define_pub(name, value);
        }
        let env = Rc::new(env);

        if path.len() == 2 && path[0] == "std" {
            let module_name = path[1].clone();
            let record = Value::Record(Rc::new(env.public_bindings()));
            std_env.define_pub(module_name, record);
        }

        overrides.insert(path, env);
    }

    overrides.insert(vec!["std".to_string()], Rc::new(std_env));
    overrides
}
