//! Path operations for the standard library.
//! 标准库的路径操作。

use neve_eval::value::{BuiltinFn, Value};
use std::rc::Rc;

/// Returns all path builtins.
/// 返回所有路径内置函数。
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // join : String -> String -> String
        // Joins two path components / 连接两个路径组件
        (
            "path.join",
            Value::Builtin(BuiltinFn {
                name: "path.join",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(a), Value::String(b)) => {
                        let path = std::path::Path::new(a.as_str()).join(b.as_str());
                        Ok(Value::String(Rc::new(path.to_string_lossy().to_string())))
                    }
                    _ => Err("path.join expects two strings".to_string()),
                },
            }),
        ),
        // parent : String -> Option String
        // Gets the parent directory / 获取父目录
        (
            "path.parent",
            Value::Builtin(BuiltinFn {
                name: "path.parent",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => {
                        let path = std::path::Path::new(s.as_str());
                        match path.parent() {
                            Some(p) => Ok(Value::Some(Box::new(Value::String(Rc::new(
                                p.to_string_lossy().to_string(),
                            ))))),
                            None => Ok(Value::None),
                        }
                    }
                    _ => Err("path.parent expects a string".to_string()),
                },
            }),
        ),
        // filename : String -> Option String
        // Gets the file name component / 获取文件名组件
        (
            "path.filename",
            Value::Builtin(BuiltinFn {
                name: "path.filename",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => {
                        let path = std::path::Path::new(s.as_str());
                        match path.file_name() {
                            Some(name) => Ok(Value::Some(Box::new(Value::String(Rc::new(
                                name.to_string_lossy().to_string(),
                            ))))),
                            None => Ok(Value::None),
                        }
                    }
                    _ => Err("path.filename expects a string".to_string()),
                },
            }),
        ),
        // extension : String -> Option String
        // Gets the file extension / 获取文件扩展名
        (
            "path.extension",
            Value::Builtin(BuiltinFn {
                name: "path.extension",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => {
                        let path = std::path::Path::new(s.as_str());
                        match path.extension() {
                            Some(ext) => Ok(Value::Some(Box::new(Value::String(Rc::new(
                                ext.to_string_lossy().to_string(),
                            ))))),
                            None => Ok(Value::None),
                        }
                    }
                    _ => Err("path.extension expects a string".to_string()),
                },
            }),
        ),
        // is_absolute : String -> Bool
        // Checks if path is absolute / 检查路径是否为绝对路径
        (
            "path.is_absolute",
            Value::Builtin(BuiltinFn {
                name: "path.is_absolute",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => {
                        let path = std::path::Path::new(s.as_str());
                        Ok(Value::Bool(path.is_absolute()))
                    }
                    _ => Err("path.is_absolute expects a string".to_string()),
                },
            }),
        ),
    ]
}
