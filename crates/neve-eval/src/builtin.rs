//! Built-in functions.

use crate::value::{Value, BuiltinFn};
use neve_derive::Derivation;
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
                        .map_err(|_| format!("cannot convert '{s}' to Int")),
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
                        .map_err(|_| format!("cannot convert '{s}' to Float")),
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
                    Value::Err(e) => Err(format!("unwrap called on Err: {e:?}")),
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
                    Value::Thunk(_) => "Thunk",
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
                    Err(format!("assertion failed: {:?} != {:?}", &args[0], &args[1]))
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
        
        // === Type predicates ===
        ("isInt", Value::Builtin(BuiltinFn {
            name: "isInt",
            arity: 1,
            func: |args| Ok(Value::Bool(matches!(&args[0], Value::Int(_)))),
        })),
        ("isFloat", Value::Builtin(BuiltinFn {
            name: "isFloat",
            arity: 1,
            func: |args| Ok(Value::Bool(matches!(&args[0], Value::Float(_)))),
        })),
        ("isBool", Value::Builtin(BuiltinFn {
            name: "isBool",
            arity: 1,
            func: |args| Ok(Value::Bool(matches!(&args[0], Value::Bool(_)))),
        })),
        ("isString", Value::Builtin(BuiltinFn {
            name: "isString",
            arity: 1,
            func: |args| Ok(Value::Bool(matches!(&args[0], Value::String(_)))),
        })),
        ("isList", Value::Builtin(BuiltinFn {
            name: "isList",
            arity: 1,
            func: |args| Ok(Value::Bool(matches!(&args[0], Value::List(_)))),
        })),
        ("isRecord", Value::Builtin(BuiltinFn {
            name: "isRecord",
            arity: 1,
            func: |args| Ok(Value::Bool(matches!(&args[0], Value::Record(_)))),
        })),
        ("isFunction", Value::Builtin(BuiltinFn {
            name: "isFunction",
            arity: 1,
            func: |args| Ok(Value::Bool(matches!(&args[0], 
                Value::Closure { .. } | Value::AstClosure(_) | Value::Builtin(_) | Value::BuiltinFn(_, _)))),
        })),
        ("isLazy", Value::Builtin(BuiltinFn {
            name: "isLazy",
            arity: 1,
            func: |args| Ok(Value::Bool(matches!(&args[0], Value::Thunk(_)))),
        })),
        
        // === Lazy evaluation ===
        // `force` forces evaluation of a thunk. The actual implementation is in
        // AstEvaluator::apply which intercepts calls to this builtin and handles
        // them specially since they need evaluator access.
        ("force", Value::Builtin(BuiltinFn {
            name: "force",
            arity: 1,
            func: |args| {
                // This is a fallback for when force is called outside of AstEvaluator.
                // The real implementation is in AstEvaluator::force_value.
                match &args[0] {
                    Value::Thunk(thunk) => {
                        // If already evaluated, return the cached value
                        use crate::value::ThunkState;
                        match &*thunk.state() {
                            ThunkState::Evaluated(v) => Ok(v.clone()),
                            _ => Err("cannot force unevaluated thunk in this context".to_string()),
                        }
                    }
                    other => Ok(other.clone()), // Non-thunks are returned as-is
                }
            },
        })),
        ("isEvaluated", Value::Builtin(BuiltinFn {
            name: "isEvaluated",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Thunk(thunk) => Ok(Value::Bool(thunk.is_evaluated())),
                    _ => Ok(Value::Bool(true)), // Non-thunks are always "evaluated"
                }
            },
        })),
        
        // === Derivation helpers ===
        ("derivation", Value::Builtin(BuiltinFn {
            name: "derivation",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Record(attrs) => {
                        // Extract required fields
                        let name = match attrs.get("name") {
                            Some(Value::String(s)) => s.to_string(),
                            Some(_) => return Err("derivation 'name' must be a string".to_string()),
                            None => return Err("derivation requires 'name' field".to_string()),
                        };
                        
                        let builder = match attrs.get("builder") {
                            Some(Value::String(s)) => s.to_string(),
                            Some(_) => return Err("derivation 'builder' must be a string".to_string()),
                            None => return Err("derivation requires 'builder' field".to_string()),
                        };
                        
                        let system = match attrs.get("system") {
                            Some(Value::String(s)) => s.to_string(),
                            Some(_) => return Err("derivation 'system' must be a string".to_string()),
                            None => return Err("derivation requires 'system' field".to_string()),
                        };
                        
                        // Extract optional version
                        let version = match attrs.get("version") {
                            Some(Value::String(s)) => s.to_string(),
                            _ => "0.0.0".to_string(),
                        };
                        
                        // Extract args if present
                        let args_list = match attrs.get("args") {
                            Some(Value::List(items)) => {
                                items.iter().filter_map(|v| {
                                    if let Value::String(s) = v {
                                        Some(s.to_string())
                                    } else {
                                        None
                                    }
                                }).collect()
                            }
                            _ => Vec::new(),
                        };
                        
                        // Build the derivation using neve-derive
                        let mut drv_builder = Derivation::builder(&name, &version)
                            .system(&system)
                            .builder_path(&builder);
                        
                        for arg in &args_list {
                            drv_builder = drv_builder.arg(arg);
                        }
                        
                        // Add environment variables from attrs (excluding special fields)
                        let special_fields = ["name", "version", "system", "builder", "args", "outputs"];
                        for (key, value) in attrs.iter() {
                            if !special_fields.contains(&key.as_str()) {
                                if let Value::String(s) = value {
                                    drv_builder = drv_builder.env(key, s.as_str());
                                }
                            }
                        }
                        
                        let drv = drv_builder.build();
                        
                        // Get computed paths
                        let drv_path = drv.drv_path();
                        let out_path = drv.out_path()
                            .map(|p| p.to_string())
                            .unwrap_or_else(|| format!("/neve/store/{}-{}", drv.hash(), name));
                        
                        // Return the derivation as a record with computed fields
                        let mut result = (**attrs).clone();
                        result.insert("type".to_string(), Value::String(Rc::new("derivation".to_string())));
                        result.insert("drvPath".to_string(), Value::String(Rc::new(drv_path.to_string())));
                        result.insert("outPath".to_string(), Value::String(Rc::new(out_path.clone())));
                        result.insert("out".to_string(), Value::String(Rc::new(out_path)));
                        
                        Ok(Value::Record(Rc::new(result)))
                    }
                    _ => Err("derivation expects a record".to_string()),
                }
            },
        })),
        
        // === Sequence/error handling ===
        ("seq", Value::Builtin(BuiltinFn {
            name: "seq",
            arity: 2,
            func: |args| {
                // seq forces evaluation of first arg, returns second
                let _ = &args[0]; // Force evaluation
                Ok(args[1].clone())
            },
        })),
        ("deepSeq", Value::Builtin(BuiltinFn {
            name: "deepSeq",
            arity: 2,
            func: |args| {
                // deepSeq forces deep evaluation of first arg, returns second
                fn force_deep(v: &Value) {
                    match v {
                        Value::List(items) => items.iter().for_each(force_deep),
                        Value::Record(fields) => fields.values().for_each(force_deep),
                        Value::Tuple(items) => items.iter().for_each(force_deep),
                        _ => {}
                    }
                }
                force_deep(&args[0]);
                Ok(args[1].clone())
            },
        })),
        ("throw", Value::Builtin(BuiltinFn {
            name: "throw",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(msg) => Err(msg.to_string()),
                    _ => Err(format!("{:?}", args[0])),
                }
            },
        })),
        
        // === JSON-like operations ===
        ("toJSON", Value::Builtin(BuiltinFn {
            name: "toJSON",
            arity: 1,
            func: |args| {
                Ok(Value::String(Rc::new(value_to_json(&args[0]))))
            },
        })),
        ("fromJSON", Value::Builtin(BuiltinFn {
            name: "fromJSON",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => json_to_value(s.as_str()),
                    _ => Err("fromJSON expects a string".to_string()),
                }
            },
        })),
        
        // === List higher-order operations (non-lazy versions) ===
        ("concat", Value::Builtin(BuiltinFn {
            name: "concat",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(lists) => {
                        let mut result = Vec::new();
                        for item in lists.iter() {
                            match item {
                                Value::List(inner) => result.extend(inner.iter().cloned()),
                                _ => return Err("concat expects a list of lists".to_string()),
                            }
                        }
                        Ok(Value::List(Rc::new(result)))
                    }
                    _ => Err("concat expects a list of lists".to_string()),
                }
            },
        })),
        ("flatten", Value::Builtin(BuiltinFn {
            name: "flatten",
            arity: 1,
            func: |args| {
                fn flatten_recursive(v: &Value, result: &mut Vec<Value>) {
                    match v {
                        Value::List(items) => {
                            for item in items.iter() {
                                flatten_recursive(item, result);
                            }
                        }
                        other => result.push(other.clone()),
                    }
                }
                let mut result = Vec::new();
                flatten_recursive(&args[0], &mut result);
                Ok(Value::List(Rc::new(result)))
            },
        })),
        ("unique", Value::Builtin(BuiltinFn {
            name: "unique",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(items) => {
                        let mut seen = Vec::new();
                        for item in items.iter() {
                            if !seen.contains(item) {
                                seen.push(item.clone());
                            }
                        }
                        Ok(Value::List(Rc::new(seen)))
                    }
                    _ => Err("unique expects a list".to_string()),
                }
            },
        })),
        // === Higher-order functions ===
        // These are stub definitions - actual implementation is in AstEvaluator::apply
        // which intercepts calls to these builtins and evaluates them with evaluator access.
        ("map", Value::Builtin(BuiltinFn {
            name: "map",
            arity: 2,
            func: |_| Err("map requires evaluator context".to_string()),
        })),
        ("filter", Value::Builtin(BuiltinFn {
            name: "filter",
            arity: 2,
            func: |_| Err("filter requires evaluator context".to_string()),
        })),
        ("all", Value::Builtin(BuiltinFn {
            name: "all",
            arity: 2,
            func: |_| Err("all requires evaluator context".to_string()),
        })),
        ("any", Value::Builtin(BuiltinFn {
            name: "any",
            arity: 2,
            func: |_| Err("any requires evaluator context".to_string()),
        })),
        ("foldl", Value::Builtin(BuiltinFn {
            name: "foldl",
            arity: 3,
            func: |_| Err("foldl requires evaluator context".to_string()),
        })),
        ("foldr", Value::Builtin(BuiltinFn {
            name: "foldr",
            arity: 3,
            func: |_| Err("foldr requires evaluator context".to_string()),
        })),
        ("genList", Value::Builtin(BuiltinFn {
            name: "genList",
            arity: 2,
            func: |_| Err("genList requires evaluator context".to_string()),
        })),
        ("mapAttrs", Value::Builtin(BuiltinFn {
            name: "mapAttrs",
            arity: 2,
            func: |_| Err("mapAttrs requires evaluator context".to_string()),
        })),
        ("filterAttrs", Value::Builtin(BuiltinFn {
            name: "filterAttrs",
            arity: 2,
            func: |_| Err("filterAttrs requires evaluator context".to_string()),
        })),
        ("concatMap", Value::Builtin(BuiltinFn {
            name: "concatMap",
            arity: 2,
            func: |_| Err("concatMap requires evaluator context".to_string()),
        })),
        ("partition", Value::Builtin(BuiltinFn {
            name: "partition",
            arity: 2,
            func: |_| Err("partition requires evaluator context".to_string()),
        })),
        ("groupBy", Value::Builtin(BuiltinFn {
            name: "groupBy",
            arity: 2,
            func: |_| Err("groupBy requires evaluator context".to_string()),
        })),
        ("sort", Value::Builtin(BuiltinFn {
            name: "sort",
            arity: 2,
            func: |_| Err("sort requires evaluator context".to_string()),
        })),
        
        // === Bitwise operations ===
        ("bitAnd", Value::Builtin(BuiltinFn {
            name: "bitAnd",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a & b)),
                    _ => Err("bitAnd expects two integers".to_string()),
                }
            },
        })),
        ("bitOr", Value::Builtin(BuiltinFn {
            name: "bitOr",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a | b)),
                    _ => Err("bitOr expects two integers".to_string()),
                }
            },
        })),
        ("bitXor", Value::Builtin(BuiltinFn {
            name: "bitXor",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a ^ b)),
                    _ => Err("bitXor expects two integers".to_string()),
                }
            },
        })),
        ("bitNot", Value::Builtin(BuiltinFn {
            name: "bitNot",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Int(a) => Ok(Value::Int(!a)),
                    _ => Err("bitNot expects an integer".to_string()),
                }
            },
        })),
        ("bitShiftLeft", Value::Builtin(BuiltinFn {
            name: "bitShiftLeft",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a << b)),
                    _ => Err("bitShiftLeft expects two integers".to_string()),
                }
            },
        })),
        ("bitShiftRight", Value::Builtin(BuiltinFn {
            name: "bitShiftRight",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a >> b)),
                    _ => Err("bitShiftRight expects two integers".to_string()),
                }
            },
        })),
        
        // === String formatting ===
        ("padLeft", Value::Builtin(BuiltinFn {
            name: "padLeft",
            arity: 3,
            func: |args| {
                match (&args[0], &args[1], &args[2]) {
                    (Value::Int(width), Value::String(pad), Value::String(s)) => {
                        let width = *width as usize;
                        if s.len() >= width {
                            Ok(Value::String(s.clone()))
                        } else {
                            let pad_char = pad.chars().next().unwrap_or(' ');
                            let padding: String = std::iter::repeat_n(pad_char, width - s.len()).collect();
                            Ok(Value::String(Rc::new(format!("{}{}", padding, s))))
                        }
                    }
                    _ => Err("padLeft expects (Int, String, String)".to_string()),
                }
            },
        })),
        ("padRight", Value::Builtin(BuiltinFn {
            name: "padRight",
            arity: 3,
            func: |args| {
                match (&args[0], &args[1], &args[2]) {
                    (Value::Int(width), Value::String(pad), Value::String(s)) => {
                        let width = *width as usize;
                        if s.len() >= width {
                            Ok(Value::String(s.clone()))
                        } else {
                            let pad_char = pad.chars().next().unwrap_or(' ');
                            let padding: String = std::iter::repeat_n(pad_char, width - s.len()).collect();
                            Ok(Value::String(Rc::new(format!("{}{}", s, padding))))
                        }
                    }
                    _ => Err("padRight expects (Int, String, String)".to_string()),
                }
            },
        })),
        
        // === Comparison ===
        ("compare", Value::Builtin(BuiltinFn {
            name: "compare",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(match a.cmp(b) {
                        std::cmp::Ordering::Less => -1,
                        std::cmp::Ordering::Equal => 0,
                        std::cmp::Ordering::Greater => 1,
                    })),
                    (Value::Float(a), Value::Float(b)) => {
                        Ok(Value::Int(if a < b { -1 } else if a > b { 1 } else { 0 }))
                    }
                    (Value::String(a), Value::String(b)) => Ok(Value::Int(match a.cmp(b) {
                        std::cmp::Ordering::Less => -1,
                        std::cmp::Ordering::Equal => 0,
                        std::cmp::Ordering::Greater => 1,
                    })),
                    _ => Err("compare expects two comparable values of same type".to_string()),
                }
            },
        })),
        
        // === Record merging ===
        ("merge", Value::Builtin(BuiltinFn {
            name: "merge",
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Record(a), Value::Record(b)) => {
                        let mut result = (**a).clone();
                        result.extend((**b).clone());
                        Ok(Value::Record(Rc::new(result)))
                    }
                    _ => Err("merge expects two records".to_string()),
                }
            },
        })),
        ("mergeRecursive", Value::Builtin(BuiltinFn {
            name: "mergeRecursive",
            arity: 2,
            func: |args| {
                fn merge_deep(a: &Value, b: &Value) -> Value {
                    match (a, b) {
                        (Value::Record(ra), Value::Record(rb)) => {
                            let mut result = (**ra).clone();
                            for (k, v) in rb.iter() {
                                if let Some(existing) = result.get(k) {
                                    result.insert(k.clone(), merge_deep(existing, v));
                                } else {
                                    result.insert(k.clone(), v.clone());
                                }
                            }
                            Value::Record(Rc::new(result))
                        }
                        (_, b) => b.clone(),
                    }
                }
                Ok(merge_deep(&args[0], &args[1]))
            },
        })),
    ]
}

