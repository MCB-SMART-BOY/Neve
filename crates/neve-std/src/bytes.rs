//! Bytes operations for the standard library.
//! 标准库的字节操作。

use neve_common::Int;
use neve_eval::value::{BuiltinFn, Value};
use std::rc::Rc;

/// Returns all bytes builtins.
/// 返回所有字节内置函数。
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // len : Bytes -> Int
        (
            "bytes.len",
            Value::Builtin(BuiltinFn {
                name: "bytes.len",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Bytes(b) => Ok(Value::Int(b.len().into())),
                    _ => Err("bytes.len expects Bytes".to_string()),
                },
            }),
        ),
        // isEmpty : Bytes -> Bool
        (
            "bytes.isEmpty",
            Value::Builtin(BuiltinFn {
                name: "bytes.isEmpty",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Bytes(b) => Ok(Value::Bool(b.is_empty())),
                    _ => Err("bytes.isEmpty expects Bytes".to_string()),
                },
            }),
        ),
        // concat : Bytes -> Bytes -> Bytes
        (
            "bytes.concat",
            Value::Builtin(BuiltinFn {
                name: "bytes.concat",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Bytes(a), Value::Bytes(b)) => {
                        let mut result = Vec::with_capacity(a.len() + b.len());
                        result.extend_from_slice(a);
                        result.extend_from_slice(b);
                        Ok(Value::Bytes(Rc::new(result)))
                    }
                    _ => Err("bytes.concat expects two Bytes".to_string()),
                },
            }),
        ),
        // fromString : String -> Bytes
        (
            "bytes.fromString",
            Value::Builtin(BuiltinFn {
                name: "bytes.fromString",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => {
                        Ok(Value::Bytes(Rc::new(s.as_bytes().to_vec())))
                    }
                    _ => Err("bytes.fromString expects a String".to_string()),
                },
            }),
        ),
        // toString : Bytes -> String (UTF-8)
        (
            "bytes.toString",
            Value::Builtin(BuiltinFn {
                name: "bytes.toString",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Bytes(b) => {
                        match String::from_utf8(b.to_vec()) {
                            Ok(s) => Ok(Value::String(Rc::new(s))),
                            Err(e) => Err(format!("bytes.toString: invalid UTF-8: {e}")),
                        }
                    }
                    _ => Err("bytes.toString expects Bytes".to_string()),
                },
            }),
        ),
        // toList : Bytes -> List<Int> (byte values 0-255)
        (
            "bytes.toList",
            Value::Builtin(BuiltinFn {
                name: "bytes.toList",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Bytes(b) => {
                        let ints: Vec<Value> = b.iter()
                            .map(|byte| Value::Int((*byte as i64).into()))
                            .collect();
                        Ok(Value::List(Rc::new(ints)))
                    }
                    _ => Err("bytes.toList expects Bytes".to_string()),
                },
            }),
        ),
        // fromList : List<Int> -> Bytes
        (
            "bytes.fromList",
            Value::Builtin(BuiltinFn {
                name: "bytes.fromList",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => {
                        let mut bytes = Vec::with_capacity(items.len());
                        for item in items.iter() {
                            match item {
                                Value::Int(n) => {
                                    let byte: u8 = n.clone().try_into()
                                        .map_err(|_| "bytes.fromList: value out of byte range (0-255)".to_string())?;
                                    bytes.push(byte);
                                }
                                _ => return Err("bytes.fromList expects List<Int>".to_string()),
                            }
                        }
                        Ok(Value::Bytes(Rc::new(bytes)))
                    }
                    _ => Err("bytes.fromList expects a List".to_string()),
                },
            }),
        ),
    ]
}
