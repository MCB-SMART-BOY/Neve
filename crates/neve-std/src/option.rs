//! Option operations for the standard library.

use neve_eval::value::{Value, BuiltinFn};

pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        ("option.some", Value::Builtin(BuiltinFn {
            name: "option.some",
            arity: 1,
            func: |args| {
                Ok(Value::Some(Box::new(args[0].clone())))
            },
        })),
        ("option.none", Value::None),
        ("option.is_some", Value::Builtin(BuiltinFn {
            name: "option.is_some",
            arity: 1,
            func: |args| {
                Ok(Value::Bool(matches!(args[0], Value::Some(_))))
            },
        })),
        ("option.is_none", Value::Builtin(BuiltinFn {
            name: "option.is_none",
            arity: 1,
            func: |args| {
                Ok(Value::Bool(matches!(args[0], Value::None)))
            },
        })),
        ("option.unwrap", Value::Builtin(BuiltinFn {
            name: "option.unwrap",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Some(v) => Ok((**v).clone()),
                    Value::None => Err("called unwrap on None".to_string()),
                    _ => Err("option.unwrap expects an Option".to_string()),
                }
            },
        })),
        ("option.unwrap_or", Value::Builtin(BuiltinFn {
            name: "option.unwrap_or",
            arity: 2,
            func: |args| {
                match &args[0] {
                    Value::Some(v) => Ok((**v).clone()),
                    Value::None => Ok(args[1].clone()),
                    _ => Err("option.unwrap_or expects an Option".to_string()),
                }
            },
        })),
    ]
}
