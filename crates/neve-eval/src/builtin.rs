//! Built-in functions.

use crate::value::{Value, BuiltinFn};
use std::rc::Rc;

/// Get all built-in functions.
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // === I/O ===
        ("print", Value::Builtin(BuiltinFn {
            name: "print",
            arity: 1,
            func: |args| {
                println!("{}", format_value(&args[0]));
                Ok(Value::Unit)
            },
        })),

        // === Type conversion ===
        ("toString", Value::Builtin(BuiltinFn {
            name: "toString",
            arity: 1,
            func: |args| {
                Ok(Value::String(Rc::new(format_value(&args[0]))))
            },
        })),
        ("toInt", Value::Builtin(BuiltinFn {
            name: "toInt",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Int(n) => Ok(Value::Int(*n)),
                    Value::Float(f) => Ok(Value::Int(*f as i64)),
                    Value::String(s) => s.parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| format!("cannot convert '{}' to Int", s)),
                    Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                    _ => Err("toInt expects Int, Float, String, or Bool".to_string()),
                }
            },
        })),
        ("toFloat", Value::Builtin(BuiltinFn {
            name: "toFloat",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Int(n) => Ok(Value::Float(*n as f64)),
                    Value::Float(f) => Ok(Value::Float(*f)),
                    Value::String(s) => s.parse::<f64>()
                        .map(Value::Float)
                        .map_err(|_| format!("cannot convert '{}' to Float", s)),
                    _ => Err("toFloat expects Int, Float, or String".to_string()),
                }
            },
        })),

        // === List operations ===
        ("len", Value::Builtin(BuiltinFn {
            name: "len",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(items) => Ok(Value::Int(items.len() as i64)),
                    Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
                    Value::Record(fields) => Ok(Value::Int(fields.len() as i64)),
                    _ => Err("len expects a list, string, or record".to_string()),
                }
            },
        })),
        ("head", Value::Builtin(BuiltinFn {
            name: "head",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(items) => {
                        items.first().cloned().map(|v| Value::Some(Box::new(v))).unwrap_or(Value::None).pipe(Ok)
                    }
                    _ => Err("head expects a list".to_string()),
                }
            },
        })),
        ("tail", Value::Builtin(BuiltinFn {
            name: "tail",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(items) => {
                        if items.is_empty() {
                            Ok(Value::List(Rc::new(Vec::new())))
                        } else {
                            Ok(Value::List(Rc::new(items[1..].to_vec())))
                        }
                    }
                    _ => Err("tail expects a list".to_string()),
                }
            },
        })),
        ("last", Value::Builtin(BuiltinFn {
            name: "last",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(items) => {
                        items.last().cloned().map(|v| Value::Some(Box::new(v))).unwrap_or(Value::None).pipe(Ok)
                    }
                    _ => Err("last expects a list".to_string()),
                }
            },
        })),
        ("init", Value::Builtin(BuiltinFn {
            name: "init",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(items) => {
                        if items.is_empty() {
                            Ok(Value::List(Rc::new(Vec::new())))
                        } else {
                            Ok(Value::List(Rc::new(items[..items.len()-1].to_vec())))
                        }
                    }
                    _ => Err("init expects a list".to_string()),
                }
            },
        })),
        ("reverse", Value::Builtin(BuiltinFn {
            name: "reverse",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(items) => {
                        let mut rev: Vec<_> = (**items).clone();
                        rev.reverse();
                        Ok(Value::List(Rc::new(rev)))
                    }
                    Value::String(s) => {
                        Ok(Value::String(Rc::new(s.chars().rev().collect())))
                    }
                    _ => Err("reverse expects a list or string".to_string()),
                }
            },
        })),
        ("isEmpty", Value::Builtin(BuiltinFn {
            name: "isEmpty",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(items) => Ok(Value::Bool(items.is_empty())),
                    Value::String(s) => Ok(Value::Bool(s.is_empty())),
                    Value::Record(fields) => Ok(Value::Bool(fields.is_empty())),
                    _ => Err("isEmpty expects a list, string, or record".to_string()),
                }
            },
        })),
        ("elem", Value::Builtin(BuiltinFn {
            name: "elem",
            arity: 2,
            func: |args| {
                match &args[1] {
                    Value::List(items) => Ok(Value::Bool(items.contains(&args[0]))),
                    _ => Err("elem expects (element, list)".to_string()),
                }
            },
        })),
        ("take", Value::Builtin(BuiltinFn {
            name: "take",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Int(n), Value::List(items)) => {
                        let n = (*n).max(0) as usize;
                        Ok(Value::List(Rc::new(items.iter().take(n).cloned().collect())))
                    }
                    _ => Err("take expects (Int, List)".to_string()),
                }
            },
        })),
        ("drop", Value::Builtin(BuiltinFn {
            name: "drop",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Int(n), Value::List(items)) => {
                        let n = (*n).max(0) as usize;
                        Ok(Value::List(Rc::new(items.iter().skip(n).cloned().collect())))
                    }
                    _ => Err("drop expects (Int, List)".to_string()),
                }
            },
        })),
        ("range", Value::Builtin(BuiltinFn {
            name: "range",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Int(start), Value::Int(end)) => {
                        let items: Vec<Value> = (*start..*end).map(Value::Int).collect();
                        Ok(Value::List(Rc::new(items)))
                    }
                    _ => Err("range expects (Int, Int)".to_string()),
                }
            },
        })),
        ("replicate", Value::Builtin(BuiltinFn {
            name: "replicate",
            arity: 2,
            func: |args| {
                match &args[0] {
                    Value::Int(n) => {
                        let n = (*n).max(0) as usize;
                        let items: Vec<Value> = std::iter::repeat_n(args[1].clone(), n).collect();
                        Ok(Value::List(Rc::new(items)))
                    }
                    _ => Err("replicate expects (Int, a)".to_string()),
                }
            },
        })),

        // === String operations ===
        ("chars", Value::Builtin(BuiltinFn {
            name: "chars",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => {
                        let chars: Vec<Value> = s.chars().map(Value::Char).collect();
                        Ok(Value::List(Rc::new(chars)))
                    }
                    _ => Err("chars expects a String".to_string()),
                }
            },
        })),
        ("words", Value::Builtin(BuiltinFn {
            name: "words",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => {
                        let words: Vec<Value> = s.split_whitespace()
                            .map(|w| Value::String(Rc::new(w.to_string())))
                            .collect();
                        Ok(Value::List(Rc::new(words)))
                    }
                    _ => Err("words expects a String".to_string()),
                }
            },
        })),
        ("lines", Value::Builtin(BuiltinFn {
            name: "lines",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => {
                        let lines: Vec<Value> = s.lines()
                            .map(|l| Value::String(Rc::new(l.to_string())))
                            .collect();
                        Ok(Value::List(Rc::new(lines)))
                    }
                    _ => Err("lines expects a String".to_string()),
                }
            },
        })),
        ("unwords", Value::Builtin(BuiltinFn {
            name: "unwords",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(items) => {
                        let words: Result<Vec<_>, _> = items.iter().map(|v| {
                            if let Value::String(s) = v {
                                Ok(s.as_str())
                            } else {
                                Err("unwords expects a list of strings".to_string())
                            }
                        }).collect();
                        Ok(Value::String(Rc::new(words?.join(" "))))
                    }
                    _ => Err("unwords expects a list of strings".to_string()),
                }
            },
        })),
        ("unlines", Value::Builtin(BuiltinFn {
            name: "unlines",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(items) => {
                        let lines: Result<Vec<_>, _> = items.iter().map(|v| {
                            if let Value::String(s) = v {
                                Ok(s.as_str())
                            } else {
                                Err("unlines expects a list of strings".to_string())
                            }
                        }).collect();
                        Ok(Value::String(Rc::new(lines?.join("\n"))))
                    }
                    _ => Err("unlines expects a list of strings".to_string()),
                }
            },
        })),
        ("trim", Value::Builtin(BuiltinFn {
            name: "trim",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => Ok(Value::String(Rc::new(s.trim().to_string()))),
                    _ => Err("trim expects a String".to_string()),
                }
            },
        })),
        ("split", Value::Builtin(BuiltinFn {
            name: "split",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::String(sep), Value::String(s)) => {
                        let parts: Vec<Value> = s.split(sep.as_str())
                            .map(|p| Value::String(Rc::new(p.to_string())))
                            .collect();
                        Ok(Value::List(Rc::new(parts)))
                    }
                    _ => Err("split expects (String, String)".to_string()),
                }
            },
        })),
        ("join", Value::Builtin(BuiltinFn {
            name: "join",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::String(sep), Value::List(items)) => {
                        let strings: Result<Vec<_>, _> = items.iter().map(|v| {
                            if let Value::String(s) = v {
                                Ok(s.as_str())
                            } else {
                                Err("join expects a list of strings".to_string())
                            }
                        }).collect();
                        Ok(Value::String(Rc::new(strings?.join(sep.as_str()))))
                    }
                    _ => Err("join expects (String, [String])".to_string()),
                }
            },
        })),
        ("uppercase", Value::Builtin(BuiltinFn {
            name: "uppercase",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => Ok(Value::String(Rc::new(s.to_uppercase()))),
                    _ => Err("uppercase expects a String".to_string()),
                }
            },
        })),
        ("lowercase", Value::Builtin(BuiltinFn {
            name: "lowercase",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => Ok(Value::String(Rc::new(s.to_lowercase()))),
                    _ => Err("lowercase expects a String".to_string()),
                }
            },
        })),
        ("startsWith", Value::Builtin(BuiltinFn {
            name: "startsWith",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::String(prefix), Value::String(s)) => {
                        Ok(Value::Bool(s.starts_with(prefix.as_str())))
                    }
                    _ => Err("startsWith expects (String, String)".to_string()),
                }
            },
        })),
        ("endsWith", Value::Builtin(BuiltinFn {
            name: "endsWith",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::String(suffix), Value::String(s)) => {
                        Ok(Value::Bool(s.ends_with(suffix.as_str())))
                    }
                    _ => Err("endsWith expects (String, String)".to_string()),
                }
            },
        })),
        ("contains", Value::Builtin(BuiltinFn {
            name: "contains",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::String(needle), Value::String(haystack)) => {
                        Ok(Value::Bool(haystack.contains(needle.as_str())))
                    }
                    _ => Err("contains expects (String, String)".to_string()),
                }
            },
        })),
        ("replace", Value::Builtin(BuiltinFn {
            name: "replace",
            arity: 3,
            func: |args| {
                match (&args[0], &args[1], &args[2]) {
                    (Value::String(from), Value::String(to), Value::String(s)) => {
                        Ok(Value::String(Rc::new(s.replace(from.as_str(), to.as_str()))))
                    }
                    _ => Err("replace expects (String, String, String)".to_string()),
                }
            },
        })),

        // === Math ===
        ("abs", Value::Builtin(BuiltinFn {
            name: "abs",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Int(n) => Ok(Value::Int(n.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err("abs expects Int or Float".to_string()),
                }
            },
        })),
        ("min", Value::Builtin(BuiltinFn {
            name: "min",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.min(b))),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
                    _ => Err("min expects two Ints or two Floats".to_string()),
                }
            },
        })),
        ("max", Value::Builtin(BuiltinFn {
            name: "max",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.max(b))),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
                    _ => Err("max expects two Ints or two Floats".to_string()),
                }
            },
        })),
        ("floor", Value::Builtin(BuiltinFn {
            name: "floor",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Float(f) => Ok(Value::Int(f.floor() as i64)),
                    Value::Int(n) => Ok(Value::Int(*n)),
                    _ => Err("floor expects Float or Int".to_string()),
                }
            },
        })),
        ("ceil", Value::Builtin(BuiltinFn {
            name: "ceil",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Float(f) => Ok(Value::Int(f.ceil() as i64)),
                    Value::Int(n) => Ok(Value::Int(*n)),
                    _ => Err("ceil expects Float or Int".to_string()),
                }
            },
        })),
        ("round", Value::Builtin(BuiltinFn {
            name: "round",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Float(f) => Ok(Value::Int(f.round() as i64)),
                    Value::Int(n) => Ok(Value::Int(*n)),
                    _ => Err("round expects Float or Int".to_string()),
                }
            },
        })),
        ("sqrt", Value::Builtin(BuiltinFn {
            name: "sqrt",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Float(f) => Ok(Value::Float(f.sqrt())),
                    Value::Int(n) => Ok(Value::Float((*n as f64).sqrt())),
                    _ => Err("sqrt expects Float or Int".to_string()),
                }
            },
        })),
        ("pow", Value::Builtin(BuiltinFn {
            name: "pow",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Int(base), Value::Int(exp)) => {
                        if *exp >= 0 {
                            Ok(Value::Int(base.pow(*exp as u32)))
                        } else {
                            Ok(Value::Float((*base as f64).powi(*exp as i32)))
                        }
                    }
                    (Value::Float(base), Value::Float(exp)) => Ok(Value::Float(base.powf(*exp))),
                    (Value::Int(base), Value::Float(exp)) => Ok(Value::Float((*base as f64).powf(*exp))),
                    (Value::Float(base), Value::Int(exp)) => Ok(Value::Float(base.powi(*exp as i32))),
                    _ => Err("pow expects numeric arguments".to_string()),
                }
            },
        })),

        // === Option/Result helpers ===
        ("isSome", Value::Builtin(BuiltinFn {
            name: "isSome",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Some(_) => Ok(Value::Bool(true)),
                    Value::None => Ok(Value::Bool(false)),
                    _ => Err("isSome expects an Option".to_string()),
                }
            },
        })),
        ("isNone", Value::Builtin(BuiltinFn {
            name: "isNone",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Some(_) => Ok(Value::Bool(false)),
                    Value::None => Ok(Value::Bool(true)),
                    _ => Err("isNone expects an Option".to_string()),
                }
            },
        })),
        ("unwrap", Value::Builtin(BuiltinFn {
            name: "unwrap",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Some(v) => Ok((**v).clone()),
                    Value::None => Err("unwrap called on None".to_string()),
                    Value::Ok(v) => Ok((**v).clone()),
                    Value::Err(e) => Err(format!("unwrap called on Err: {:?}", e)),
                    _ => Err("unwrap expects Option or Result".to_string()),
                }
            },
        })),
        ("unwrapOr", Value::Builtin(BuiltinFn {
            name: "unwrapOr",
            arity: 2,
            func: |args| {
                match &args[0] {
                    Value::Some(v) => Ok((**v).clone()),
                    Value::None => Ok(args[1].clone()),
                    Value::Ok(v) => Ok((**v).clone()),
                    Value::Err(_) => Ok(args[1].clone()),
                    _ => Err("unwrapOr expects Option or Result".to_string()),
                }
            },
        })),
        ("isOk", Value::Builtin(BuiltinFn {
            name: "isOk",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Ok(_) => Ok(Value::Bool(true)),
                    Value::Err(_) => Ok(Value::Bool(false)),
                    _ => Err("isOk expects a Result".to_string()),
                }
            },
        })),
        ("isErr", Value::Builtin(BuiltinFn {
            name: "isErr",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Ok(_) => Ok(Value::Bool(false)),
                    Value::Err(_) => Ok(Value::Bool(true)),
                    _ => Err("isErr expects a Result".to_string()),
                }
            },
        })),

        // === Record operations ===
        ("keys", Value::Builtin(BuiltinFn {
            name: "keys",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Record(fields) => {
                        let keys: Vec<Value> = fields.keys()
                            .map(|k| Value::String(Rc::new(k.clone())))
                            .collect();
                        Ok(Value::List(Rc::new(keys)))
                    }
                    _ => Err("keys expects a record".to_string()),
                }
            },
        })),
        ("values", Value::Builtin(BuiltinFn {
            name: "values",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Record(fields) => {
                        let values: Vec<Value> = fields.values().cloned().collect();
                        Ok(Value::List(Rc::new(values)))
                    }
                    _ => Err("values expects a record".to_string()),
                }
            },
        })),
        ("hasField", Value::Builtin(BuiltinFn {
            name: "hasField",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::String(key), Value::Record(fields)) => {
                        Ok(Value::Bool(fields.contains_key(key.as_str())))
                    }
                    _ => Err("hasField expects (String, Record)".to_string()),
                }
            },
        })),
        ("getField", Value::Builtin(BuiltinFn {
            name: "getField",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::String(key), Value::Record(fields)) => {
                        match fields.get(key.as_str()) {
                            Some(v) => Ok(Value::Some(Box::new(v.clone()))),
                            None => Ok(Value::None),
                        }
                    }
                    _ => Err("getField expects (String, Record)".to_string()),
                }
            },
        })),
        ("setField", Value::Builtin(BuiltinFn {
            name: "setField",
            arity: 3,
            func: |args| {
                match (&args[0], &args[2]) {
                    (Value::String(key), Value::Record(fields)) => {
                        let mut new_fields = (**fields).clone();
                        new_fields.insert(key.to_string(), args[1].clone());
                        Ok(Value::Record(Rc::new(new_fields)))
                    }
                    _ => Err("setField expects (String, value, Record)".to_string()),
                }
            },
        })),
        ("removeField", Value::Builtin(BuiltinFn {
            name: "removeField",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::String(key), Value::Record(fields)) => {
                        let mut new_fields = (**fields).clone();
                        new_fields.remove(key.as_str());
                        Ok(Value::Record(Rc::new(new_fields)))
                    }
                    _ => Err("removeField expects (String, Record)".to_string()),
                }
            },
        })),

        // === Type checking ===
        ("typeOf", Value::Builtin(BuiltinFn {
            name: "typeOf",
            arity: 1,
            func: |args| {
                let type_name = match &args[0] {
                    Value::Unit => "Unit",
                    Value::Bool(_) => "Bool",
                    Value::Int(_) => "Int",
                    Value::Float(_) => "Float",
                    Value::Char(_) => "Char",
                    Value::String(_) => "String",
                    Value::List(_) => "List",
                    Value::Tuple(_) => "Tuple",
                    Value::Record(_) => "Record",
                    Value::Map(_) => "Map",
                    Value::Set(_) => "Set",
                    Value::Closure { .. } => "Function",
                    Value::AstClosure(_) => "Function",
                    Value::Builtin(_) => "Function",
                    Value::BuiltinFn(_, _) => "Function",
                    Value::Variant(tag, _) => tag.as_str(),
                    Value::Some(_) => "Some",
                    Value::None => "None",
                    Value::Ok(_) => "Ok",
                    Value::Err(_) => "Err",
                };
                Ok(Value::String(Rc::new(type_name.to_string())))
            },
        })),

        // === Assertion/debugging ===
        ("assert", Value::Builtin(BuiltinFn {
            name: "assert",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Bool(true) => Ok(Value::Unit),
                    Value::Bool(false) => Err("assertion failed".to_string()),
                    _ => Err("assert expects a Bool".to_string()),
                }
            },
        })),
        ("assertEq", Value::Builtin(BuiltinFn {
            name: "assertEq",
            arity: 2,
            func: |args| {
                if args[0] == args[1] {
                    Ok(Value::Unit)
                } else {
                    Err(format!("assertion failed: {:?} != {:?}", args[0], args[1]))
                }
            },
        })),
        ("trace", Value::Builtin(BuiltinFn {
            name: "trace",
            arity: 2,
            func: |args| {
                eprintln!("trace: {}", format_value(&args[0]));
                Ok(args[1].clone())
            },
        })),

        // === Identity ===
        ("id", Value::Builtin(BuiltinFn {
            name: "id",
            arity: 1,
            func: |args| {
                Ok(args[0].clone())
            },
        })),
        ("const", Value::Builtin(BuiltinFn {
            name: "const",
            arity: 2,
            func: |args| {
                Ok(args[0].clone())
            },
        })),
    ]
}