/// Parse JSON string to value.
fn json_to_value(s: &str) -> Result<Value, String> {
    let s = s.trim();
    
    if s.is_empty() {
        return Err("empty JSON string".to_string());
    }
    
    // Parse based on first character
    match s.chars().next().unwrap() {
        'n' if s == "null" => Ok(Value::None),
        't' if s == "true" => Ok(Value::Bool(true)),
        'f' if s == "false" => Ok(Value::Bool(false)),
        '"' => {
            // Parse string
            if s.len() < 2 || !s.ends_with('"') {
                return Err("invalid JSON string".to_string());
            }
            let inner = &s[1..s.len()-1];
            // Handle escape sequences
            let mut result = String::new();
            let mut chars = inner.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() {
                        Some('n') => result.push('\n'),
                        Some('r') => result.push('\r'),
                        Some('t') => result.push('\t'),
                        Some('\\') => result.push('\\'),
                        Some('"') => result.push('"'),
                        Some('/') => result.push('/'),
                        Some('u') => {
                            // Unicode escape
                            let hex: String = chars.by_ref().take(4).collect();
                            if let Ok(code) = u32::from_str_radix(&hex, 16)
                                && let Some(c) = char::from_u32(code)
                            {
                                result.push(c);
                            }
                        }
                        _ => return Err("invalid escape sequence".to_string()),
                    }
                } else {
                    result.push(c);
                }
            }
            Ok(Value::String(Rc::new(result)))
        }
        '[' => {
            // Parse array
            if !s.ends_with(']') {
                return Err("invalid JSON array".to_string());
            }
            let inner = s[1..s.len()-1].trim();
            if inner.is_empty() {
                return Ok(Value::List(Rc::new(Vec::new())));
            }
            
            let elements = split_json_elements(inner)?;
            let values: Result<Vec<_>, _> = elements.iter().map(|e| json_to_value(e)).collect();
            Ok(Value::List(Rc::new(values?)))
        }
        '{' => {
            // Parse object
            if !s.ends_with('}') {
                return Err("invalid JSON object".to_string());
            }
            let inner = s[1..s.len()-1].trim();
            if inner.is_empty() {
                return Ok(Value::Record(Rc::new(std::collections::HashMap::new())));
            }
            
            let pairs = split_json_elements(inner)?;
            let mut record = std::collections::HashMap::new();
            for pair in pairs {
                let pair = pair.trim();
                if let Some(colon_pos) = find_json_colon(pair) {
                    let key = pair[..colon_pos].trim();
                    let value = pair[colon_pos+1..].trim();
                    
                    // Key must be a string
                    if !key.starts_with('"') || !key.ends_with('"') {
                        return Err("JSON object keys must be strings".to_string());
                    }
                    let key_str = &key[1..key.len()-1];
                    let value = json_to_value(value)?;
                    record.insert(key_str.to_string(), value);
                } else {
                    return Err("invalid JSON object pair".to_string());
                }
            }
            Ok(Value::Record(Rc::new(record)))
        }
        c if c == '-' || c.is_ascii_digit() => {
            // Parse number
            if s.contains('.') || s.contains('e') || s.contains('E') {
                s.parse::<f64>()
                    .map(Value::Float)
                    .map_err(|_| "invalid JSON number".to_string())
            } else {
                s.parse::<i64>()
                    .map(Value::Int)
                    .map_err(|_| "invalid JSON number".to_string())
            }
        }
        _ => Err(format!("unexpected JSON token: {}", s)),
    }
}

