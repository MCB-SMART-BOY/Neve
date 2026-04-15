//! Path operations for the standard library.
//! 标准库的路径操作。

use neve_eval::value::{BuiltinFn, Value};
use std::path::PathBuf;
use std::rc::Rc;

/// Returns all path builtins.
/// 返回所有路径内置函数。
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // fromString : String -> Path
        // Creates a structured Path runtime object / 创建结构化 Path 运行时对象
        (
            "path.fromString",
            Value::Builtin(BuiltinFn {
                name: "path.fromString",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => Ok(Value::Path(Rc::new(PathBuf::from(path.as_str())))),
                    _ => Err("path.fromString expects a string".to_string()),
                },
            }),
        ),
        // joinPath : Path -> String -> Path
        // Joins a structured path with a child component / 将结构化路径与子路径片段连接
        (
            "path.joinPath",
            Value::Builtin(BuiltinFn {
                name: "path.joinPath",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Path(base), Value::String(child)) => {
                        Ok(Value::Path(Rc::new(base.join(child.as_str()))))
                    }
                    _ => Err("path.joinPath expects (Path, String)".to_string()),
                },
            }),
        ),
        // parentPath : Path -> Option Path
        // Gets the parent of a structured path / 获取结构化路径的父路径
        (
            "path.parentPath",
            Value::Builtin(BuiltinFn {
                name: "path.parentPath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => match path.parent() {
                        Some(parent) => Ok(Value::Some(Box::new(Value::Path(Rc::new(
                            parent.to_path_buf(),
                        ))))),
                        None => Ok(Value::None),
                    },
                    _ => Err("path.parentPath expects a Path".to_string()),
                },
            }),
        ),
        // filenamePath : Path -> Option String
        // Gets the file name component from a structured path / 从结构化路径获取文件名组件
        (
            "path.filenamePath",
            Value::Builtin(BuiltinFn {
                name: "path.filenamePath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => match path.file_name() {
                        Some(name) => Ok(Value::Some(Box::new(Value::String(Rc::new(
                            name.to_string_lossy().to_string(),
                        ))))),
                        None => Ok(Value::None),
                    },
                    _ => Err("path.filenamePath expects a Path".to_string()),
                },
            }),
        ),
        // extensionPath : Path -> Option String
        // Gets the file extension from a structured path / 从结构化路径获取文件扩展名
        (
            "path.extensionPath",
            Value::Builtin(BuiltinFn {
                name: "path.extensionPath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => match path.extension() {
                        Some(ext) => Ok(Value::Some(Box::new(Value::String(Rc::new(
                            ext.to_string_lossy().to_string(),
                        ))))),
                        None => Ok(Value::None),
                    },
                    _ => Err("path.extensionPath expects a Path".to_string()),
                },
            }),
        ),
        // isAbsolutePath : Path -> Bool
        // Checks whether a structured path is absolute / 检查结构化路径是否为绝对路径
        (
            "path.isAbsolutePath",
            Value::Builtin(BuiltinFn {
                name: "path.isAbsolutePath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => Ok(Value::Bool(path.is_absolute())),
                    _ => Err("path.isAbsolutePath expects a Path".to_string()),
                },
            }),
        ),
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
