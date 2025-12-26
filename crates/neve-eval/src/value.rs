//! Runtime values.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;
use neve_hir::{Expr, Param};
use crate::Environment;

// Forward declaration for AstClosure
pub use crate::ast_eval::AstClosure;

/// A runtime value.
#[derive(Clone)]
pub enum Value {
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Bool(bool),
    /// Character value
    Char(char),
    /// String value
    String(Rc<String>),
    /// Unit value
    Unit,
    /// List value
    List(Rc<Vec<Value>>),
    /// Tuple value
    Tuple(Rc<Vec<Value>>),
    /// Record value
    Record(Rc<HashMap<String, Value>>),
    /// Map value (immutable hash map)
    Map(Rc<HashMap<String, Value>>),
    /// Set value (immutable hash set)
    Set(Rc<HashSet<String>>),
    /// Closure (for HIR evaluation)
    Closure {
        params: Vec<Param>,
        body: Expr,
        env: Environment,
    },
    /// AST Closure (for direct AST evaluation)
    AstClosure(Rc<AstClosure>),
    /// Built-in function
    Builtin(BuiltinFn),
    /// Built-in function with Rc closure (for stdlib)
    BuiltinFn(&'static str, Rc<dyn Fn(Vec<Value>) -> Result<Value, String>>),
    /// Variant/enum value (tag, payload)
    Variant(String, Box<Value>),
    /// Option::Some
    Some(Box<Value>),
    /// Option::None
    None,
    /// Result::Ok
    Ok(Box<Value>),
    /// Result::Err
    Err(Box<Value>),
}

/// A built-in function.
#[derive(Clone)]
pub struct BuiltinFn {
    pub name: &'static str,
    pub arity: usize,
    pub func: fn(&[Value]) -> Result<Value, String>,
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Char(c) => write!(f, "'{}'", c),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Unit => write!(f, "()"),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", item)?;
                }
                write!(f, "]")
            }
            Value::Tuple(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", item)?;
                }
                write!(f, ")")
            }
            Value::Record(fields) => {
                write!(f, "#{{")?;
                for (i, (name, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} = {:?}", name, value)?;
                }
                write!(f, "}}")
            }
            Value::Map(map) => {
                write!(f, "Map{{")?;
                for (i, (key, value)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} => {:?}", key, value)?;
                }
                write!(f, "}}")
            }
            Value::Set(set) => {
                write!(f, "Set{{")?;
                for (i, elem) in set.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "}}")
            }
            Value::Closure { .. } => write!(f, "<closure>"),
            Value::AstClosure(_) => write!(f, "<function>"),
            Value::Builtin(b) => write!(f, "<builtin:{}>", b.name),
            Value::BuiltinFn(name, _) => write!(f, "<builtin:{}>", name),
            Value::Variant(tag, payload) => {
                if matches!(**payload, Value::Unit) {
                    write!(f, "{}", tag)
                } else {
                    write!(f, "{}({:?})", tag, payload)
                }
            }
            Value::Some(v) => write!(f, "Some({:?})", v),
            Value::None => write!(f, "None"),
            Value::Ok(v) => write!(f, "Ok({:?})", v),
            Value::Err(v) => write!(f, "Err({:?})", v),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Record(a), Value::Record(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Set(a), Value::Set(b)) => a == b,
            (Value::Variant(t1, v1), Value::Variant(t2, v2)) => t1 == t2 && v1 == v2,
            (Value::Some(a), Value::Some(b)) => a == b,
            (Value::None, Value::None) => true,
            (Value::Ok(a), Value::Ok(b)) => a == b,
            (Value::Err(a), Value::Err(b)) => a == b,
            // Closures and builtins are never equal
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Value {
    /// Check if the value is truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::None => false,
            _ => true,
        }
    }

    /// Try to get as integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to get as float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Try to get as bool.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as string.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}
