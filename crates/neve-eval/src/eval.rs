//! HIR expression evaluation.
//! HIR 表达式求值。
//!
//! This module implements the evaluator for the High-level Intermediate Representation (HIR).
//! It provides a tree-walking interpreter with tail call optimization.
//! 本模块实现了高级中间表示（HIR）的求值器。
//! 它提供了一个带有尾调用优化的树遍历解释器。

use crate::{Environment, Value};
use neve_common::{Span, int_is_negative, int_is_zero, int_to_f64, int_to_u32};
use neve_diagnostic::Diagnostic;
use neve_hir::{
    BinOp, DefId, Expr, ExprKind, FnDef, Generator, Item, ItemKind, Literal, LocalId, Module,
    UnaryOp, builtin_constructor_name,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use thiserror::Error;

/// Evaluation errors.
/// 求值错误。
#[derive(Debug, Error)]
pub enum EvalError {
    /// Unbound variable error / 未绑定变量错误
    #[error("unbound variable")]
    UnboundVariable,

    /// Type error / 类型错误
    #[error("type error: {0}")]
    TypeError(String),

    /// Division by zero error / 除零错误
    #[error("division by zero")]
    DivisionByZero,

    /// Assertion failed error / 断言失败错误
    #[error("assertion failed: {0}")]
    AssertionFailed(String),

    /// Pattern match failed error / 模式匹配失败错误
    #[error("pattern match failed")]
    PatternMatchFailed,

    /// Not a function error / 非函数错误
    #[error("not a function")]
    NotAFunction,

    /// Wrong number of arguments error / 参数数量错误
    #[error("wrong number of arguments")]
    WrongArity,

    /// Parse diagnostics for imported modules / 导入模块的解析诊断
    #[error("parse error in module '{module}'")]
    ParseDiagnostics {
        /// Module name / 模块名称
        module: String,
        /// File path / 文件路径
        path: PathBuf,
        /// Source content / 源码内容
        source_text: String,
        /// Parser diagnostics / 解析诊断
        diagnostics: Vec<Diagnostic>,
    },
}

/// Result of evaluating an expression with tail call detection.
/// 带有尾调用检测的表达式求值结果。
enum TcoResult {
    /// Normal value result / 正常值结果
    Value(Value),
    /// Tail call detected: (function, arguments) / 检测到尾调用：（函数，参数）
    TailCall(Value, Vec<Value>),
}

/// The HIR evaluator.
/// HIR 求值器。
///
/// This evaluator interprets HIR expressions with support for:
/// 此求值器解释 HIR 表达式，支持：
/// - Lexically scoped variables / 词法作用域变量
/// - Tail call optimization / 尾调用优化
/// - Pattern matching / 模式匹配
/// - Closures / 闭包
#[derive(Clone)]
pub struct Evaluator {
    /// Local variable environment / 局部变量环境
    env: Environment,
    /// Global definitions (functions, etc.) / 全局定义（函数等）
    globals: HashMap<DefId, GlobalDef>,
    /// Variant constructors by DefId. / 按 DefId 存储的变体构造器。
    variant_ctors: HashMap<DefId, VariantCtor>,
    /// Resolved method call targets keyed by expression span.
    /// 按表达式 span 存储的方法调用解析结果。
    method_resolutions: HashMap<Span, DefId>,
    /// Additional builtin bindings injected by the caller.
    /// 调用方注入的额外内置绑定。
    extra_builtins: HashMap<String, Value>,
}

/// A global definition.
/// 全局定义。
#[derive(Clone)]
enum GlobalDef {
    /// Function definition / 函数定义
    Function(FnDef),
    /// Evaluated value / 已求值的值
    Value(Value),
}

/// A variant constructor definition.
/// 变体构造器定义。
#[derive(Clone)]
struct VariantCtor {
    name: String,
    arity: usize,
}

impl Evaluator {
    /// Create a new evaluator.
    /// 创建一个新的求值器。
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
            globals: HashMap::new(),
            variant_ctors: HashMap::new(),
            method_resolutions: HashMap::new(),
            extra_builtins: HashMap::new(),
        }
    }

    /// Attach statically resolved method call targets.
    /// 绑定静态解析得到的方法调用目标。
    pub fn with_method_resolutions(mut self, method_resolutions: HashMap<Span, DefId>) -> Self {
        self.method_resolutions = method_resolutions;
        self
    }

    /// Replace statically resolved method call targets.
    /// 替换静态解析得到的方法调用目标。
    pub fn set_method_resolutions(&mut self, method_resolutions: HashMap<Span, DefId>) {
        self.method_resolutions = method_resolutions;
    }

    /// Evaluate a module using a caller-provided method-resolution table.
    /// 使用调用方提供的方法解析表来求值模块。
    pub fn eval_module_with_method_resolutions(
        &mut self,
        module: &Module,
        method_resolutions: &HashMap<Span, DefId>,
    ) -> Result<Value, EvalError> {
        self.set_method_resolutions(method_resolutions.clone());
        self.eval_module(module)
    }

    /// Call a runtime function value with explicit arguments.
    /// 使用显式参数调用运行时函数值。
    pub fn call_value(&mut self, func: Value, args: Vec<Value>) -> Result<Value, EvalError> {
        self.apply(func, args)
    }

    /// Attach additional builtin bindings.
    /// 绑定额外的内置值。
    pub fn with_extra_builtins<I>(mut self, builtins: I) -> Self
    where
        I: IntoIterator<Item = (String, Value)>,
    {
        self.extra_builtins.extend(builtins);
        self
    }

    /// Create an evaluator with built-in functions.
    /// 创建一个带有内置函数的求值器。
    pub fn with_builtins() -> Self {
        let mut eval = Self::new();
        eval.define_builtins();
        eval
    }

    /// Define built-in functions.
    /// 定义内置函数。
    fn define_builtins(&mut self) {
        // We'll store builtins as special values that can be called
        // For now, they're handled specially in apply()
        // 我们将内置函数存储为可调用的特殊值
        // 目前，它们在 apply() 中进行特殊处理
    }

    /// Evaluate a module and return the last value.
    /// 求值一个模块并返回最后一个值。
    pub fn eval_module(&mut self, module: &Module) -> Result<Value, EvalError> {
        // First pass: collect all global definitions
        for item in &module.items {
            self.collect_item(item);
        }

        // Second pass: evaluate definitions (for values) and return last result
        let mut result = Value::Unit;
        for item in &module.items {
            result = self.eval_item(item)?;
        }

        Ok(result)
    }

    fn collect_item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Fn(fn_def) => {
                if fn_def.params.is_empty()
                    && matches!(self.globals.get(&item.id), Some(GlobalDef::Value(_)))
                {
                    return;
                }
                self.globals
                    .insert(item.id, GlobalDef::Function(fn_def.clone()));
            }
            ItemKind::Enum(enum_def) => {
                for variant in &enum_def.variants {
                    self.variant_ctors.insert(
                        variant.id,
                        VariantCtor {
                            name: variant.name.clone(),
                            arity: variant.fields.len(),
                        },
                    );
                }
            }
            ItemKind::Impl(impl_def) => {
                for item in &impl_def.items {
                    self.globals.insert(
                        item.id,
                        GlobalDef::Function(FnDef {
                            name: item.name.clone(),
                            generics: item.generics.clone(),
                            params: item.params.clone(),
                            return_ty: item.return_ty.clone(),
                            body: item.body.clone(),
                        }),
                    );
                }
            }
            ItemKind::Expr(_) => {}
            _ => {}
        }
    }

    fn eval_item(&mut self, item: &Item) -> Result<Value, EvalError> {
        match &item.kind {
            ItemKind::Fn(fn_def) => {
                // For top-level let (converted to zero-param function), evaluate immediately
                if fn_def.params.is_empty() {
                    let value = self.eval(&fn_def.body)?;
                    self.globals
                        .insert(item.id, GlobalDef::Value(value.clone()));
                    Ok(value)
                } else {
                    // For real functions, they're already collected
                    Ok(Value::Unit)
                }
            }
            ItemKind::Expr(expr) => self.eval(expr),
            _ => Ok(Value::Unit),
        }
    }

    fn global_callable(&self, def_id: DefId) -> Option<Value> {
        match self.globals.get(&def_id).cloned() {
            Some(GlobalDef::Value(value)) => Some(value),
            Some(GlobalDef::Function(fn_def)) => Some(Value::Closure {
                params: fn_def.params,
                body: fn_def.body,
                env: self.env.clone(),
            }),
            None => None,
        }
    }

    /// Evaluate an expression.
    /// 求值一个表达式。
    pub fn eval(&mut self, expr: &Expr) -> Result<Value, EvalError> {
        match &expr.kind {
            ExprKind::Literal(lit) => Ok(self.eval_literal(lit)),

            ExprKind::Var(id) => self.env.get(*id).ok_or(EvalError::UnboundVariable),

            ExprKind::Global(def_id) => {
                match self.globals.get(def_id).cloned() {
                    Some(GlobalDef::Value(v)) => Ok(v),
                    Some(GlobalDef::Function(fn_def)) => {
                        // Return a closure value
                        Ok(Value::Closure {
                            params: fn_def.params.clone(),
                            body: fn_def.body.clone(),
                            env: self.env.clone(),
                        })
                    }
                    None => {
                        if let Some(ctor) = self.variant_ctors.get(def_id) {
                            return Ok(Value::VariantCtor {
                                name: ctor.name.clone(),
                                arity: ctor.arity,
                            });
                        }

                        // Check if it's a builtin
                        self.get_builtin(*def_id).ok_or(EvalError::UnboundVariable)
                    }
                }
            }

            ExprKind::Builtin(name) => self
                .builtin_value(name)
                .ok_or_else(|| EvalError::TypeError(format!("unknown builtin: {name}"))),

            ExprKind::Record(fields) => {
                let mut map = HashMap::with_capacity(fields.len());
                for (name, expr) in fields {
                    map.insert(name.clone(), self.eval(expr)?);
                }
                Ok(Value::Record(Rc::new(map)))
            }

            ExprKind::List(items) => {
                let values: Result<Vec<_>, _> = items.iter().map(|e| self.eval(e)).collect();
                Ok(Value::List(Rc::new(values?)))
            }

            ExprKind::Tuple(items) => {
                let values: Result<Vec<_>, _> = items.iter().map(|e| self.eval(e)).collect();
                Ok(Value::Tuple(Rc::new(values?)))
            }

            ExprKind::Lambda(params, body) => Ok(Value::Closure {
                params: params.clone(),
                body: (**body).clone(),
                env: self.env.clone(),
            }),

            ExprKind::Call(func, args) => {
                let func_val = self.eval(func)?;
                let arg_vals: Result<Vec<_>, _> = args.iter().map(|e| self.eval(e)).collect();
                self.apply(func_val, arg_vals?)
            }

            ExprKind::MethodCall {
                receiver,
                target,
                args,
                ..
            } => {
                let recv_val = self.eval(receiver)?;
                let mut arg_vals = vec![recv_val];
                for arg in args {
                    arg_vals.push(self.eval(arg)?);
                }

                // Canonical dispatch order mirrors type checking:
                // 1. use resolved inherent/trait method targets when available
                // 2. otherwise evaluate the lowered callable fallback target
                if let Some(method_def_id) = self.method_resolutions.get(&expr.span).copied()
                    && let Some(func_val) = self.global_callable(method_def_id)
                {
                    self.apply(func_val, arg_vals)
                } else {
                    let func_val = self.eval(target)?;
                    self.apply(func_val, arg_vals)
                }
            }

            ExprKind::Field(base, field) => {
                let base_val = self.eval(base)?;
                match base_val {
                    Value::Record(fields) => fields
                        .get(field)
                        .cloned()
                        .ok_or_else(|| EvalError::TypeError(format!("no field '{}'", field))),
                    _ => Err(EvalError::TypeError("not a record".to_string())),
                }
            }

            ExprKind::SafeField { base, field } => {
                let base_val = self.eval(base)?;
                match base_val {
                    Value::None => Ok(Value::None),
                    Value::Some(inner) => match *inner {
                        Value::Record(fields) => match fields.get(field) {
                            Some(v) => Ok(Value::Some(Box::new(v.clone()))),
                            None => Ok(Value::None),
                        },
                        _ => Err(EvalError::TypeError(
                            "safe field access requires a record".to_string(),
                        )),
                    },
                    Value::Record(fields) => match fields.get(field) {
                        Some(v) => Ok(Value::Some(Box::new(v.clone()))),
                        None => Ok(Value::None),
                    },
                    _ => Err(EvalError::TypeError(
                        "safe field access requires an option or record".to_string(),
                    )),
                }
            }

            ExprKind::TupleIndex(base, index) => {
                let base_val = self.eval(base)?;
                match base_val {
                    Value::Tuple(items) => items.get(*index as usize).cloned().ok_or_else(|| {
                        EvalError::TypeError("tuple index out of bounds".to_string())
                    }),
                    _ => Err(EvalError::TypeError("not a tuple".to_string())),
                }
            }

            ExprKind::Binary(op, left, right) => {
                let left_val = self.eval(left)?;
                let right_val = self.eval(right)?;
                self.eval_binary(*op, left_val, right_val)
            }

            ExprKind::Unary(op, operand) => {
                let val = self.eval(operand)?;
                self.eval_unary(*op, val)
            }

            ExprKind::If(cond, then_branch, else_branch) => {
                let cond_val = self.eval(cond)?;
                if cond_val.is_truthy() {
                    self.eval(then_branch)
                } else {
                    self.eval(else_branch)
                }
            }

            ExprKind::Coalesce { value, default } => {
                let value = self.eval(value)?;
                match value {
                    Value::None => self.eval(default),
                    Value::Some(value) => Ok((*value).clone()),
                    Value::VariantCtor { name, arity } if arity == 0 && name == "None" => {
                        self.eval(default)
                    }
                    Value::VariantCtor { .. } => Err(EvalError::TypeError(
                        "coalesce requires an option-like value".to_string(),
                    )),
                    Value::Variant(tag, payload) => match tag.as_str() {
                        "None" => self.eval(default),
                        "Some" => Ok((*payload).clone()),
                        _ => Err(EvalError::TypeError(
                            "coalesce requires an option-like value".to_string(),
                        )),
                    },
                    _ => Err(EvalError::TypeError(
                        "coalesce requires an option-like value".to_string(),
                    )),
                }
            }

            ExprKind::Try(inner) => {
                fn unwrap_try_value(value: Value) -> Result<Value, EvalError> {
                    match value {
                        Value::Ok(v) | Value::Some(v) => Ok((*v).clone()),
                        Value::Err(e) => Err(EvalError::TypeError(format!("{:?}", e))),
                        Value::None => Err(EvalError::TypeError("unwrap on None".to_string())),
                        Value::VariantCtor { name, arity } if arity == 0 && name == "None" => {
                            Err(EvalError::TypeError("unwrap on None".to_string()))
                        }
                        Value::VariantCtor { .. } => Err(EvalError::TypeError(
                            "try requires an option-like or result-like value".to_string(),
                        )),
                        Value::Variant(tag, payload) => match tag.as_str() {
                            "Ok" | "Some" => Ok((*payload).clone()),
                            "Err" => Err(EvalError::TypeError(format!("{:?}", payload))),
                            "None" => Err(EvalError::TypeError("unwrap on None".to_string())),
                            _ => Err(EvalError::TypeError(
                                "try requires an option-like or result-like value".to_string(),
                            )),
                        },
                        _ => Err(EvalError::TypeError(
                            "try requires an option-like or result-like value".to_string(),
                        )),
                    }
                }

                unwrap_try_value(self.eval(inner)?)
            }

            ExprKind::Block(stmts, expr) => {
                let old_env = self.env.clone();
                self.env = self.env.child();

                for stmt in stmts {
                    self.eval_stmt(stmt)?;
                }

                let result = if let Some(e) = expr {
                    self.eval(e)?
                } else {
                    Value::Unit
                };

                self.env = old_env;
                Ok(result)
            }

            ExprKind::Match(scrutinee, arms) => {
                let val = self.eval(scrutinee)?;
                for arm in arms {
                    if let Some(bindings) = self.match_pattern(&arm.pattern, &val) {
                        let old_env = self.env.clone();
                        self.env = self.env.child();

                        for (id, value) in bindings {
                            self.env.define(id, value);
                        }

                        // Check guard if present
                        if let Some(guard) = &arm.guard {
                            let guard_val = self.eval(guard)?;
                            if !guard_val.is_truthy() {
                                self.env = old_env;
                                continue;
                            }
                        }

                        let result = self.eval(&arm.body)?;
                        self.env = old_env;
                        return Ok(result);
                    }
                }
                Err(EvalError::PatternMatchFailed)
            }

            ExprKind::Interpolated(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        neve_hir::StringPart::Literal(s) => result.push_str(s),
                        neve_hir::StringPart::Expr(e) => {
                            let val = self.eval(e)?;
                            result.push_str(&Self::value_to_string(&val));
                        }
                    }
                }
                Ok(Value::String(Rc::new(result)))
            }

            ExprKind::Let {
                pattern,
                value,
                body,
                ..
            } => {
                let val = self.eval(value)?;
                let bindings = self
                    .match_pattern(pattern, &val)
                    .ok_or(EvalError::PatternMatchFailed)?;

                let old_env = self.env.clone();
                let new_env = self.env.child();
                new_env.define_many(bindings);
                self.env = new_env;

                let result = self.eval(body);
                self.env = old_env;
                result
            }

            ExprKind::Lazy(inner) => Ok(Value::Thunk(crate::value::Thunk::new_hir(
                inner.as_ref().clone(),
                self.env.clone(),
            ))),

            ExprKind::ListComp { body, generators } => {
                let mut results = Vec::new();
                self.eval_generators(body, generators, 0, &mut results)?;
                Ok(Value::List(Rc::new(results)))
            }

            ExprKind::Error(message) => Err(EvalError::TypeError(message.clone())),
        }
    }

    fn eval_generators(
        &mut self,
        body: &Expr,
        generators: &[Generator],
        index: usize,
        results: &mut Vec<Value>,
    ) -> Result<(), EvalError> {
        if index >= generators.len() {
            let value = self.eval(body)?;
            results.push(value);
            return Ok(());
        }

        let generator = &generators[index];
        let iter_val = self.eval(&generator.iter)?;
        let items = match iter_val {
            Value::List(items) => items,
            _ => {
                return Err(EvalError::TypeError(
                    "generator requires a list".to_string(),
                ));
            }
        };

        for item in items.iter() {
            let bindings = self
                .match_pattern(&generator.pattern, item)
                .ok_or(EvalError::PatternMatchFailed)?;

            let old_env = self.env.clone();
            let new_env = self.env.child();
            new_env.define_many(bindings);
            self.env = new_env;

            if let Some(condition) = &generator.condition {
                let cond_val = self.eval(condition)?;
                if !cond_val.is_truthy() {
                    self.env = old_env;
                    continue;
                }
            }

            self.eval_generators(body, generators, index + 1, results)?;
            self.env = old_env;
        }

        Ok(())
    }

    fn get_builtin(&self, _def_id: DefId) -> Option<Value> {
        // Builtin lookup is currently name-based through the shared builtin registry.
        // Evaluator-owned builtins that need runtime context are intercepted in apply().
        // 当前内置函数查找仍通过共享注册表按名称进行。
        // 需要求值器上下文的内置函数会在 apply() 中被求值器拦截处理。
        None
    }

    fn builtin_value(&self, name: &str) -> Option<Value> {
        self.extra_builtins.get(name).cloned().or_else(|| {
            crate::builtins()
                .into_iter()
                .find_map(|(builtin_name, value)| (builtin_name == name).then_some(value))
        })
    }

    fn eval_literal(&self, lit: &Literal) -> Value {
        match lit {
            Literal::Int(n) => Value::Int(n.clone()),
            Literal::Float(f) => Value::Float(*f),
            Literal::String(s) => Value::String(Rc::new(s.clone())),
            Literal::Char(c) => Value::Char(*c),
            Literal::Bool(b) => Value::Bool(*b),
            Literal::Unit => Value::Unit,
        }
    }

    fn eval_binary(&mut self, op: BinOp, left: Value, right: Value) -> Result<Value, EvalError> {
        match op {
            BinOp::Add => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => {
                    let af = int_to_f64(a).ok_or_else(|| {
                        EvalError::TypeError("integer too large for float".to_string())
                    })?;
                    Ok(Value::Float(af + b))
                }
                (Value::Float(a), Value::Int(b)) => {
                    let bf = int_to_f64(b).ok_or_else(|| {
                        EvalError::TypeError("integer too large for float".to_string())
                    })?;
                    Ok(Value::Float(a + bf))
                }
                _ => Err(EvalError::TypeError("cannot add".to_string())),
            },
            BinOp::Sub => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                _ => Err(EvalError::TypeError("cannot subtract".to_string())),
            },
            BinOp::Mul => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                _ => Err(EvalError::TypeError("cannot multiply".to_string())),
            },
            BinOp::Div => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => {
                    if int_is_zero(b) {
                        Err(EvalError::DivisionByZero)
                    } else {
                        Ok(Value::Int(a / b))
                    }
                }
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                _ => Err(EvalError::TypeError("cannot divide".to_string())),
            },
            BinOp::Mod => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => {
                    if int_is_zero(b) {
                        Err(EvalError::DivisionByZero)
                    } else {
                        Ok(Value::Int(a % b))
                    }
                }
                _ => Err(EvalError::TypeError("cannot modulo".to_string())),
            },
            BinOp::Pow => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => {
                    if int_is_negative(b) {
                        return Err(EvalError::TypeError(
                            "negative exponent for integer power".to_string(),
                        ));
                    }
                    let exp = int_to_u32(b).ok_or_else(|| {
                        EvalError::TypeError("integer exponent too large".to_string())
                    })?;
                    Ok(Value::Int(a.pow(exp)))
                }
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(*b))),
                _ => Err(EvalError::TypeError("cannot power".to_string())),
            },
            BinOp::Eq => Ok(Value::Bool(Self::values_equal(&left, &right))),
            BinOp::Ne => Ok(Value::Bool(!Self::values_equal(&left, &right))),
            BinOp::Lt => self.compare(&left, &right).map(|o| Value::Bool(o.is_lt())),
            BinOp::Le => self.compare(&left, &right).map(|o| Value::Bool(o.is_le())),
            BinOp::Gt => self.compare(&left, &right).map(|o| Value::Bool(o.is_gt())),
            BinOp::Ge => self.compare(&left, &right).map(|o| Value::Bool(o.is_ge())),
            BinOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            BinOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
            BinOp::Concat => match (&left, &right) {
                (Value::List(a), Value::List(b)) => {
                    let mut result: Vec<Value> = (*a).iter().cloned().collect();
                    result.extend((*b).iter().cloned());
                    Ok(Value::List(Rc::new(result)))
                }
                (Value::String(a), Value::String(b)) => {
                    Ok(Value::String(Rc::new(format!("{}{}", a, b))))
                }
                _ => Err(EvalError::TypeError("cannot concatenate".to_string())),
            },
            BinOp::Merge => match (&left, &right) {
                (Value::Record(a), Value::Record(b)) => {
                    let mut result: HashMap<String, Value> =
                        (*a).iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                    for (k, v) in b.iter() {
                        result.insert(k.clone(), v.clone());
                    }
                    Ok(Value::Record(Rc::new(result)))
                }
                _ => Err(EvalError::TypeError("cannot merge".to_string())),
            },
            BinOp::Pipe => {
                // a |> f  =>  f(a)
                self.apply(right, vec![left])
            }
        }
    }

    fn eval_unary(&self, op: UnaryOp, val: Value) -> Result<Value, EvalError> {
        match op {
            UnaryOp::Neg => match val {
                Value::Int(n) => Ok(Value::Int(-n)),
                Value::Float(f) => Ok(Value::Float(-f)),
                _ => Err(EvalError::TypeError("cannot negate".to_string())),
            },
            UnaryOp::Not => Ok(Value::Bool(!val.is_truthy())),
        }
    }

    fn values_equal(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Char(x), Value::Char(y)) => x == y,
            (Value::String(x), Value::String(y)) => x == y,
            (Value::Path(x), Value::Path(y)) => x == y,
            (Value::Bytes(x), Value::Bytes(y)) => x == y,
            (Value::Command(x), Value::Command(y)) => x == y,
            (Value::ProcessResult(x), Value::ProcessResult(y)) => x == y,
            (Value::Unit, Value::Unit) => true,
            (Value::None, Value::None) => true,
            (Value::List(x), Value::List(y)) => {
                x.len() == y.len()
                    && x.iter()
                        .zip(y.iter())
                        .all(|(a, b)| Self::values_equal(a, b))
            }
            (Value::Tuple(x), Value::Tuple(y)) => {
                x.len() == y.len()
                    && x.iter()
                        .zip(y.iter())
                        .all(|(a, b)| Self::values_equal(a, b))
            }
            _ => false,
        }
    }

    fn compare(&self, a: &Value, b: &Value) -> Result<std::cmp::Ordering, EvalError> {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(x.cmp(y)),
            (Value::Float(x), Value::Float(y)) => x
                .partial_cmp(y)
                .ok_or_else(|| EvalError::TypeError("cannot compare NaN".to_string())),
            (Value::String(x), Value::String(y)) => Ok(x.cmp(y)),
            (Value::Char(x), Value::Char(y)) => Ok(x.cmp(y)),
            _ => Err(EvalError::TypeError("cannot compare".to_string())),
        }
    }

    fn apply(&mut self, func: Value, args: Vec<Value>) -> Result<Value, EvalError> {
        // Tail call optimization: use iteration instead of recursion
        let mut current_func = func;
        let mut current_args = args;

        loop {
            match current_func {
                Value::Closure { params, body, env } => {
                    if current_args.len() != params.len() {
                        return Err(EvalError::WrongArity);
                    }

                    // Set up environment for function execution
                    let old_env = self.env.clone();
                    self.env = env.child();

                    for (param, arg) in params.iter().zip(current_args) {
                        self.env.define(param.id, arg);
                    }

                    // Evaluate the body and check if result is a tail call
                    match self.eval_with_tco(&body)? {
                        TcoResult::Value(v) => {
                            self.env = old_env;
                            return Ok(v);
                        }
                        TcoResult::TailCall(func, args) => {
                            // Tail call detected - loop instead of recurring
                            self.env = old_env;
                            current_func = func;
                            current_args = args;
                            continue;
                        }
                    }
                }
                Value::Builtin(builtin) => {
                    if let Some(result) =
                        self.apply_evaluator_owned_builtin(builtin.name, &current_args)?
                    {
                        return Ok(result);
                    }
                    if current_args.len() != builtin.arity {
                        return Err(EvalError::WrongArity);
                    }
                    return (builtin.func)(&current_args).map_err(EvalError::TypeError);
                }
                Value::BuiltinFn(name, func) => {
                    if let Some(result) = self.apply_evaluator_owned_builtin(name, &current_args)? {
                        return Ok(result);
                    }
                    return func(current_args).map_err(EvalError::TypeError);
                }
                Value::VariantCtor { name, arity } => {
                    if current_args.len() != arity {
                        return Err(EvalError::WrongArity);
                    }

                    let payload = match current_args.len() {
                        0 => Value::Unit,
                        1 => current_args.into_iter().next().unwrap(),
                        _ => Value::Tuple(Rc::new(current_args)),
                    };

                    return Ok(Value::Variant(name, Box::new(payload)));
                }
                Value::AstClosure(_) => {
                    // AstClosure not supported in HIR evaluator
                    return Err(EvalError::TypeError(
                        "AstClosure not supported in HIR evaluator".to_string(),
                    ));
                }
                _ => return Err(EvalError::NotAFunction),
            }
        }
    }

    fn apply_evaluator_owned_builtin(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> Result<Option<Value>, EvalError> {
        match name {
            "force" => {
                if args.len() != 1 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.force_value(&args[0])?))
            }
            "map" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_map(&args[0], &args[1])?))
            }
            "list.map" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_map(&args[0], &args[1])?))
            }
            "filter" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_filter(&args[0], &args[1])?))
            }
            "list.filter" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_filter(&args[0], &args[1])?))
            }
            "all" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_all(&args[0], &args[1])?))
            }
            "any" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_any(&args[0], &args[1])?))
            }
            _ => Ok(None),
        }
    }

    /// Evaluate expression and detect tail calls.
    /// 求值表达式并检测尾调用。
    ///
    /// This is the core of tail call optimization. When a call is in tail position,
    /// we return a TailCall result instead of recursing, allowing the outer loop
    /// in apply() to handle it iteratively.
    /// 这是尾调用优化的核心。当调用处于尾位置时，我们返回 TailCall 结果
    /// 而不是递归，从而允许 apply() 中的外层循环迭代处理它。
    fn eval_with_tco(&mut self, expr: &Expr) -> Result<TcoResult, EvalError> {
        match &expr.kind {
            // Direct call in tail position
            ExprKind::Call(func, args) => {
                let func_val = self.eval(func)?;
                let arg_vals: Result<Vec<_>, _> = args.iter().map(|e| self.eval(e)).collect();
                Ok(TcoResult::TailCall(func_val, arg_vals?))
            }

            // If-then-else: evaluate condition, then the appropriate branch with TCO
            ExprKind::If(cond, then_branch, else_branch) => {
                let cond_val = self.eval(cond)?;
                match cond_val {
                    Value::Bool(true) => self.eval_with_tco(then_branch),
                    Value::Bool(false) => self.eval_with_tco(else_branch),
                    _ => Err(EvalError::TypeError(
                        "condition must be boolean".to_string(),
                    )),
                }
            }

            // Block: evaluate statements, then final expression with TCO
            ExprKind::Block(stmts, final_expr) => {
                for stmt in stmts {
                    self.eval_stmt(stmt)?;
                }

                if let Some(expr) = final_expr {
                    self.eval_with_tco(expr)
                } else {
                    Ok(TcoResult::Value(Value::Unit))
                }
            }

            // Match: evaluate scrutinee, match pattern, then evaluate arm with TCO
            ExprKind::Match(scrutinee, arms) => {
                let scrutinee_val = self.eval(scrutinee)?;

                for arm in arms {
                    if let Some(bindings) = self.match_pattern(&arm.pattern, &scrutinee_val) {
                        // Check guard if present
                        if let Some(guard) = &arm.guard {
                            let old_env = self.env.clone();
                            for (id, val) in bindings {
                                self.env.define(id, val);
                            }
                            let guard_val = self.eval(guard)?;
                            self.env = old_env;

                            if guard_val != Value::Bool(true) {
                                continue;
                            }
                        } else {
                            // No guard, just bind variables
                            for (id, val) in bindings {
                                self.env.define(id, val);
                            }
                        }

                        // Evaluate the arm body with TCO
                        return self.eval_with_tco(&arm.body);
                    }
                }

                Err(EvalError::PatternMatchFailed)
            }

            // All other expressions are not in tail position - evaluate normally
            _ => {
                let val = self.eval(expr)?;
                Ok(TcoResult::Value(val))
            }
        }
    }

    fn force_value(&mut self, value: &Value) -> Result<Value, EvalError> {
        match value {
            Value::Thunk(thunk) => self.force_thunk(thunk),
            other => Ok(other.clone()),
        }
    }

    fn builtin_map(&mut self, func: &Value, list: &Value) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("map expects a list".to_string())),
        };

        let mut results = Vec::with_capacity(items.len());
        for item in items.iter() {
            results.push(self.apply(func.clone(), vec![item.clone()])?);
        }
        Ok(Value::List(Rc::new(results)))
    }

    fn builtin_filter(&mut self, pred: &Value, list: &Value) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("filter expects a list".to_string())),
        };

        let mut results = Vec::with_capacity(items.len());
        for item in items.iter() {
            if let Value::Bool(true) = self.apply(pred.clone(), vec![item.clone()])? {
                results.push(item.clone());
            }
        }
        Ok(Value::List(Rc::new(results)))
    }

    fn builtin_all(&mut self, pred: &Value, list: &Value) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("all expects a list".to_string())),
        };

        for item in items.iter() {
            if let Value::Bool(false) = self.apply(pred.clone(), vec![item.clone()])? {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    }

    fn builtin_any(&mut self, pred: &Value, list: &Value) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("any expects a list".to_string())),
        };

        for item in items.iter() {
            if let Value::Bool(true) = self.apply(pred.clone(), vec![item.clone()])? {
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    }

    fn force_thunk(&mut self, thunk: &crate::value::Thunk) -> Result<Value, EvalError> {
        {
            let state = thunk.state();
            match &*state {
                crate::value::ThunkState::Evaluated(v) => return Ok(v.clone()),
                crate::value::ThunkState::Evaluating => {
                    return Err(EvalError::TypeError(
                        "infinite recursion in lazy evaluation".to_string(),
                    ));
                }
                crate::value::ThunkState::HirUnevaluated { .. } => {}
                crate::value::ThunkState::AstUnevaluated { .. } => {
                    return Err(EvalError::TypeError(
                        "cannot force AST thunk in HIR evaluator".to_string(),
                    ));
                }
            }
        }

        let (expr, env) = {
            let mut state = thunk.state_mut();
            match std::mem::replace(&mut *state, crate::value::ThunkState::Evaluating) {
                crate::value::ThunkState::HirUnevaluated { expr, env } => (expr, env),
                _ => unreachable!(),
            }
        };

        let mut eval = Self {
            env,
            globals: self.globals.clone(),
            variant_ctors: self.variant_ctors.clone(),
            method_resolutions: self.method_resolutions.clone(),
            extra_builtins: self.extra_builtins.clone(),
        };
        let result = eval.eval(&expr);

        match result {
            Ok(value) => {
                let mut state = thunk.state_mut();
                *state = crate::value::ThunkState::Evaluated(value.clone());
                Ok(value)
            }
            Err(e) => {
                let mut state = thunk.state_mut();
                *state = crate::value::ThunkState::Evaluated(Value::Err(Box::new(Value::String(
                    Rc::new(e.to_string()),
                ))));
                Err(e)
            }
        }
    }

    fn eval_stmt(&mut self, stmt: &neve_hir::Stmt) -> Result<(), EvalError> {
        match &stmt.kind {
            neve_hir::StmtKind::Let { pattern, value, .. } => {
                let val = self.eval(value)?;
                let bindings = self
                    .match_pattern(pattern, &val)
                    .ok_or(EvalError::PatternMatchFailed)?;
                self.env.define_many(bindings);
                Ok(())
            }
            neve_hir::StmtKind::Expr(expr) => {
                self.eval(expr)?;
                Ok(())
            }
        }
    }

    fn match_pattern(
        &self,
        pattern: &neve_hir::Pattern,
        value: &Value,
    ) -> Option<Vec<(LocalId, Value)>> {
        use neve_hir::PatternKind;

        // Pre-calculate expected binding count to reduce allocations
        // 预先计算预期绑定数量以减少分配
        fn estimate_bindings(pattern: &neve_hir::Pattern) -> usize {
            use neve_hir::PatternKind;
            match &pattern.kind {
                PatternKind::Wildcard => 0,
                PatternKind::Var(_, _) => 1,
                PatternKind::Binding(_, _, pattern) => 1 + estimate_bindings(pattern),
                PatternKind::Literal(_) => 0,
                PatternKind::Tuple(patterns) | PatternKind::List(patterns) => {
                    patterns.iter().map(estimate_bindings).sum()
                }
                PatternKind::ListRest { init, rest, tail } => {
                    init.iter().map(estimate_bindings).sum::<usize>()
                        + rest.as_deref().map(estimate_bindings).unwrap_or(0)
                        + tail.iter().map(estimate_bindings).sum::<usize>()
                }
                PatternKind::Record(fields) => {
                    fields.iter().map(|(_, pat)| estimate_bindings(pat)).sum()
                }
                PatternKind::Constructor(_, patterns) | PatternKind::Or(patterns) => {
                    patterns.iter().map(estimate_bindings).sum()
                }
            }
        }

        match &pattern.kind {
            PatternKind::Wildcard => Some(Vec::new()),
            PatternKind::Var(id, _) => Some(vec![(*id, value.clone())]),
            PatternKind::Binding(id, _, pattern) => {
                let mut bindings = self.match_pattern(pattern, value)?;
                bindings.push((*id, value.clone()));
                Some(bindings)
            }
            PatternKind::Literal(lit) => {
                let lit_val = self.eval_literal(lit);
                if Self::values_equal(&lit_val, value) {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            PatternKind::Tuple(patterns) => {
                if let Value::Tuple(values) = value {
                    if patterns.len() != values.len() {
                        return None;
                    }
                    let capacity = patterns.iter().map(estimate_bindings).sum();
                    let mut bindings = Vec::with_capacity(capacity);
                    for (p, v) in patterns.iter().zip(values.iter()) {
                        bindings.extend(self.match_pattern(p, v)?);
                    }
                    Some(bindings)
                } else {
                    None
                }
            }
            PatternKind::List(patterns) => {
                if let Value::List(values) = value {
                    if patterns.len() != values.len() {
                        return None;
                    }
                    let capacity = patterns.iter().map(estimate_bindings).sum();
                    let mut bindings = Vec::with_capacity(capacity);
                    for (p, v) in patterns.iter().zip(values.iter()) {
                        bindings.extend(self.match_pattern(p, v)?);
                    }
                    Some(bindings)
                } else {
                    None
                }
            }
            PatternKind::ListRest { init, rest, tail } => {
                if let Value::List(values) = value {
                    let min_len = init.len() + tail.len();
                    if values.len() < min_len {
                        return None;
                    }

                    let capacity = init.iter().map(estimate_bindings).sum::<usize>()
                        + rest.as_deref().map(estimate_bindings).unwrap_or(0)
                        + tail.iter().map(estimate_bindings).sum::<usize>();
                    let mut bindings = Vec::with_capacity(capacity);

                    for (pattern, value) in init.iter().zip(values.iter()) {
                        bindings.extend(self.match_pattern(pattern, value)?);
                    }

                    if let Some(pattern) = rest {
                        let middle_start = init.len();
                        let middle_end = values.len() - tail.len();
                        let middle =
                            Value::List(Rc::new(values[middle_start..middle_end].to_vec()));
                        bindings.extend(self.match_pattern(pattern, &middle)?);
                    }

                    let tail_start = values.len() - tail.len();
                    for (pattern, value) in tail.iter().zip(values[tail_start..].iter()) {
                        bindings.extend(self.match_pattern(pattern, value)?);
                    }

                    Some(bindings)
                } else {
                    None
                }
            }
            PatternKind::Record(fields) => {
                if let Value::Record(record) = value {
                    let capacity = fields.iter().map(|(_, pat)| estimate_bindings(pat)).sum();
                    let mut bindings = Vec::with_capacity(capacity);
                    for (name, pat) in fields {
                        let val = record.get(name)?;
                        bindings.extend(self.match_pattern(pat, val)?);
                    }
                    Some(bindings)
                } else {
                    None
                }
            }
            PatternKind::Constructor(def_id, patterns) => {
                if let Some(ctor) = self.variant_ctors.get(def_id)
                    && let Value::Variant(tag, payload) = value
                    && tag == &ctor.name
                {
                    return match patterns.as_slice() {
                        [] => {
                            if matches!(**payload, Value::Unit) {
                                Some(Vec::new())
                            } else {
                                None
                            }
                        }
                        [p] => self.match_pattern(p, payload),
                        _ => {
                            if let Value::Tuple(values) = payload.as_ref() {
                                if values.len() != patterns.len() {
                                    return None;
                                }
                                let capacity = patterns.iter().map(estimate_bindings).sum();
                                let mut bindings = Vec::with_capacity(capacity);
                                for (p, v) in patterns.iter().zip(values.iter()) {
                                    bindings.extend(self.match_pattern(p, v)?);
                                }
                                Some(bindings)
                            } else {
                                None
                            }
                        }
                    };
                }

                match (
                    builtin_constructor_name(*def_id),
                    patterns.as_slice(),
                    value,
                ) {
                    (Some("Some"), [p], Value::Some(v)) => self.match_pattern(p, v),
                    (Some("None"), [], Value::None) => Some(Vec::new()),
                    (Some("Ok"), [p], Value::Ok(v)) => self.match_pattern(p, v),
                    (Some("Err"), [p], Value::Err(v)) => self.match_pattern(p, v),
                    _ => None,
                }
            }
            PatternKind::Or(patterns) => {
                for pattern in patterns {
                    if let Some(bindings) = self.match_pattern(pattern, value) {
                        return Some(bindings);
                    }
                }
                None
            }
        }
    }

    /// Convert a value to its string representation for interpolation.
    /// 将值转换为用于字符串插值的字符串表示。
    fn value_to_string(value: &Value) -> String {
        match value {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{:.1}", f)
                } else {
                    f.to_string()
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::Char(c) => c.to_string(),
            Value::String(s) => s.to_string(),
            Value::Unit => "()".to_string(),
            Value::Path(path) => path.display().to_string(),
            Value::Bytes(bytes) => format!("<bytes:{}>", bytes.len()),
            Value::Command(command) => {
                format!(
                    "<command:{} {} arg(s)>",
                    command.program(),
                    command.args().len()
                )
            }
            Value::ProcessResult(result) => format!(
                "<process-result:{} {}>",
                result.code(),
                if result.is_success() { "ok" } else { "err" }
            ),
            Value::None => "None".to_string(),
            Value::Some(v) => format!("Some({})", Self::value_to_string(v)),
            Value::Ok(v) => format!("Ok({})", Self::value_to_string(v)),
            Value::Err(v) => format!("Err({})", Self::value_to_string(v)),
            Value::List(items) => {
                let strs: Vec<String> = items.iter().map(Self::value_to_string).collect();
                format!("[{}]", strs.join(", "))
            }
            Value::Tuple(items) => {
                let strs: Vec<String> = items.iter().map(Self::value_to_string).collect();
                format!("({})", strs.join(", "))
            }
            Value::Record(fields) => {
                let strs: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{} = {}", k, Self::value_to_string(v)))
                    .collect();
                format!("#{{ {} }}", strs.join(", "))
            }
            Value::Map(map) => {
                let strs: Vec<String> = map
                    .iter()
                    .map(|(k, v)| format!("{} => {}", k, Self::value_to_string(v)))
                    .collect();
                format!("Map{{ {} }}", strs.join(", "))
            }
            Value::Set(set) => {
                let strs: Vec<String> = set.iter().cloned().collect();
                format!("Set{{ {} }}", strs.join(", "))
            }
            Value::Variant(tag, payload) => {
                if matches!(**payload, Value::Unit) {
                    tag.clone()
                } else {
                    format!("{}({})", tag, Self::value_to_string(payload))
                }
            }
            Value::VariantCtor { name, arity } => format!("<variant:{}:{}>", name, arity),
            Value::Builtin(b) => format!("<builtin:{}>", b.name),
            Value::BuiltinFn(name, _) => format!("<builtin:{}>", name),
            Value::AstClosure(_) => "<function>".to_string(),
            Value::Closure { .. } => "<function>".to_string(),
            Value::Thunk(thunk) => {
                use crate::value::ThunkState;
                match &*thunk.state() {
                    ThunkState::Evaluated(v) => Self::value_to_string(v),
                    ThunkState::Evaluating => "<thunk:evaluating>".to_string(),
                    ThunkState::AstUnevaluated { .. } | ThunkState::HirUnevaluated { .. } => {
                        "<thunk>".to_string()
                    }
                }
            }
        }
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neve_common::Span;
    use neve_hir::*;

    #[test]
    fn test_tail_call_optimization() {
        // This test verifies that TCO prevents stack overflow on deep recursion
        // We create a tail-recursive sum function: sum(n, acc) = if n == 0 then acc else sum(n-1, acc+n)

        let mut evaluator = Evaluator::new();

        // Create a simple tail-recursive function manually in HIR
        // fn sum(n: Int, acc: Int) -> Int = if n <= 0 then acc else sum(n - 1, acc + n)

        let span = Span::default();
        let n_id = LocalId(0);
        let acc_id = LocalId(1);
        let sum_def_id = DefId(0);

        // Build: if n <= 0 then acc else sum(n - 1, acc + n)
        let condition = Expr {
            kind: ExprKind::Binary(
                BinOp::Le,
                Box::new(Expr {
                    kind: ExprKind::Var(n_id),
                    ty: Ty {
                        kind: TyKind::Int,
                        span,
                    },
                    span,
                }),
                Box::new(Expr {
                    kind: ExprKind::Literal(Literal::Int(0.into())),
                    ty: Ty {
                        kind: TyKind::Int,
                        span,
                    },
                    span,
                }),
            ),
            ty: Ty {
                kind: TyKind::Bool,
                span,
            },
            span,
        };

        let then_branch = Expr {
            kind: ExprKind::Var(acc_id),
            ty: Ty {
                kind: TyKind::Int,
                span,
            },
            span,
        };

        // sum(n - 1, acc + n)
        let recursive_call = Expr {
            kind: ExprKind::Call(
                Box::new(Expr {
                    kind: ExprKind::Global(sum_def_id),
                    ty: Ty {
                        kind: TyKind::Int,
                        span,
                    },
                    span,
                }),
                vec![
                    Expr {
                        kind: ExprKind::Binary(
                            BinOp::Sub,
                            Box::new(Expr {
                                kind: ExprKind::Var(n_id),
                                ty: Ty {
                                    kind: TyKind::Int,
                                    span,
                                },
                                span,
                            }),
                            Box::new(Expr {
                                kind: ExprKind::Literal(Literal::Int(1.into())),
                                ty: Ty {
                                    kind: TyKind::Int,
                                    span,
                                },
                                span,
                            }),
                        ),
                        ty: Ty {
                            kind: TyKind::Int,
                            span,
                        },
                        span,
                    },
                    Expr {
                        kind: ExprKind::Binary(
                            BinOp::Add,
                            Box::new(Expr {
                                kind: ExprKind::Var(acc_id),
                                ty: Ty {
                                    kind: TyKind::Int,
                                    span,
                                },
                                span,
                            }),
                            Box::new(Expr {
                                kind: ExprKind::Var(n_id),
                                ty: Ty {
                                    kind: TyKind::Int,
                                    span,
                                },
                                span,
                            }),
                        ),
                        ty: Ty {
                            kind: TyKind::Int,
                            span,
                        },
                        span,
                    },
                ],
            ),
            ty: Ty {
                kind: TyKind::Int,
                span,
            },
            span,
        };

        let body = Expr {
            kind: ExprKind::If(
                Box::new(condition),
                Box::new(then_branch),
                Box::new(recursive_call),
            ),
            ty: Ty {
                kind: TyKind::Int,
                span,
            },
            span,
        };

        let fn_def = FnDef {
            name: "sum".to_string(),
            generics: vec![],
            params: vec![
                Param {
                    id: n_id,
                    name: "n".to_string(),
                    ty: Ty {
                        kind: TyKind::Int,
                        span,
                    },
                    span,
                },
                Param {
                    id: acc_id,
                    name: "acc".to_string(),
                    ty: Ty {
                        kind: TyKind::Int,
                        span,
                    },
                    span,
                },
            ],
            return_ty: Ty {
                kind: TyKind::Int,
                span,
            },
            body,
        };

        evaluator
            .globals
            .insert(sum_def_id, GlobalDef::Function(fn_def.clone()));

        // Call sum(100, 0) - should compute 1+2+...+100 = 5050
        let closure = Value::Closure {
            params: fn_def.params.clone(),
            body: fn_def.body.clone(),
            env: Environment::new(),
        };

        let result = evaluator.apply(closure, vec![Value::Int(100.into()), Value::Int(0.into())]);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(5050.into()));
    }
}
