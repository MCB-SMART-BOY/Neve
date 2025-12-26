//! String operations for the standard library.

use neve_eval::value::{Value, BuiltinFn};
use std::rc::Rc;

pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        ("string.len", Value::Builtin(BuiltinFn {
            name: "string.len",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => Ok(Value::Int(s.len() as i64)),
                    _ => Err("string.len expects a string".to_string()),
                }
            },
        })),
        ("string.chars", Value::Builtin(BuiltinFn {
            name: "string.chars",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => {
                        let chars: Vec<Value> = s.chars().map(Value::Char).collect();
                        Ok(Value::List(Rc::new(chars)))
                    }
                    _ => Err("string.chars expects a string".to_string()),
                }
            },
        })),
        ("string.split", Value::Builtin(BuiltinFn {
            name: "string.split",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(sep)) => {
                        let parts: Vec<Value> = s
                            .split(sep.as_str())
                            .map(|p| Value::String(Rc::new(p.to_string())))
                            .collect();
                        Ok(Value::List(Rc::new(parts)))
                    }
                    _ => Err("string.split expects two strings".to_string()),
                }
            },
        })),
        ("string.join", Value::Builtin(BuiltinFn {
            name: "string.join",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
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
                }
            },
        })),
        ("string.trim", Value::Builtin(BuiltinFn {
            name: "string.trim",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => Ok(Value::String(Rc::new(s.trim().to_string()))),
                    _ => Err("string.trim expects a string".to_string()),
                }
            },
        })),
        ("string.upper", Value::Builtin(BuiltinFn {
            name: "string.upper",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => Ok(Value::String(Rc::new(s.to_uppercase()))),
                    _ => Err("string.upper expects a string".to_string()),
                }
            },
        })),
        ("string.lower", Value::Builtin(BuiltinFn {
            name: "string.lower",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => Ok(Value::String(Rc::new(s.to_lowercase()))),
                    _ => Err("string.lower expects a string".to_string()),
                }
            },
        })),
    ]
}
