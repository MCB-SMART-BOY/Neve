//! Option operations for the standard library.
//! 标准库的 Option 操作。

use neve_eval::value::{BuiltinFn, Value};

/// Returns all option builtins.
/// 返回所有 Option 内置函数。
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // some : a -> Option a
        // Wraps a value in Some / 将值包装为 Some
        (
            "option.some",
            Value::Builtin(BuiltinFn {
                name: "option.some",
                arity: 1,
                func: |args| Ok(Value::Some(Box::new(args[0].clone()))),
            }),
        ),
        // none : Option a
        // The None value / None 值
        ("option.none", Value::None),
        // is_some : Option a -> Bool
        // Checks if option is Some / 检查 option 是否为 Some
        (
            "option.is_some",
            Value::Builtin(BuiltinFn {
                name: "option.is_some",
                arity: 1,
                func: |args| Ok(Value::Bool(matches!(args[0], Value::Some(_)))),
            }),
        ),
        // is_none : Option a -> Bool
        // Checks if option is None / 检查 option 是否为 None
        (
            "option.is_none",
            Value::Builtin(BuiltinFn {
                name: "option.is_none",
                arity: 1,
                func: |args| Ok(Value::Bool(matches!(args[0], Value::None))),
            }),
        ),
        // unwrap : Option a -> a
        // Extracts the value, panics if None / 提取值，如果为 None 则报错
        (
            "option.unwrap",
            Value::Builtin(BuiltinFn {
                name: "option.unwrap",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Some(v) => Ok((**v).clone()),
                    Value::None => Err("called unwrap on None".to_string()),
                    _ => Err("option.unwrap expects an Option".to_string()),
                },
            }),
        ),
        // unwrap_or : Option a -> a -> a
        // Extracts the value or returns default / 提取值或返回默认值
        (
            "option.unwrap_or",
            Value::Builtin(BuiltinFn {
                name: "option.unwrap_or",
                arity: 2,
                func: |args| match &args[0] {
                    Value::Some(v) => Ok((**v).clone()),
                    Value::None => Ok(args[1].clone()),
                    _ => Err("option.unwrap_or expects an Option".to_string()),
                },
            }),
        ),
    ]
}
