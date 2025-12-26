//! Path operations for the standard library.

use neve_eval::value::{Value, BuiltinFn};
use std::rc::Rc;

pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        ("path.join", Value::Builtin(BuiltinFn {
            name: "path.join",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::String(a), Value::String(b)) => {
                        let path = std::path::Path::new(a.as_str()).join(b.as_str());
                        Ok(Value::String(Rc::new(path.to_string_lossy().to_string())))
                    }
                    _ => Err("path.join expects two strings".to_string()),
                }
            },
        })),
        ("path.parent", Value::Builtin(BuiltinFn {
            name: "path.parent",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => {
                        let path = std::path::Path::new(s.as_str());
                        match path.parent() {
                            Some(p) => Ok(Value::Some(Box::new(Value::String(
                                Rc::new(p.to_string_lossy().to_string()),
                            )))),
                            None => Ok(Value::None),
                        }
                    }
                    _ => Err("path.parent expects a string".to_string()),
                }
            },
        })),
        ("path.filename", Value::Builtin(BuiltinFn {
            name: "path.filename",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => {
                        let path = std::path::Path::new(s.as_str());
                        match path.file_name() {
                            Some(name) => Ok(Value::Some(Box::new(Value::String(
                                Rc::new(name.to_string_lossy().to_string()),
                            )))),
                            None => Ok(Value::None),
                        }
                    }
                    _ => Err("path.filename expects a string".to_string()),
                }
            },
        })),
        ("path.extension", Value::Builtin(BuiltinFn {
            name: "path.extension",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => {
                        let path = std::path::Path::new(s.as_str());
                        match path.extension() {
                            Some(ext) => Ok(Value::Some(Box::new(Value::String(
                                Rc::new(ext.to_string_lossy().to_string()),
                            )))),
                            None => Ok(Value::None),
                        }
                    }
                    _ => Err("path.extension expects a string".to_string()),
                }
            },
        })),
        ("path.is_absolute", Value::Builtin(BuiltinFn {
            name: "path.is_absolute",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => {
                        let path = std::path::Path::new(s.as_str());
                        Ok(Value::Bool(path.is_absolute()))
                    }
                    _ => Err("path.is_absolute expects a string".to_string()),
                }
            },
        })),
    ]
}