/// Split JSON array/object elements respecting nesting.
fn split_json_elements(s: &str) -> Result<Vec<&str>, String> {
    let mut elements = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    
    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        
        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '[' | '{' if !in_string => depth += 1,
            ']' | '}' if !in_string => depth -= 1,
            ',' if !in_string && depth == 0 => {
                elements.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    
    if start < s.len() {
        elements.push(s[start..].trim());
    }
    
    Ok(elements)
}

/// Find the colon in a JSON key-value pair respecting strings.
fn find_json_colon(s: &str) -> Option<usize> {
    let mut in_string = false;
    let mut escape = false;
    
    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        
        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            ':' if !in_string => return Some(i),
            _ => {}
        }
    }
    
    None
}

/// Convert a value to JSON string.
fn value_to_json(v: &Value) -> String {
    match v {
        Value::Unit => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => {
            if f.is_nan() {
                "null".to_string()
            } else if f.is_infinite() {
                if *f > 0.0 { "1e309".to_string() } else { "-1e309".to_string() }
            } else {
                f.to_string()
            }
        }
        Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")),
        Value::List(items) => {
            let parts: Vec<String> = items.iter().map(value_to_json).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Record(fields) => {
            let parts: Vec<String> = fields.iter()
                .map(|(k, v)| format!("\"{}\":{}", k, value_to_json(v)))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::None => "null".to_string(),
        Value::Some(v) => value_to_json(v),
        _ => "null".to_string(),
    }
}

/// Format a value for display (user-friendly, not debug).
pub fn format_value(v: &Value) -> String {
    match v {
        Value::Unit => "()".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => {
            if f.fract() == 0.0 {
                format!("{f}.0")
            } else {
                f.to_string()
            }
        }
        Value::Char(c) => format!("'{c}'"),
        Value::String(s) => format!("\"{s}\""),
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
        Value::BuiltinFn(name, _) => format!("<builtin:{name}>"),
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
        Value::Thunk(thunk) => {
            use crate::value::ThunkState;
            match &*thunk.state() {
                ThunkState::Evaluated(v) => format_value(v),
                ThunkState::Evaluating => "<thunk:evaluating>".to_string(),
                ThunkState::Unevaluated { .. } => "<thunk>".to_string(),
            }
        }
    }
}

/// Extension trait for functional pipe-style chaining.
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}
