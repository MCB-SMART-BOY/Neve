//! List operations for the standard library.
//! 标准库的列表操作。

use neve_common::{Int, int_is_negative, int_to_i64, int_to_usize};
use neve_eval::value::{BuiltinFn, Value};
use std::rc::Rc;

/// Returns all list builtins.
/// 返回所有列表内置函数。
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // Basic operations / 基本操作
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
                    Value::List(items) => Ok(Value::Int(items.len().into())),
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
        // Access operations / 访问操作
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
                        let idx = index_to_usize(idx);
                        Ok(idx
                            .and_then(|i| items.get(i).cloned())
                            .map(|v| Value::Some(Box::new(v)))
                            .unwrap_or(Value::None))
                    }
                    _ => Err("list.get expects (index, list)".to_string()),
                },
            }),
        ),
        // Modification operations / 修改操作
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
                        let n = clamp_len(n, items.len());
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
                        let n = clamp_len(n, items.len());
                        Ok(Value::List(Rc::new(items[n..].to_vec())))
                    }
                    _ => Err("list.drop expects (n, list)".to_string()),
                },
            }),
        ),
        // Higher-order functions (simplified - use evaluator for full closure support)
        // 高阶函数（简化版 - 完整闭包支持需要求值器）
        (
            "list.map",
            Value::Builtin(BuiltinFn {
                name: "list.map",
                arity: 2,
                func: |_args| {
                    // Full implementation requires evaluator integration
                    // 完整实现需要求值器集成
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
        // Aggregation / 聚合
        (
            "list.sum",
            Value::Builtin(BuiltinFn {
                name: "list.sum",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(items) => {
                        let mut sum = Int::from(0);
                        for item in items.iter() {
                            match item {
                                Value::Int(n) => sum += n.clone(),
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
                        let mut product = Int::from(1);
                        for item in items.iter() {
                            match item {
                                Value::Int(n) => product *= n.clone(),
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
                        let mut max: Option<Int> = None;
                        for item in items.iter() {
                            match item {
                                Value::Int(n) => {
                                    max = Some(match max {
                                        Some(ref m) if m >= n => m.clone(),
                                        _ => n.clone(),
                                    });
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
                        let mut min: Option<Int> = None;
                        for item in items.iter() {
                            match item {
                                Value::Int(n) => {
                                    min = Some(match min {
                                        Some(ref m) if m <= n => m.clone(),
                                        _ => n.clone(),
                                    });
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
        // Search / 搜索
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
                                return Ok(Value::Some(Box::new(Value::Int(i.into()))));
                            }
                        }
                        Ok(Value::None)
                    }
                    _ => Err("list.indexOf expects (element, list)".to_string()),
                },
            }),
        ),
        // Sorting / 排序
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
                            // 仅在所有元素可比较时排序（当前支持整数、字符串与 Path）
                            sorted.sort_by(|a, b| match (a, b) {
                                (Value::Int(x), Value::Int(y)) => x.cmp(y),
                                (Value::String(x), Value::String(y)) => x.cmp(y),
                                (Value::Path(x), Value::Path(y)) => {
                                    x.as_os_str().cmp(y.as_os_str())
                                }
                                _ => std::cmp::Ordering::Equal,
                            });
                            Ok(Value::List(Rc::new(sorted)))
                        }
                        _ => Err("list.sort expects a list".to_string()),
                    }
                },
            }),
        ),
        // Conversion / 转换
        (
            "list.range",
            Value::Builtin(BuiltinFn {
                name: "list.range",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Int(start), Value::Int(end)) => {
                        let start = require_i64(start, "list.range start")?;
                        let end = require_i64(end, "list.range end")?;
                        let items: Vec<Value> =
                            (start..end).map(|n| Value::Int(n.into())).collect();
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
                        let count = clamp_non_negative_usize(n, "list.replicate count")?;
                        let items: Vec<Value> = (0..count).map(|_| args[1].clone()).collect();
                        Ok(Value::List(Rc::new(items)))
                    }
                    _ => Err("list.replicate expects (n, value)".to_string()),
                },
            }),
        ),
        // Zipping / 压缩
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

fn index_to_usize(value: &Int) -> Option<usize> {
    if int_is_negative(value) {
        None
    } else {
        int_to_usize(value)
    }
}

fn clamp_len(value: &Int, len: usize) -> usize {
    if int_is_negative(value) {
        0
    } else {
        int_to_usize(value).unwrap_or(len).min(len)
    }
}

fn clamp_non_negative_usize(value: &Int, context: &str) -> Result<usize, String> {
    if int_is_negative(value) {
        return Ok(0);
    }
    int_to_usize(value).ok_or_else(|| format!("{context} is too large"))
}

fn require_i64(value: &Int, context: &str) -> Result<i64, String> {
    int_to_i64(value).ok_or_else(|| format!("{context} is out of range"))
}

/// Check if two values are equal (simplified comparison).
/// 检查两个值是否相等（简化比较）。
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
