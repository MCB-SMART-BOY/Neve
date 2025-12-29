//! Math operations for the standard library.
//! 标准库的数学操作。

use neve_eval::value::{BuiltinFn, Value};

/// Returns all math builtins.
/// 返回所有数学内置函数。
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // Basic math / 基本数学运算
        (
            "math.abs",
            Value::Builtin(BuiltinFn {
                name: "math.abs",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Int(n) => Ok(Value::Int(n.abs())),
                    Value::Float(n) => Ok(Value::Float(n.abs())),
                    _ => Err("math.abs expects a number".to_string()),
                },
            }),
        ),
        (
            "math.floor",
            Value::Builtin(BuiltinFn {
                name: "math.floor",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => Ok(Value::Int(n.floor() as i64)),
                    Value::Int(n) => Ok(Value::Int(*n)),
                    _ => Err("math.floor expects a number".to_string()),
                },
            }),
        ),
        (
            "math.ceil",
            Value::Builtin(BuiltinFn {
                name: "math.ceil",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => Ok(Value::Int(n.ceil() as i64)),
                    Value::Int(n) => Ok(Value::Int(*n)),
                    _ => Err("math.ceil expects a number".to_string()),
                },
            }),
        ),
        (
            "math.round",
            Value::Builtin(BuiltinFn {
                name: "math.round",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => Ok(Value::Int(n.round() as i64)),
                    Value::Int(n) => Ok(Value::Int(*n)),
                    _ => Err("math.round expects a number".to_string()),
                },
            }),
        ),
        (
            "math.sqrt",
            Value::Builtin(BuiltinFn {
                name: "math.sqrt",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => Ok(Value::Float(n.sqrt())),
                    Value::Int(n) => Ok(Value::Float((*n as f64).sqrt())),
                    _ => Err("math.sqrt expects a number".to_string()),
                },
            }),
        ),
        (
            "math.pow",
            Value::Builtin(BuiltinFn {
                name: "math.pow",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Int(base), Value::Int(exp)) => {
                        if *exp >= 0 {
                            Ok(Value::Int(base.pow(*exp as u32)))
                        } else {
                            Ok(Value::Float((*base as f64).powi(*exp as i32)))
                        }
                    }
                    (Value::Float(base), Value::Int(exp)) => {
                        Ok(Value::Float(base.powi(*exp as i32)))
                    }
                    (Value::Float(base), Value::Float(exp)) => Ok(Value::Float(base.powf(*exp))),
                    (Value::Int(base), Value::Float(exp)) => {
                        Ok(Value::Float((*base as f64).powf(*exp)))
                    }
                    _ => Err("math.pow expects two numbers".to_string()),
                },
            }),
        ),
        (
            "math.log",
            Value::Builtin(BuiltinFn {
                name: "math.log",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => Ok(Value::Float(n.ln())),
                    Value::Int(n) => Ok(Value::Float((*n as f64).ln())),
                    _ => Err("math.log expects a number".to_string()),
                },
            }),
        ),
        (
            "math.log10",
            Value::Builtin(BuiltinFn {
                name: "math.log10",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => Ok(Value::Float(n.log10())),
                    Value::Int(n) => Ok(Value::Float((*n as f64).log10())),
                    _ => Err("math.log10 expects a number".to_string()),
                },
            }),
        ),
        (
            "math.exp",
            Value::Builtin(BuiltinFn {
                name: "math.exp",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => Ok(Value::Float(n.exp())),
                    Value::Int(n) => Ok(Value::Float((*n as f64).exp())),
                    _ => Err("math.exp expects a number".to_string()),
                },
            }),
        ),
        // Trigonometry / 三角函数
        (
            "math.sin",
            Value::Builtin(BuiltinFn {
                name: "math.sin",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => Ok(Value::Float(n.sin())),
                    Value::Int(n) => Ok(Value::Float((*n as f64).sin())),
                    _ => Err("math.sin expects a number".to_string()),
                },
            }),
        ),
        (
            "math.cos",
            Value::Builtin(BuiltinFn {
                name: "math.cos",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => Ok(Value::Float(n.cos())),
                    Value::Int(n) => Ok(Value::Float((*n as f64).cos())),
                    _ => Err("math.cos expects a number".to_string()),
                },
            }),
        ),
        (
            "math.tan",
            Value::Builtin(BuiltinFn {
                name: "math.tan",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => Ok(Value::Float(n.tan())),
                    Value::Int(n) => Ok(Value::Float((*n as f64).tan())),
                    _ => Err("math.tan expects a number".to_string()),
                },
            }),
        ),
        // Comparison / 比较
        (
            "math.max",
            Value::Builtin(BuiltinFn {
                name: "math.max",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.max(b))),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
                    (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).max(*b))),
                    (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.max(*b as f64))),
                    _ => Err("math.max expects two numbers".to_string()),
                },
            }),
        ),
        (
            "math.min",
            Value::Builtin(BuiltinFn {
                name: "math.min",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.min(b))),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
                    (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).min(*b))),
                    (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.min(*b as f64))),
                    _ => Err("math.min expects two numbers".to_string()),
                },
            }),
        ),
        (
            "math.clamp",
            Value::Builtin(BuiltinFn {
                name: "math.clamp",
                arity: 3,
                func: |args| match (&args[0], &args[1], &args[2]) {
                    (Value::Int(val), Value::Int(min), Value::Int(max)) => {
                        Ok(Value::Int(*val.max(min).min(max)))
                    }
                    (Value::Float(val), Value::Float(min), Value::Float(max)) => {
                        Ok(Value::Float(val.max(*min).min(*max)))
                    }
                    _ => Err("math.clamp expects three numbers of the same type".to_string()),
                },
            }),
        ),
        // Constants / 常量
        ("math.pi", Value::Float(std::f64::consts::PI)),
        ("math.e", Value::Float(std::f64::consts::E)),
        ("math.inf", Value::Float(f64::INFINITY)),
        ("math.nan", Value::Float(f64::NAN)),
        // Type conversion / 类型转换
        (
            "math.toInt",
            Value::Builtin(BuiltinFn {
                name: "math.toInt",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Int(n) => Ok(Value::Int(*n)),
                    Value::Float(n) => Ok(Value::Int(*n as i64)),
                    Value::String(s) => s
                        .parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| format!("cannot parse '{}' as integer", s)),
                    _ => Err("math.toInt expects a number or string".to_string()),
                },
            }),
        ),
        (
            "math.toFloat",
            Value::Builtin(BuiltinFn {
                name: "math.toFloat",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Int(n) => Ok(Value::Float(*n as f64)),
                    Value::Float(n) => Ok(Value::Float(*n)),
                    Value::String(s) => s
                        .parse::<f64>()
                        .map(Value::Float)
                        .map_err(|_| format!("cannot parse '{}' as float", s)),
                    _ => Err("math.toFloat expects a number or string".to_string()),
                },
            }),
        ),
        // Predicates / 谓词
        (
            "math.isNan",
            Value::Builtin(BuiltinFn {
                name: "math.isNan",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => Ok(Value::Bool(n.is_nan())),
                    Value::Int(_) => Ok(Value::Bool(false)),
                    _ => Err("math.isNan expects a number".to_string()),
                },
            }),
        ),
        (
            "math.isInf",
            Value::Builtin(BuiltinFn {
                name: "math.isInf",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => Ok(Value::Bool(n.is_infinite())),
                    Value::Int(_) => Ok(Value::Bool(false)),
                    _ => Err("math.isInf expects a number".to_string()),
                },
            }),
        ),
    ]
}