/// Format a value for display (user-friendly, not debug).
pub fn format_value(v: &Value) -> String {
    match v {
        Value::Unit => "()".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}.0", f)
            } else {
                f.to_string()
            }
        }
        Value::Char(c) => format!("'{}'", c),
        Value::String(s) => format!("\"{}\"", s),
        Value::List(items) => {
            let parts: Vec<String> = items.iter().map(format_value).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Tuple(items) => {
            let parts: Vec<String> = items.iter().map(format_value).collect();
            format!("({})", parts.join(", "))
        }
        Value::Record(fields) => {
            let parts: Vec<String> = fields.iter()
                .map(|(k, v)| format!("{} = {}", k, format_value(v)))
                .collect();
            format!("#{{ {} }}", parts.join(", "))
        }
        Value::Map(map) => {
            let parts: Vec<String> = map.iter()
                .map(|(k, v)| format!("{} => {}", k, format_value(v)))
                .collect();
            format!("Map{{ {} }}", parts.join(", "))
        }
        Value::Set(set) => {
            let parts: Vec<String> = set.iter().cloned().collect();
            format!("Set{{ {} }}", parts.join(", "))
        }
        Value::Closure { .. } => "<function>".to_string(),
        Value::AstClosure(_) => "<function>".to_string(),
        Value::Builtin(f) => format!("<builtin:{}>", f.name),
        Value::BuiltinFn(name, _) => format!("<builtin:{}>", name),
        Value::Variant(tag, payload) => {
            if matches!(**payload, Value::Unit) {
                tag.clone()
            } else {
                format!("{}({})", tag, format_value(payload))
            }
        }
        Value::Some(v) => format!("Some({})", format_value(v)),
        Value::None => "None".to_string(),
        Value::Ok(v) => format!("Ok({})", format_value(v)),
        Value::Err(v) => format!("Err({})", format_value(v)),
    }
}

#[allow(dead_code)]
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}
