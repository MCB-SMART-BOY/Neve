//! Result operations for the standard library.

use neve_eval::value::{BuiltinFn, Value};

pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        (
            "result.ok",
            Value::Builtin(BuiltinFn {
                name: "result.ok",
                arity: 1,
                func: |args| Ok(Value::Ok(Box::new(args[0].clone()))),
            }),
        ),
        (
            "result.err",
            Value::Builtin(BuiltinFn {
                name: "result.err",
                arity: 1,
                func: |args| Ok(Value::Err(Box::new(args[0].clone()))),
            }),
        ),
        (
            "result.is_ok",
            Value::Builtin(BuiltinFn {
                name: "result.is_ok",
                arity: 1,
                func: |args| Ok(Value::Bool(matches!(args[0], Value::Ok(_)))),
            }),
        ),
        (
            "result.is_err",
            Value::Builtin(BuiltinFn {
                name: "result.is_err",
                arity: 1,
                func: |args| Ok(Value::Bool(matches!(args[0], Value::Err(_)))),
            }),
        ),
        (
            "result.unwrap",
            Value::Builtin(BuiltinFn {
                name: "result.unwrap",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Ok(v) => Ok((**v).clone()),
                    Value::Err(e) => Err(format!("called unwrap on Err: {:?}", e)),
                    _ => Err("result.unwrap expects a Result".to_string()),
                },
            }),
        ),
        (
            "result.unwrap_err",
            Value::Builtin(BuiltinFn {
                name: "result.unwrap_err",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Err(e) => Ok((**e).clone()),
                    Value::Ok(_) => Err("called unwrap_err on Ok".to_string()),
                    _ => Err("result.unwrap_err expects a Result".to_string()),
                },
            }),
        ),
    ]
}
