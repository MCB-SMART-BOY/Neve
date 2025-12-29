//! String operations for the standard library.
//! 标准库的字符串操作。

use neve_eval::value::{BuiltinFn, Value};
use std::rc::Rc;

/// Returns all string builtins.
/// 返回所有字符串内置函数。
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // len : String -> Int
        // Returns the length of a string / 返回字符串长度
        (
            "string.len",
            Value::Builtin(BuiltinFn {
                name: "string.len",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => Ok(Value::Int(s.len() as i64)),
                    _ => Err("string.len expects a string".to_string()),
                },
            }),
        ),
        // chars : String -> List Char
        // Converts string to list of characters / 将字符串转换为字符列表
        (
            "string.chars",
            Value::Builtin(BuiltinFn {
                name: "string.chars",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => {
                        let chars: Vec<Value> = s.chars().map(Value::Char).collect();
                        Ok(Value::List(Rc::new(chars)))
                    }
                    _ => Err("string.chars expects a string".to_string()),
                },
            }),
        ),
        // split : String -> String -> List String
        // Splits string by separator / 按分隔符分割字符串
        (
            "string.split",
            Value::Builtin(BuiltinFn {
                name: "string.split",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(sep)) => {
                        let parts: Vec<Value> = s
                            .split(sep.as_str())
                            .map(|p| Value::String(Rc::new(p.to_string())))
                            .collect();
                        Ok(Value::List(Rc::new(parts)))
                    }
                    _ => Err("string.split expects two strings".to_string()),
                },
            }),
        ),
        // join : List String -> String -> String
        // Joins strings with separator / 用分隔符连接字符串
        (
            "string.join",
            Value::Builtin(BuiltinFn {
                name: "string.join",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::List(items), Value::String(sep)) => {
                        let strings: Result<Vec<_>, _> = items
                            .iter()
                            .map(|v| match v {
                                Value::String(s) => Ok(s.as_str().to_string()),
                                _ => Err("string.join expects a list of strings"),
                            })
                            .collect();
                        strings
                            .map(|ss| Value::String(Rc::new(ss.join(sep.as_str()))))
                            .map_err(|e| e.to_string())
                    }
                    _ => Err("string.join expects a list and a string".to_string()),
                },
            }),
        ),
        // trim : String -> String
        // Removes leading and trailing whitespace / 移除首尾空白字符
        (
            "string.trim",
            Value::Builtin(BuiltinFn {
                name: "string.trim",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => Ok(Value::String(Rc::new(s.trim().to_string()))),
                    _ => Err("string.trim expects a string".to_string()),
                },
            }),
        ),
        // upper : String -> String
        // Converts to uppercase / 转换为大写
        (
            "string.upper",
            Value::Builtin(BuiltinFn {
                name: "string.upper",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => Ok(Value::String(Rc::new(s.to_uppercase()))),
                    _ => Err("string.upper expects a string".to_string()),
                },
            }),
        ),
        // lower : String -> String
        // Converts to lowercase / 转换为小写
        (
            "string.lower",
            Value::Builtin(BuiltinFn {
                name: "string.lower",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => Ok(Value::String(Rc::new(s.to_lowercase()))),
                    _ => Err("string.lower expects a string".to_string()),
                },
            }),
        ),
        // contains : String -> String -> Bool
        // Checks if string contains substring / 检查字符串是否包含子串
        (
            "string.contains",
            Value::Builtin(BuiltinFn {
                name: "string.contains",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(haystack), Value::String(needle)) => {
                        Ok(Value::Bool(haystack.contains(needle.as_str())))
                    }
                    _ => Err("string.contains expects two strings".to_string()),
                },
            }),
        ),
        // startsWith : String -> String -> Bool
        // Checks if string starts with prefix / 检查字符串是否以前缀开头
        (
            "string.startsWith",
            Value::Builtin(BuiltinFn {
                name: "string.startsWith",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(prefix)) => {
                        Ok(Value::Bool(s.starts_with(prefix.as_str())))
                    }
                    _ => Err("string.startsWith expects two strings".to_string()),
                },
            }),
        ),
        // endsWith : String -> String -> Bool
        // Checks if string ends with suffix / 检查字符串是否以后缀结尾
        (
            "string.endsWith",
            Value::Builtin(BuiltinFn {
                name: "string.endsWith",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(suffix)) => {
                        Ok(Value::Bool(s.ends_with(suffix.as_str())))
                    }
                    _ => Err("string.endsWith expects two strings".to_string()),
                },
            }),
        ),
        // replace : String -> String -> String -> String
        // Replaces occurrences of from with to / 将 from 替换为 to
        (
            "string.replace",
            Value::Builtin(BuiltinFn {
                name: "string.replace",
                arity: 3,
                func: |args| match (&args[0], &args[1], &args[2]) {
                    (Value::String(s), Value::String(from), Value::String(to)) => Ok(
                        Value::String(Rc::new(s.replace(from.as_str(), to.as_str()))),
                    ),
                    _ => Err("string.replace expects three strings".to_string()),
                },
            }),
        ),
        // substring : String -> Int -> Int -> String
        // Extracts substring from start to end / 提取从 start 到 end 的子串
        (
            "string.substring",
            Value::Builtin(BuiltinFn {
                name: "string.substring",
                arity: 3,
                func: |args| match (&args[0], &args[1], &args[2]) {
                    (Value::String(s), Value::Int(start), Value::Int(end)) => {
                        let start = (*start as usize).min(s.len());
                        let end = (*end as usize).min(s.len());
                        if start <= end {
                            Ok(Value::String(Rc::new(s[start..end].to_string())))
                        } else {
                            Ok(Value::String(Rc::new(String::new())))
                        }
                    }
                    _ => Err("string.substring expects (string, start, end)".to_string()),
                },
            }),
        ),
        // isEmpty : String -> Bool
        // Checks if string is empty / 检查字符串是否为空
        (
            "string.isEmpty",
            Value::Builtin(BuiltinFn {
                name: "string.isEmpty",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => Ok(Value::Bool(s.is_empty())),
                    _ => Err("string.isEmpty expects a string".to_string()),
                },
            }),
        ),
        // repeat : String -> Int -> String
        // Repeats string n times / 将字符串重复 n 次
        (
            "string.repeat",
            Value::Builtin(BuiltinFn {
                name: "string.repeat",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(s), Value::Int(n)) => {
                        Ok(Value::String(Rc::new(s.repeat(*n as usize))))
                    }
                    _ => Err("string.repeat expects (string, count)".to_string()),
                },
            }),
        ),
        // lines : String -> List String
        // Splits string into lines / 将字符串分割成行
        (
            "string.lines",
            Value::Builtin(BuiltinFn {
                name: "string.lines",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => {
                        let lines: Vec<Value> = s
                            .lines()
                            .map(|l| Value::String(Rc::new(l.to_string())))
                            .collect();
                        Ok(Value::List(Rc::new(lines)))
                    }
                    _ => Err("string.lines expects a string".to_string()),
                },
            }),
        ),
    ]
}
