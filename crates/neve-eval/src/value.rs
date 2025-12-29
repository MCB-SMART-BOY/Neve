//! Runtime values.
//! 运行时值。
//!
//! This module defines all value types that can exist during Neve program execution.
//! 本模块定义了 Neve 程序执行过程中可能存在的所有值类型。

use crate::Environment;
use neve_hir::{Expr, Param};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

// Forward declaration for AstClosure
// AstClosure 的前向声明
pub use crate::ast_eval::AstClosure;

/// A thunk represents a suspended computation for lazy evaluation.
/// Thunk 表示用于惰性求值的暂停计算。
///
/// It can be in one of three states:
/// 它可以处于以下三种状态之一：
/// - Unevaluated: contains the expression and environment to evaluate
///   未求值：包含要求值的表达式和环境
/// - Evaluating: currently being evaluated (used to detect cycles)
///   正在求值：当前正在求值（用于检测循环）
/// - Evaluated: contains the cached result
///   已求值：包含缓存的结果
#[derive(Clone)]
pub struct Thunk {
    /// The inner state of the thunk, wrapped in Rc<RefCell> for shared mutable access.
    /// Thunk 的内部状态，用 Rc<RefCell> 包装以实现共享可变访问。
    inner: Rc<RefCell<ThunkState>>,
}

/// The state of a thunk.
/// Thunk 的状态。
#[derive(Clone)]
pub enum ThunkState {
    /// Unevaluated thunk with AST expression.
    /// 带有 AST 表达式的未求值 thunk。
    Unevaluated {
        expr: neve_syntax::Expr,
        env: Rc<crate::ast_eval::AstEnv>,
    },
    /// Currently being evaluated (for cycle detection).
    /// 当前正在求值（用于循环检测）。
    Evaluating,
    /// Already evaluated and cached.
    /// 已求值并缓存。
    Evaluated(Value),
}

impl Thunk {
    /// Create a new unevaluated thunk from an AST expression.
    /// 从 AST 表达式创建新的未求值 thunk。
    pub fn new(expr: neve_syntax::Expr, env: Rc<crate::ast_eval::AstEnv>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ThunkState::Unevaluated { expr, env })),
        }
    }

    /// Create a thunk that is already evaluated.
    /// 创建一个已求值的 thunk。
    pub fn evaluated(value: Value) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ThunkState::Evaluated(value))),
        }
    }

    /// Check if the thunk has been evaluated.
    /// 检查 thunk 是否已求值。
    pub fn is_evaluated(&self) -> bool {
        matches!(&*self.inner.borrow(), ThunkState::Evaluated(_))
    }

    /// Check if the thunk is currently being evaluated (cycle detection).
    /// 检查 thunk 是否正在求值（循环检测）。
    pub fn is_evaluating(&self) -> bool {
        matches!(&*self.inner.borrow(), ThunkState::Evaluating)
    }

    /// Get the state for inspection.
    /// 获取状态以供检查。
    pub fn state(&self) -> std::cell::Ref<'_, ThunkState> {
        self.inner.borrow()
    }

    /// Get mutable state for force evaluation.
    /// 获取可变状态以进行强制求值。
    pub fn state_mut(&self) -> std::cell::RefMut<'_, ThunkState> {
        self.inner.borrow_mut()
    }
}

impl fmt::Debug for Thunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.inner.borrow() {
            ThunkState::Unevaluated { .. } => write!(f, "<thunk:unevaluated>"),
            ThunkState::Evaluating => write!(f, "<thunk:evaluating>"),
            ThunkState::Evaluated(v) => write!(f, "<thunk:{:?}>", v),
        }
    }
}

/// A runtime value.
/// 运行时值。
///
/// This enum represents all possible values that can exist during program execution.
/// 此枚举表示程序执行期间可能存在的所有值。
#[derive(Clone)]
pub enum Value {
    // ===== Primitive types 基本类型 =====
    /// Integer value / 整数值
    Int(i64),
    /// Float value / 浮点数值
    Float(f64),
    /// Boolean value / 布尔值
    Bool(bool),
    /// Character value / 字符值
    Char(char),
    /// String value / 字符串值
    String(Rc<String>),
    /// Unit value / 单元值
    Unit,

    // ===== Collection types 集合类型 =====
    /// List value / 列表值
    List(Rc<Vec<Value>>),
    /// Tuple value / 元组值
    Tuple(Rc<Vec<Value>>),
    /// Record value / 记录值
    Record(Rc<HashMap<String, Value>>),
    /// Map value (immutable hash map) / 映射值（不可变哈希映射）
    Map(Rc<HashMap<String, Value>>),
    /// Set value (immutable hash set) / 集合值（不可变哈希集合）
    Set(Rc<HashSet<String>>),

    // ===== Function types 函数类型 =====
    /// Closure (for HIR evaluation) / 闭包（用于 HIR 求值）
    Closure {
        params: Vec<Param>,
        body: Expr,
        env: Environment,
    },
    /// AST Closure (for direct AST evaluation) / AST 闭包（用于直接 AST 求值）
    AstClosure(Rc<AstClosure>),
    /// Built-in function / 内置函数
    Builtin(BuiltinFn),
    /// Built-in function with Rc closure (for stdlib) / 带 Rc 闭包的内置函数（用于标准库）
    BuiltinFn(
        &'static str,
        Rc<dyn Fn(Vec<Value>) -> Result<Value, String>>,
    ),

    // ===== Algebraic data types 代数数据类型 =====
    /// Variant/enum value (tag, payload) / 变体/枚举值（标签，载荷）
    Variant(String, Box<Value>),
    /// Option::Some / 可选值 Some
    Some(Box<Value>),
    /// Option::None / 可选值 None
    None,
    /// Result::Ok / 结果值 Ok
    Ok(Box<Value>),
    /// Result::Err / 结果值 Err
    Err(Box<Value>),

    // ===== Lazy evaluation 惰性求值 =====
    /// Thunk (lazy value) / Thunk（惰性值）
    Thunk(Thunk),
}

/// A built-in function.
/// 内置函数。
#[derive(Clone)]
pub struct BuiltinFn {
    /// Function name / 函数名称
    pub name: &'static str,
    /// Number of arguments / 参数数量
    pub arity: usize,
    /// Function implementation / 函数实现
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
            Value::Thunk(thunk) => write!(f, "{:?}", thunk),
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
            // Thunks: compare by evaluated value if both are evaluated
            // Thunk：如果两者都已求值，则按求值后的值比较
            (Value::Thunk(a), Value::Thunk(b)) => {
                match (&*a.state(), &*b.state()) {
                    (ThunkState::Evaluated(va), ThunkState::Evaluated(vb)) => va == vb,
                    _ => false, // Unevaluated thunks are not equal / 未求值的 thunk 不相等
                }
            }
            // Closures and builtins are never equal
            // 闭包和内置函数永远不相等
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Value {
    /// Check if the value is truthy.
    /// 检查值是否为真值。
    ///
    /// In Neve, only `false` and `None` are falsy.
    /// 在 Neve 中，只有 `false` 和 `None` 是假值。
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::None => false,
            _ => true,
        }
    }

    /// Try to get as integer.
    /// 尝试获取整数值。
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to get as float.
    /// 尝试获取浮点数值。
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Try to get as bool.
    /// 尝试获取布尔值。
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as string.
    /// 尝试获取字符串值。
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}
