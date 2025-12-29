//! Result operations for the standard library.
//! 标准库的 Result 操作。

use neve_eval::value::{BuiltinFn, Value};

/// Returns all result builtins.
/// 返回所有 Result 内置函数。
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // ok : a -> Result a e
        // Wraps a value in Ok / 将值包装为 Ok
        (
            "result.ok",
            Value::Builtin(BuiltinFn {
                name: "result.ok",
                arity: 1,
                func: |args| Ok(Value::Ok(Box::new(args[0].clone()))),
            }),
        ),
        // err : e -> Result a e
        // Wraps a value in Err / 将值包装为 Err
        (
            "result.err",
            Value::Builtin(BuiltinFn {
                name: "result.err",
                arity: 1,
                func: |args| Ok(Value::Err(Box::new(args[0].clone()))),
            }),
        ),
        // is_ok : Result a e -> Bool
        // Checks if result is Ok / 检查 result 是否为 Ok
        (
            "result.is_ok",
            Value::Builtin(BuiltinFn {
                name: "result.is_ok",
                arity: 1,
                func: |args| Ok(Value::Bool(matches!(args[0], Value::Ok(_)))),
            }),
        ),
        // is_err : Result a e -> Bool
        // Checks if result is Err / 检查 result 是否为 Err
        (
            "result.is_err",
            Value::Builtin(BuiltinFn {
                name: "result.is_err",
                arity: 1,
                func: |args| Ok(Value::Bool(matches!(args[0], Value::Err(_)))),
            }),
        ),
        // unwrap : Result a e -> a
        // Extracts the Ok value, panics if Err / 提取 Ok 值，如果为 Err 则报错
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
        // unwrap_err : Result a e -> e
        // Extracts the Err value, panics if Ok / 提取 Err 值，如果为 Ok 则报错
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
