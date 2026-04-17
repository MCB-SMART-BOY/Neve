//! Math operations for the standard library.
//! 标准库的数学操作。

use neve_common::{
    Int, int_abs, int_from_f64, int_is_negative, int_to_f64, int_to_i64, int_to_u32, parse_int,
};
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
                    Value::Int(n) => Ok(Value::Int(int_abs(n))),
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
                    Value::Float(n) => int_from_f64(n.floor())
                        .map(Value::Int)
                        .ok_or_else(|| "math.floor expects a finite number".to_string()),
                    _ => Err("math.floor expects a Float".to_string()),
                },
            }),
        ),
        (
            "math.ceil",
            Value::Builtin(BuiltinFn {
                name: "math.ceil",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => int_from_f64(n.ceil())
                        .map(Value::Int)
                        .ok_or_else(|| "math.ceil expects a finite number".to_string()),
                    _ => Err("math.ceil expects a Float".to_string()),
                },
            }),
        ),
        (
            "math.round",
            Value::Builtin(BuiltinFn {
                name: "math.round",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Float(n) => int_from_f64(n.round())
                        .map(Value::Int)
                        .ok_or_else(|| "math.round expects a finite number".to_string()),
                    _ => Err("math.round expects a Float".to_string()),
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
                    _ => Err("math.sqrt expects a Float".to_string()),
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
                        if int_is_negative(exp) {
                            let base_f = int_to_f64(base)
                                .ok_or_else(|| "math.pow expects finite numbers".to_string())?;
                            let exp_i32 = int_to_i32(exp, "math.pow exponent")?;
                            Ok(Value::Float(base_f.powi(exp_i32)))
                        } else {
                            let exp_u32 = int_to_u32(exp)
                                .ok_or_else(|| "math.pow exponent is too large".to_string())?;
                            Ok(Value::Int(base.pow(exp_u32)))
                        }
                    }
                    (Value::Float(base), Value::Int(exp)) => {
                        let exp_i32 = int_to_i32(exp, "math.pow exponent")?;
                        Ok(Value::Float(base.powi(exp_i32)))
                    }
                    (Value::Float(base), Value::Float(exp)) => Ok(Value::Float(base.powf(*exp))),
                    (Value::Int(base), Value::Float(exp)) => {
                        let base_f = int_to_f64(base)
                            .ok_or_else(|| "math.pow expects finite numbers".to_string())?;
                        Ok(Value::Float(base_f.powf(*exp)))
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
                    _ => Err("math.log expects a Float".to_string()),
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
                    _ => Err("math.log10 expects a Float".to_string()),
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
                    _ => Err("math.exp expects a Float".to_string()),
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
                    _ => Err("math.sin expects a Float".to_string()),
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
                    _ => Err("math.cos expects a Float".to_string()),
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
                    _ => Err("math.tan expects a Float".to_string()),
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
                    (Value::Int(a), Value::Int(b)) => {
                        Ok(Value::Int(if a >= b { a.clone() } else { b.clone() }))
                    }
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
                    (Value::Int(a), Value::Float(b)) => {
                        let a = int_to_f64(a)
                            .ok_or_else(|| "math.max expects finite numbers".to_string())?;
                        Ok(Value::Float(a.max(*b)))
                    }
                    (Value::Float(a), Value::Int(b)) => {
                        let b = int_to_f64(b)
                            .ok_or_else(|| "math.max expects finite numbers".to_string())?;
                        Ok(Value::Float(a.max(b)))
                    }
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
                    (Value::Int(a), Value::Int(b)) => {
                        Ok(Value::Int(if a <= b { a.clone() } else { b.clone() }))
                    }
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
                    (Value::Int(a), Value::Float(b)) => {
                        let a = int_to_f64(a)
                            .ok_or_else(|| "math.min expects finite numbers".to_string())?;
                        Ok(Value::Float(a.min(*b)))
                    }
                    (Value::Float(a), Value::Int(b)) => {
                        let b = int_to_f64(b)
                            .ok_or_else(|| "math.min expects finite numbers".to_string())?;
                        Ok(Value::Float(a.min(b)))
                    }
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
                        let clamped = if val < min {
                            min.clone()
                        } else if val > max {
                            max.clone()
                        } else {
                            val.clone()
                        };
                        Ok(Value::Int(clamped))
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
                    Value::Int(n) => Ok(Value::Int(n.clone())),
                    Value::Float(n) => int_from_f64(*n)
                        .map(Value::Int)
                        .ok_or_else(|| format!("cannot parse '{}' as integer", n)),
                    Value::String(s) => parse_int(s)
                        .map(Value::Int)
                        .ok_or_else(|| format!("cannot parse '{}' as integer", s)),
                    Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 }.into())),
                    _ => Err("math.toInt expects a number, string, or bool".to_string()),
                },
            }),
        ),
        (
            "math.toFloat",
            Value::Builtin(BuiltinFn {
                name: "math.toFloat",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Int(n) => int_to_f64(n)
                        .map(Value::Float)
                        .ok_or_else(|| "math.toFloat expects a finite number".to_string()),
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
                    _ => Err("math.isNan expects a Float".to_string()),
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
                    _ => Err("math.isInf expects a Float".to_string()),
                },
            }),
        ),
    ]
}

fn int_to_i32(value: &Int, context: &str) -> Result<i32, String> {
    let as_i64 = int_to_i64(value).ok_or_else(|| format!("{context} is out of range"))?;
    i32::try_from(as_i64).map_err(|_| format!("{context} is out of range"))
}
