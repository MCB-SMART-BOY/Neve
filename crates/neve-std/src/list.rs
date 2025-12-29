//! List operations for the standard library.

use neve_eval::value::{BuiltinFn, Value};
use std::rc::Rc;

pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // Basic operations
        (
            "list.empty",
            Value::Builtin(BuiltinFn {
                name: "list.empty",
                arity: 0,
                func: |_args| Ok(Value::List(Rc::new(Vec::new()))),
            }),
        ),
        (
            "list.singleton",
            Value::Builtin(BuiltinFn {
                name: "list.singleton",
                arity: 1,
                func: |args| Ok(Value::List(Rc::new(vec![args[0].clone()]))),
            }),
        ),
        (
            "list.len",
            Value::Builtin(BuiltinFn {
                name: "list.len",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => Ok(Value::Int(items.len() as i64)),
                    _ => Err("list.len expects a list".to_string()),
                },
            }),
        ),
        (
            "list.isEmpty",
            Value::Builtin(BuiltinFn {
                name: "list.isEmpty",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => Ok(Value::Bool(items.is_empty())),
                    _ => Err("list.isEmpty expects a list".to_string()),
                },
            }),
        ),
        // Access operations
        (
            "list.head",
            Value::Builtin(BuiltinFn {
                name: "list.head",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => Ok(items
                        .first()
                        .cloned()
                        .map(|v| Value::Some(Box::new(v)))
                        .unwrap_or(Value::None)),
                    _ => Err("list.head expects a list".to_string()),
                },
            }),
        ),
        (
            "list.tail",
            Value::Builtin(BuiltinFn {
                name: "list.tail",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => {
                        if items.is_empty() {
                            Ok(Value::List(Rc::new(Vec::new())))
                        } else {
                            Ok(Value::List(Rc::new(items[1..].to_vec())))
                        }
                    }
                    _ => Err("list.tail expects a list".to_string()),
                },
            }),
        ),
        (
            "list.last",
            Value::Builtin(BuiltinFn {
                name: "list.last",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => Ok(items
                        .last()
                        .cloned()
                        .map(|v| Value::Some(Box::new(v)))
                        .unwrap_or(Value::None)),
                    _ => Err("list.last expects a list".to_string()),
                },
            }),
        ),
        (
            "list.init",
            Value::Builtin(BuiltinFn {
                name: "list.init",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => {
                        if items.is_empty() {
                            Ok(Value::List(Rc::new(Vec::new())))
                        } else {
                            Ok(Value::List(Rc::new(items[..items.len() - 1].to_vec())))
                        }
                    }
                    _ => Err("list.init expects a list".to_string()),
                },
            }),
        ),
        (
            "list.get",
            Value::Builtin(BuiltinFn {
                name: "list.get",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Int(idx), Value::List(items)) => {
                        let idx = *idx as usize;
                        Ok(items
                            .get(idx)
                            .cloned()
                            .map(|v| Value::Some(Box::new(v)))
                            .unwrap_or(Value::None))
                    }
                    _ => Err("list.get expects (index, list)".to_string()),
                },
            }),
        ),
        // Modification operations
        (
            "list.cons",
            Value::Builtin(BuiltinFn {
                name: "list.cons",
                arity: 2,
                func: |args| match &args[1] {
                    Value::List(items) => {
                        let mut new_items = vec![args[0].clone()];
                        new_items.extend(items.iter().cloned());
                        Ok(Value::List(Rc::new(new_items)))
                    }
                    _ => Err("list.cons expects (element, list)".to_string()),
                },
            }),
        ),
        (
            "list.append",
            Value::Builtin(BuiltinFn {
                name: "list.append",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::List(a), Value::List(b)) => {
                        let mut new_items: Vec<_> = a.iter().cloned().collect();
                        new_items.extend(b.iter().cloned());
                        Ok(Value::List(Rc::new(new_items)))
                    }
                    _ => Err("list.append expects two lists".to_string()),
                },
            }),
        ),
        (
            "list.reverse",
            Value::Builtin(BuiltinFn {
                name: "list.reverse",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => {
                        let mut reversed: Vec<_> = items.iter().cloned().collect();
                        reversed.reverse();
                        Ok(Value::List(Rc::new(reversed)))
                    }
                    _ => Err("list.reverse expects a list".to_string()),
                },
            }),
        ),
        (
            "list.take",
            Value::Builtin(BuiltinFn {
                name: "list.take",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Int(n), Value::List(items)) => {
                        let n = (*n as usize).min(items.len());
                        Ok(Value::List(Rc::new(items[..n].to_vec())))
                    }
                    _ => Err("list.take expects (n, list)".to_string()),
                },
            }),
        ),
        (
            "list.drop",
            Value::Builtin(BuiltinFn {
                name: "list.drop",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Int(n), Value::List(items)) => {
                        let n = (*n as usize).min(items.len());
                        Ok(Value::List(Rc::new(items[n..].to_vec())))
                    }
                    _ => Err("list.drop expects (n, list)".to_string()),
                },
            }),
        ),
        // Higher-order functions (simplified - use evaluator for full closure support)
        (
            "list.map",
            Value::Builtin(BuiltinFn {
                name: "list.map",
                arity: 2,
                func: |_args| {
                    // Full implementation requires evaluator integration
                    Err("list.map requires runtime closure evaluation".to_string())
                },
            }),
        ),
        (
            "list.filter",
            Value::Builtin(BuiltinFn {
                name: "list.filter",
                arity: 2,
                func: |_args| Err("list.filter requires runtime closure evaluation".to_string()),
            }),
        ),
        (
            "list.fold",
            Value::Builtin(BuiltinFn {
                name: "list.fold",
                arity: 3,
                func: |_args| Err("list.fold requires runtime closure evaluation".to_string()),
            }),
        ),
        (
            "list.foldRight",
            Value::Builtin(BuiltinFn {
                name: "list.foldRight",
                arity: 3,
                func: |_args| Err("list.foldRight requires runtime closure evaluation".to_string()),
            }),
        ),
        // Aggregation
        (
            "list.sum",
            Value::Builtin(BuiltinFn {
                name: "list.sum",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => {
                        let mut sum = 0i64;
                        for item in items.iter() {
                            match item {
                                Value::Int(n) => sum += n,
                                _ => return Err("list.sum expects a list of integers".to_string()),
                            }
                        }
                        Ok(Value::Int(sum))
                    }
                    _ => Err("list.sum expects a list".to_string()),
                },
            }),
        ),
        (
            "list.product",
            Value::Builtin(BuiltinFn {
                name: "list.product",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => {
                        let mut product = 1i64;
                        for item in items.iter() {
                            match item {
                                Value::Int(n) => product *= n,
                                _ => {
                                    return Err(
                                        "list.product expects a list of integers".to_string()
                                    );
                                }
                            }
                        }
                        Ok(Value::Int(product))
                    }
                    _ => Err("list.product expects a list".to_string()),
                },
            }),
        ),
        (
            "list.max",
            Value::Builtin(BuiltinFn {
                name: "list.max",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => {
                        let mut max: Option<i64> = None;
                        for item in items.iter() {
                            match item {
                                Value::Int(n) => {
                                    max = Some(max.map_or(*n, |m| m.max(*n)));
                                }
                                _ => return Err("list.max expects a list of integers".to_string()),
                            }
                        }
                        Ok(max
                            .map(|m| Value::Some(Box::new(Value::Int(m))))
                            .unwrap_or(Value::None))
                    }
                    _ => Err("list.max expects a list".to_string()),
                },
            }),
        ),
        (
            "list.min",
            Value::Builtin(BuiltinFn {
                name: "list.min",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => {
                        let mut min: Option<i64> = None;
                        for item in items.iter() {
                            match item {
                                Value::Int(n) => {
                                    min = Some(min.map_or(*n, |m| m.min(*n)));
                                }
                                _ => return Err("list.min expects a list of integers".to_string()),
                            }
                        }
                        Ok(min
                            .map(|m| Value::Some(Box::new(Value::Int(m))))
                            .unwrap_or(Value::None))
                    }
                    _ => Err("list.min expects a list".to_string()),
                },
            }),
        ),
        // Search
        (
            "list.contains",
            Value::Builtin(BuiltinFn {
                name: "list.contains",
                arity: 2,
                func: |args| match &args[1] {
                    Value::List(items) => {
                        let found = items.iter().any(|item| values_equal(item, &args[0]));
                        Ok(Value::Bool(found))
                    }
                    _ => Err("list.contains expects (element, list)".to_string()),
                },
            }),
        ),
        (
            "list.indexOf",
            Value::Builtin(BuiltinFn {
                name: "list.indexOf",
                arity: 2,
                func: |args| match &args[1] {
                    Value::List(items) => {
                        for (i, item) in items.iter().enumerate() {
                            if values_equal(item, &args[0]) {
                                return Ok(Value::Some(Box::new(Value::Int(i as i64))));
                            }
                        }
                        Ok(Value::None)
                    }
                    _ => Err("list.indexOf expects (element, list)".to_string()),
                },
            }),
        ),
        // Sorting
        (
            "list.sort",
            Value::Builtin(BuiltinFn {
                name: "list.sort",
                arity: 1,
                func: |args| {
                    match &args[0] {
                        Value::List(items) => {
                            let mut sorted: Vec<_> = items.iter().cloned().collect();
                            // Only sort if all elements are comparable (integers for now)
                            sorted.sort_by(|a, b| match (a, b) {
                                (Value::Int(x), Value::Int(y)) => x.cmp(y),
                                (Value::String(x), Value::String(y)) => x.cmp(y),
                                _ => std::cmp::Ordering::Equal,
                            });
                            Ok(Value::List(Rc::new(sorted)))
                        }
                        _ => Err("list.sort expects a list".to_string()),
                    }
                },
            }),
        ),
        // Conversion
        (
            "list.range",
            Value::Builtin(BuiltinFn {
                name: "list.range",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Int(start), Value::Int(end)) => {
                        let items: Vec<Value> = (*start..*end).map(Value::Int).collect();
                        Ok(Value::List(Rc::new(items)))
                    }
                    _ => Err("list.range expects (start, end)".to_string()),
                },
            }),
        ),
        (
            "list.replicate",
            Value::Builtin(BuiltinFn {
                name: "list.replicate",
                arity: 2,
                func: |args| match &args[0] {
                    Value::Int(n) => {
                        let items: Vec<Value> = (0..*n as usize).map(|_| args[1].clone()).collect();
                        Ok(Value::List(Rc::new(items)))
                    }
                    _ => Err("list.replicate expects (n, value)".to_string()),
                },
            }),
        ),
        // Zipping
        (
            "list.zip",
            Value::Builtin(BuiltinFn {
                name: "list.zip",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::List(a), Value::List(b)) => {
                        let zipped: Vec<Value> = a
                            .iter()
                            .zip(b.iter())
                            .map(|(x, y)| Value::Tuple(Rc::new(vec![x.clone(), y.clone()])))
                            .collect();
                        Ok(Value::List(Rc::new(zipped)))
                    }
                    _ => Err("list.zip expects two lists".to_string()),
                },
            }),
        ),
        (
            "list.unzip",
            Value::Builtin(BuiltinFn {
                name: "list.unzip",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => {
                        let mut firsts = Vec::new();
                        let mut seconds = Vec::new();
                        for item in items.iter() {
                            match item {
                                Value::Tuple(pair) if pair.len() == 2 => {
                                    firsts.push(pair[0].clone());
                                    seconds.push(pair[1].clone());
                                }
                                _ => return Err("list.unzip expects a list of pairs".to_string()),
                            }
                        }
                        Ok(Value::Tuple(Rc::new(vec![
                            Value::List(Rc::new(firsts)),
                            Value::List(Rc::new(seconds)),
                        ])))
                    }
                    _ => Err("list.unzip expects a list".to_string()),
                },
            }),
        ),
    ]
}

/// Check if two values are equal (simplified comparison).
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => (x - y).abs() < f64::EPSILON,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Char(x), Value::Char(y)) => x == y,
        (Value::Unit, Value::Unit) => true,
        (Value::None, Value::None) => true,
        _ => false,
    }
}
