//! HIR expression evaluation.
//! HIR 表达式求值。
//!
//! This module implements the evaluator for the High-level Intermediate Representation (HIR).
//! It provides a tree-walking interpreter with tail call optimization.
//! 本模块实现了高级中间表示（HIR）的求值器。
//! 它提供了一个带有尾调用优化的树遍历解释器。

use crate::value::{EventKind, EventValue};
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
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

/// One HIR module together with the semantic side tables needed for evaluation.
/// 一个带有求值所需语义 side table 的 HIR 模块视图。
#[derive(Debug, Clone, Copy)]
pub struct EvaluableModuleRef<'a> {
    module: &'a Module,
    method_resolutions: &'a HashMap<Span, DefId>,
}

impl<'a> EvaluableModuleRef<'a> {
    /// Build a borrowed evaluable-module view from HIR plus method resolutions.
    /// 通过 HIR 与方法解析结果构建借用式可求值模块视图。
    pub fn new(module: &'a Module, method_resolutions: &'a HashMap<Span, DefId>) -> Self {
        Self {
            module,
            method_resolutions,
        }
    }
}

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

// === Signal handling infrastructure ===

/// Number of supported signal types.
const SIGNAL_COUNT: usize = 5;

/// Global atomic flags for signal detection.
static SIGNAL_FLAGS: [AtomicBool; SIGNAL_COUNT] = [
    AtomicBool::new(false), // SIGINT
    AtomicBool::new(false), // SIGTERM
    AtomicBool::new(false), // SIGHUP
    AtomicBool::new(false), // SIGUSR1
    AtomicBool::new(false), // SIGUSR2
];

fn signal_index(name: &str) -> Option<usize> {
    match name {
        "INT" => Some(0),
        "TERM" => Some(1),
        "HUP" => Some(2),
        "USR1" => Some(3),
        "USR2" => Some(4),
        _ => None,
    }
}

fn set_signal_flag(index: usize) {
    if index < SIGNAL_COUNT {
        // Release ordering: ensures the signal delivery is visible to the
        // evaluator thread when it does an Acquire load.
        SIGNAL_FLAGS[index].store(true, Ordering::Release);
    }
}

#[cfg(unix)]
extern "C" fn handle_sigint(_: i32) {
    set_signal_flag(0);
}
#[cfg(unix)]
extern "C" fn handle_sigterm(_: i32) {
    set_signal_flag(1);
}
#[cfg(unix)]
extern "C" fn handle_sighup(_: i32) {
    set_signal_flag(2);
}
#[cfg(unix)]
extern "C" fn handle_sigusr1(_: i32) {
    set_signal_flag(3);
}
#[cfg(unix)]
extern "C" fn handle_sigusr2(_: i32) {
    set_signal_flag(4);
}

#[cfg(unix)]
fn install_signal_handler(name: &str) -> Result<(), String> {
    let (sig, handler): (i32, extern "C" fn(i32)) = match name {
        "INT" => (libc::SIGINT, handle_sigint),
        "TERM" => (libc::SIGTERM, handle_sigterm),
        "HUP" => (libc::SIGHUP, handle_sighup),
        "USR1" => (libc::SIGUSR1, handle_sigusr1),
        "USR2" => (libc::SIGUSR2, handle_sigusr2),
        _ => return Err(format!("unknown signal: {name}")),
    };
    // Use sigaction instead of signal for portable, persistent handler semantics.
    // signal() behavior varies across Unix variants (one-shot vs persistent).
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = handler as usize;
        // SA_RESTART: automatically restart interrupted syscalls.
        // SA_NOCLDSTOP: don't receive SIGCHLD when child stops (not used here but safe).
        sa.sa_flags = libc::SA_RESTART;
        libc::sigaction(sig, &sa, std::ptr::null_mut());
    }
    Ok(())
}

#[cfg(not(unix))]
fn install_signal_handler(name: &str) -> Result<(), String> {
    let _ = name;
    Err("signal handling is not supported on this platform".to_string())
}

/// Kill a process by PID. Used for timeout enforcement in streaming I/O.
/// 通过 PID 终止进程。用于流式 I/O 的超时强制执行。
fn kill_process_by_pid(pid: u32) {
    #[cfg(unix)]
    {
        // Use libc::kill directly — more reliable than spawning a subprocess.
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();
    }
}

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
    defer_stack: Vec<Value>,
    /// Signal handlers registered via io.onSignal. Keyed by signal name ("INT", "TERM", etc.).
    /// 通过 io.onSignal 注册的信号处理程序。按信号名称（"INT"、"TERM" 等）索引。
    signal_handlers: HashMap<String, Value>,
    /// Maximum lines to process in streaming I/O before erroring.
    /// 流式 I/O 中处理的最大行数，超出则报错（默认 100,000）。
    max_stream_lines: usize,
    /// Maximum stdin payload size in bytes for process execution.
    /// 进程执行中 stdin 负载的最大字节数（默认 10 MB）。
    max_stdin_bytes: usize,
    /// Maximum intermediate buffer size in bytes for pipeline stages.
    /// 管道阶段中间缓冲区的最大字节数（默认 50 MB）。
    max_intermediate_buffer: usize,
}

/// Default limits for streaming I/O safety.
/// 流式 I/O 安全的默认限制。
const DEFAULT_MAX_STREAM_LINES: usize = 100_000;
const DEFAULT_MAX_STDIN_BYTES: usize = 10 * 1024 * 1024; // 10 MB
const DEFAULT_MAX_INTERMEDIATE_BUFFER: usize = 50 * 1024 * 1024; // 50 MB

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
            defer_stack: Vec::new(),
            signal_handlers: HashMap::new(),
            max_stream_lines: DEFAULT_MAX_STREAM_LINES,
            max_stdin_bytes: DEFAULT_MAX_STDIN_BYTES,
            max_intermediate_buffer: DEFAULT_MAX_INTERMEDIATE_BUFFER,
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

    /// Evaluate one HIR module through the canonical evaluable-module view.
    /// 通过规范的可求值模块视图求值单个 HIR 模块。
    pub fn eval_evaluable_module(
        &mut self,
        module: EvaluableModuleRef<'_>,
    ) -> Result<Value, EvalError> {
        self.eval_module_with_method_resolutions(module.module, module.method_resolutions)
    }

    /// Evaluate dependency-first modules and return the root module's value.
    /// 按依赖优先顺序求值模块，并返回根模块的值。
    pub fn eval_evaluable_modules<'a, I>(
        &mut self,
        modules: I,
        root_id: neve_hir::ModuleId,
    ) -> Result<Value, EvalError>
    where
        I: IntoIterator<Item = EvaluableModuleRef<'a>>,
    {
        let mut root_value = Value::Unit;

        for module in modules {
            let value = self.eval_evaluable_module(module)?;
            if module.module.id == root_id {
                root_value = value;
            }
        }

        Ok(root_value)
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
                            effectful: true,
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
                    self.run_defers()?;
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

    fn builtin_retry(
        &mut self,
        func: &Value,
        max_attempts: &Value,
        backoff: &Value,
    ) -> Result<Value, EvalError> {
        let max_attempts: u32 = match max_attempts {
            Value::Int(n) => n.clone().try_into().map_err(|_| {
                EvalError::TypeError("retry: maxAttempts must be positive".to_string())
            })?,
            _ => {
                return Err(EvalError::TypeError(
                    "retry: maxAttempts must be Int".to_string(),
                ));
            }
        };
        let backoff_ms: u64 = match backoff {
            Value::Int(n) => n.clone().try_into().map_err(|_| {
                EvalError::TypeError("retry: backoff must be non-negative".to_string())
            })?,
            _ => {
                return Err(EvalError::TypeError(
                    "retry: backoff must be Int".to_string(),
                ));
            }
        };

        let mut last_err = String::new();
        for attempt in 0..max_attempts {
            match self.apply(func.clone(), vec![]) {
                Ok(val) => return Ok(val),
                Err(e) => {
                    last_err = format!("{:?}", e);
                    if attempt + 1 < max_attempts {
                        std::thread::sleep(std::time::Duration::from_millis(
                            backoff_ms * (attempt as u64 + 1),
                        ));
                    }
                }
            }
        }
        Err(EvalError::TypeError(format!(
            "retry: all {} attempts failed: {}",
            max_attempts, last_err
        )))
    }

    fn builtin_ensure(
        &mut self,
        check: &Value,
        timeout: &Value,
        interval: &Value,
    ) -> Result<Value, EvalError> {
        let timeout_ms: u64 = match timeout {
            Value::Int(n) => n.clone().try_into().map_err(|_| {
                EvalError::TypeError("ensure: timeout must be non-negative".to_string())
            })?,
            _ => {
                return Err(EvalError::TypeError(
                    "ensure: timeout must be Int".to_string(),
                ));
            }
        };
        let interval_ms: u64 = match interval {
            Value::Int(n) => n.clone().try_into().map_err(|_| {
                EvalError::TypeError("ensure: interval must be positive".to_string())
            })?,
            _ => {
                return Err(EvalError::TypeError(
                    "ensure: interval must be Int".to_string(),
                ));
            }
        };

        let start = std::time::Instant::now();
        loop {
            match self.apply(check.clone(), vec![]) {
                Ok(Value::Bool(true)) => return Ok(Value::Bool(true)),
                Ok(_) => {}
                Err(e) => {
                    return Err(EvalError::TypeError(format!(
                        "ensure: check failed: {:?}",
                        e
                    )));
                }
            }
            if start.elapsed().as_millis() as u64 > timeout_ms {
                return Ok(Value::Bool(false));
            }
            std::thread::sleep(std::time::Duration::from_millis(interval_ms));
        }
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
            Literal::Path(p) => Value::Path(Rc::new(std::path::PathBuf::from(p.clone()))),
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
                (Value::String(a), Value::String(b)) => {
                    Ok(Value::String(Rc::new(format!("{}{}", a, b))))
                }
                (Value::String(s), Value::Char(c)) => {
                    Ok(Value::String(Rc::new(format!("{}{}", s, c))))
                }
                (Value::Char(c), Value::String(s)) => {
                    Ok(Value::String(Rc::new(format!("{}{}", c, s))))
                }
                _ => Err(EvalError::TypeError(format!(
                    "cannot add {:?} and {:?}",
                    left, right
                ))),
            },
            BinOp::Sub => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                _ => Err(EvalError::TypeError(format!(
                    "cannot subtract {:?} from {:?}",
                    right, left
                ))),
            },
            BinOp::Mul => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                _ => Err(EvalError::TypeError(format!(
                    "cannot multiply {:?} and {:?}",
                    left, right
                ))),
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
                _ => Err(EvalError::TypeError(format!(
                    "cannot divide {:?} by {:?}",
                    left, right
                ))),
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
                // a |> f:
                // - If left is Command and right is Command: cmd1 |> cmd2 => pipeline([cmd1, cmd2])
                // - If left is Pipeline and right is Command: pipe |> cmd => pipeline.append(cmd)
                // - Otherwise, function application: a |> f => f(a)
                match (&left, &right) {
                    (Value::Command(c1), Value::Command(c2)) => {
                        Ok(Value::Pipeline(std::rc::Rc::new(
                            crate::value::PipelineValue::new(vec![c1.clone(), c2.clone()])
                        )))
                    }
                    (Value::Pipeline(pipe), Value::Command(cmd)) => {
                        let mut commands = pipe.commands().to_vec();
                        commands.push(cmd.clone());
                        Ok(Value::Pipeline(std::rc::Rc::new(
                            crate::value::PipelineValue::new(commands)
                        )))
                    }
                    (Value::Command(_), _) => {
                        Err(EvalError::TypeError(
                            "command pipe: right side must be a Command".to_string()
                        ))
                    }
                    (Value::Pipeline(_), _) => {
                        Err(EvalError::TypeError(
                            "pipeline pipe: right side must be a Command".to_string()
                        ))
                    }
                    _ => self.apply(right, vec![left]),
                }
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
            // Check for pending signals before each evaluation step
            self.check_signals()?;

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
            "io.execCommandStreaming" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_exec_streaming(&args[0], &args[1])?))
            }
            "io.execCommandStreamingWithTimeout" => {
                if args.len() != 3 {
                    return Err(EvalError::WrongArity);
                }
                let timeout_ms = match &args[2] {
                    Value::Int(ms) => {
                        let ms: u64 = ms.clone().try_into().map_err(|_| {
                            EvalError::TypeError(
                                "execCommandStreamingWithTimeout: timeout must be non-negative"
                                    .to_string(),
                            )
                        })?;
                        ms
                    }
                    _ => {
                        return Err(EvalError::TypeError(
                            "execCommandStreamingWithTimeout: third argument must be Int (timeout in ms)".to_string(),
                        ));
                    }
                };
                Ok(Some(self.builtin_exec_streaming_with_timeout(
                    &args[0], &args[1], timeout_ms,
                )?))
            }
            "io.execPipelineStreaming" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(
                    self.builtin_exec_pipeline_streaming(&args[0], &args[1])?,
                ))
            }
            "io.execPipelineStreamingWithTimeout" => {
                if args.len() != 3 {
                    return Err(EvalError::WrongArity);
                }
                let timeout_ms = match &args[2] {
                    Value::Int(ms) => {
                        let ms: u64 = ms.clone().try_into().map_err(|_| {
                            EvalError::TypeError(
                                "execPipelineStreamingWithTimeout: timeout must be non-negative"
                                    .to_string(),
                            )
                        })?;
                        ms
                    }
                    _ => {
                        return Err(EvalError::TypeError(
                            "execPipelineStreamingWithTimeout: third argument must be Int (timeout in ms)".to_string(),
                        ));
                    }
                };
                Ok(Some(self.builtin_exec_pipeline_streaming_with_timeout(
                    &args[0], &args[1], timeout_ms,
                )?))
            }
            "io.readFileLines" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_read_file_lines(&args[0], &args[1])?))
            }
            "io.readFileLinesPath" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_read_file_lines_path(&args[0], &args[1])?))
            }
            "all" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_all(&args[0], &args[1])?))
            }
            "io.retry" => {
                if args.len() != 3 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_retry(&args[0], &args[1], &args[2])?))
            }
            "io.eventMap" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_event_map(&args[0], &args[1])?))
            }
            "io.eventFilter" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_event_filter(&args[0], &args[1])?))
            }
            "io.defer" => {
                if args.len() != 1 {
                    return Err(EvalError::WrongArity);
                }
                self.defer_stack.push(args[0].clone());
                Ok(Some(Value::Unit))
            }
            "io.onSignal" => {
                if args.len() != 2 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_on_signal(&args[0], &args[1])?))
            }
            "io.ensure" => {
                if args.len() != 3 {
                    return Err(EvalError::WrongArity);
                }
                Ok(Some(self.builtin_ensure(&args[0], &args[1], &args[2])?))
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

    fn builtin_exec_streaming(
        &mut self,
        command: &Value,
        callback: &Value,
    ) -> Result<Value, EvalError> {
        use std::io::{BufRead, Write};
        let cmd = match command {
            Value::Command(cmd) => cmd,
            _ => {
                return Err(EvalError::TypeError(
                    "execCommandStreaming expects a Command".to_string(),
                ));
            }
        };

        let mut process = std::process::Command::new(cmd.program());
        process.args(cmd.args());
        if let Some(cwd) = cmd.cwd() {
            process.current_dir(cwd);
        }
        for (k, v) in cmd.env() {
            process.env(k, v);
        }
        // Respect the Command's stdin setting instead of hardcoding null
        if cmd.stdin().is_some() {
            process.stdin(std::process::Stdio::piped());
        } else {
            process.stdin(std::process::Stdio::null());
        }
        process.stdout(std::process::Stdio::piped());
        process.stderr(std::process::Stdio::piped());

        let mut child = process
            .spawn()
            .map_err(|e| EvalError::TypeError(format!("spawn: {e}")))?;

        // Write stdin if configured, then drop pipe to signal EOF
        if let Some(stdin_data) = cmd.stdin()
            && let Some(mut stdin_pipe) = child.stdin.take()
        {
            let stdin_bytes = stdin_data.as_bytes();
            if stdin_bytes.len() > self.max_stdin_bytes {
                return Err(EvalError::TypeError(format!(
                    "execCommandStreaming: stdin size {} exceeds limit {}",
                    stdin_bytes.len(),
                    self.max_stdin_bytes
                )));
            }
            stdin_pipe
                .write_all(stdin_bytes)
                .map_err(|e| EvalError::TypeError(format!("stdin write: {e}")))?;
            // stdin_pipe dropped here -> pipe closed -> child sees EOF
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| EvalError::TypeError("no stdout".to_string()))?;
        let reader = std::io::BufReader::new(stdout);

        let mut line_count: usize = 0;
        for line in reader.lines() {
            line_count += 1;
            if line_count > self.max_stream_lines {
                // Kill the process and return error
                let _ = child.kill();
                return Err(EvalError::TypeError(format!(
                    "execCommandStreaming: exceeded max stream lines ({})",
                    self.max_stream_lines
                )));
            }
            let line = line.map_err(|e| EvalError::TypeError(format!("read: {e}")))?;
            self.apply(
                callback.clone(),
                vec![Value::String(std::rc::Rc::new(line))],
            )?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| EvalError::TypeError(format!("wait: {e}")))?;
        let code = output.status.code().unwrap_or(-1);
        Ok(Value::ProcessResult(std::rc::Rc::new(
            crate::value::ProcessResultValue::new(
                code,
                output.status.success(),
                String::from_utf8_lossy(&output.stdout).to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            ),
        )))
    }

    /// Streaming pipeline execution: execute pipeline stages, connect stdout->stdin,
    /// and stream the final stage's stdout line by line to the callback.
    /// 流式管道执行：执行管道各阶段，连接 stdout->stdin，
    /// 并将最后阶段的 stdout 逐行流式传递给回调。
    fn builtin_exec_pipeline_streaming(
        &mut self,
        pipeline: &Value,
        callback: &Value,
    ) -> Result<Value, EvalError> {
        use std::io::{BufRead, Write};
        let pipeline = match pipeline {
            Value::Pipeline(p) => p,
            _ => {
                return Err(EvalError::TypeError(
                    "execPipelineStreaming expects a Pipeline".to_string(),
                ));
            }
        };

        let commands = pipeline.commands();
        if commands.is_empty() {
            return Err(EvalError::TypeError(
                "execPipelineStreaming: pipeline requires at least one command".to_string(),
            ));
        }

        // Execute pipeline stages sequentially, connecting stdout->stdin
        let mut previous_stdout: Option<Vec<u8>> = None;
        let mut combined_stderr = Vec::new();
        let mut last_code = 0;
        let mut last_success = false;
        let mut final_stdout = Vec::new();

        for (idx, cmd) in commands.iter().enumerate() {
            let mut process = std::process::Command::new(cmd.program());
            process.args(cmd.args());
            if let Some(cwd) = cmd.cwd() {
                process.current_dir(cwd);
            }
            for (k, v) in cmd.env() {
                process.env(k, v);
            }

            let is_last = idx == commands.len() - 1;

            // Determine stdin for this stage
            let stage_stdin: Option<&[u8]> = if idx == 0 {
                // First stage: use configured stdin, or previous_stdout (unlikely but safe)
                if let Some(s) = cmd.stdin() {
                    Some(s.as_bytes())
                } else {
                    previous_stdout.as_deref()
                }
            } else {
                // Subsequent stages: always use previous stdout
                previous_stdout.as_deref()
            };

            if stage_stdin.is_some() {
                process.stdin(std::process::Stdio::piped());
            } else {
                process.stdin(std::process::Stdio::null());
            }
            process.stdout(std::process::Stdio::piped());
            process.stderr(std::process::Stdio::piped());

            let mut child = process.spawn().map_err(|e| {
                EvalError::TypeError(format!("execPipelineStreaming stage {idx}: spawn: {e}"))
            })?;

            // Write stdin for this stage, then drop pipe to signal EOF
            if let Some(data) = stage_stdin
                && let Some(mut stdin_pipe) = child.stdin.take()
            {
                // Check stdin size for first stage (user-supplied)
                if idx == 0 && data.len() > self.max_stdin_bytes {
                    return Err(EvalError::TypeError(format!(
                        "execPipelineStreaming: stage 1 stdin size {} exceeds limit {}",
                        data.len(),
                        self.max_stdin_bytes
                    )));
                }
                stdin_pipe.write_all(data).map_err(|e| {
                    EvalError::TypeError(format!(
                        "execPipelineStreaming stage {idx}: stdin write: {e}"
                    ))
                })?;
                // stdin_pipe dropped here -> pipe closed -> child sees EOF
            }

            if is_last {
                // Final stage: stream stdout line by line
                let stdout = child.stdout.take().ok_or_else(|| {
                    EvalError::TypeError(
                        "execPipelineStreaming: no stdout on final stage".to_string(),
                    )
                })?;
                let reader = std::io::BufReader::new(stdout);

                let mut line_count: usize = 0;
                for line in reader.lines() {
                    line_count += 1;
                    if line_count > self.max_stream_lines {
                        let _ = child.kill();
                        return Err(EvalError::TypeError(format!(
                            "execPipelineStreaming: exceeded max stream lines ({})",
                            self.max_stream_lines
                        )));
                    }
                    let line = line.map_err(|e| EvalError::TypeError(format!("read: {e}")))?;
                    self.apply(
                        callback.clone(),
                        vec![Value::String(std::rc::Rc::new(line))],
                    )?;
                }

                let output = child.wait_with_output().map_err(|e| {
                    EvalError::TypeError(format!("execPipelineStreaming: wait: {e}"))
                })?;
                last_code = output.status.code().unwrap_or(-1);
                last_success = output.status.success();
                final_stdout = output.stdout;
                combined_stderr.extend_from_slice(&output.stderr);
            } else {
                // Non-final stage: collect all output
                let output = child.wait_with_output().map_err(|e| {
                    EvalError::TypeError(format!("execPipelineStreaming stage {idx}: wait: {e}"))
                })?;
                last_code = output.status.code().unwrap_or(-1);
                last_success = output.status.success();
                if output.stdout.len() > self.max_intermediate_buffer {
                    return Err(EvalError::TypeError(format!(
                        "execPipelineStreaming: stage {} output {} exceeds intermediate buffer limit {}",
                        idx + 1,
                        output.stdout.len(),
                        self.max_intermediate_buffer
                    )));
                }
                previous_stdout = Some(output.stdout);
                combined_stderr.extend_from_slice(&output.stderr);
            }
        }

        Ok(Value::ProcessResult(std::rc::Rc::new(
            crate::value::ProcessResultValue::new(
                last_code,
                last_success,
                String::from_utf8_lossy(&final_stdout).to_string(),
                String::from_utf8_lossy(&combined_stderr).to_string(),
            ),
        )))
    }

    /// Streaming command execution with total timeout.
    /// 带总超时的流式命令执行。
    ///
    /// Spawns the process, streams stdout line-by-line through a channel,
    /// and enforces a total execution deadline. Returns None on timeout.
    /// 启动进程，通过通道逐行传输 stdout，并强制执行总执行期限。超时返回 None。
    fn builtin_exec_streaming_with_timeout(
        &mut self,
        command: &Value,
        callback: &Value,
        timeout_ms: u64,
    ) -> Result<Value, EvalError> {
        use std::io::{BufRead, Write};
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        let cmd = match command {
            Value::Command(cmd) => cmd,
            _ => {
                return Err(EvalError::TypeError(
                    "execCommandStreamingWithTimeout expects a Command".to_string(),
                ));
            }
        };

        let mut process = std::process::Command::new(cmd.program());
        process.args(cmd.args());
        if let Some(cwd) = cmd.cwd() {
            process.current_dir(cwd);
        }
        for (k, v) in cmd.env() {
            process.env(k, v);
        }
        if cmd.stdin().is_some() {
            process.stdin(std::process::Stdio::piped());
        } else {
            process.stdin(std::process::Stdio::null());
        }
        process.stdout(std::process::Stdio::piped());
        process.stderr(std::process::Stdio::piped());

        let mut child = process
            .spawn()
            .map_err(|e| EvalError::TypeError(format!("spawn: {e}")))?;
        let pid = child.id();

        // Write stdin if configured
        if let Some(stdin_data) = cmd.stdin()
            && let Some(mut stdin_pipe) = child.stdin.take()
        {
            let stdin_bytes = stdin_data.as_bytes();
            if stdin_bytes.len() > self.max_stdin_bytes {
                return Err(EvalError::TypeError(format!(
                    "execCommandStreamingWithTimeout: stdin size {} exceeds limit {}",
                    stdin_bytes.len(),
                    self.max_stdin_bytes
                )));
            }
            stdin_pipe
                .write_all(stdin_bytes)
                .map_err(|e| EvalError::TypeError(format!("stdin write: {e}")))?;
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| EvalError::TypeError("no stdout".to_string()))?;

        // Channel for streaming lines to the evaluator thread
        let (line_tx, line_rx) = mpsc::channel::<Result<String, String>>();
        // Channel for the final process result
        let (result_tx, result_rx) =
            mpsc::channel::<Result<(i32, bool, Vec<u8>, Vec<u8>), String>>();

        // Spawn reader thread
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if line_tx.send(Ok(l)).is_err() {
                            // Receiver dropped (timeout) — stop reading
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = line_tx.send(Err(format!("read: {e}")));
                        break;
                    }
                }
            }
            // All lines sent; now wait for process exit
            match child.wait_with_output() {
                Ok(output) => {
                    let _ = result_tx.send(Ok((
                        output.status.code().unwrap_or(-1),
                        output.status.success(),
                        output.stdout,
                        output.stderr,
                    )));
                }
                Err(e) => {
                    let _ = result_tx.send(Err(format!("wait: {e}")));
                }
            }
        });

        let deadline = Instant::now() + Duration::from_millis(timeout_ms);

        // Main loop: receive lines with timeout, call callback
        let mut line_count: usize = 0;
        loop {
            let now = Instant::now();
            if now >= deadline {
                // Timeout — kill the process
                kill_process_by_pid(pid);
                return Ok(Value::None);
            }

            let remaining = deadline - now;
            match line_rx.recv_timeout(remaining) {
                Ok(Ok(line)) => {
                    line_count += 1;
                    if line_count > self.max_stream_lines {
                        kill_process_by_pid(pid);
                        return Err(EvalError::TypeError(format!(
                            "execCommandStreamingWithTimeout: exceeded max stream lines ({})",
                            self.max_stream_lines
                        )));
                    }
                    self.apply(
                        callback.clone(),
                        vec![Value::String(std::rc::Rc::new(line))],
                    )?;
                    // Continue to next line
                }
                Ok(Err(e)) => {
                    // Reader thread error — return immediately.
                    // The result channel will be dropped when this function returns.
                    return Err(EvalError::TypeError(e));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Should not happen with correct remaining calculation,
                    // but handle as timeout
                    kill_process_by_pid(pid);
                    return Ok(Value::None);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // All lines processed; collect final result
                    break;
                }
            }
        }

        // Collect final process result
        match result_rx.recv() {
            Ok(Ok((code, success, stdout, stderr))) => Ok(Value::Some(Box::new(
                Value::ProcessResult(std::rc::Rc::new(crate::value::ProcessResultValue::new(
                    code,
                    success,
                    String::from_utf8_lossy(&stdout).to_string(),
                    String::from_utf8_lossy(&stderr).to_string(),
                ))),
            ))),
            Ok(Err(e)) => Err(EvalError::TypeError(e)),
            Err(_) => Err(EvalError::TypeError(
                "execCommandStreamingWithTimeout: internal error".to_string(),
            )),
        }
    }

    /// Streaming pipeline execution with total timeout.
    /// 带总超时的流式管道执行。
    fn builtin_exec_pipeline_streaming_with_timeout(
        &mut self,
        pipeline: &Value,
        callback: &Value,
        timeout_ms: u64,
    ) -> Result<Value, EvalError> {
        use std::io::{BufRead, Write};
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        let pipeline = match pipeline {
            Value::Pipeline(p) => p,
            _ => {
                return Err(EvalError::TypeError(
                    "execPipelineStreamingWithTimeout expects a Pipeline".to_string(),
                ));
            }
        };

        let commands = pipeline.commands();
        if commands.is_empty() {
            return Err(EvalError::TypeError(
                "execPipelineStreamingWithTimeout: pipeline requires at least one command"
                    .to_string(),
            ));
        }

        // Build command specs for the thread
        struct StageSpec {
            program: String,
            args: Vec<String>,
            cwd: Option<String>,
            env: std::collections::HashMap<String, String>,
            stdin: Option<String>,
        }

        let stages: Vec<StageSpec> = commands
            .iter()
            .map(|cmd| StageSpec {
                program: cmd.program().to_string(),
                args: cmd.args().to_vec(),
                cwd: cmd.cwd().map(|s| s.to_string()),
                env: cmd.env().clone(),
                stdin: cmd.stdin().map(|s| s.to_string()),
            })
            .collect();

        let last_idx = stages.len() - 1;

        // Channel for streaming final-stage lines
        let (line_tx, line_rx) = mpsc::channel::<Result<String, String>>();
        // Channel for the final process result
        let (result_tx, result_rx) =
            mpsc::channel::<Result<(i32, bool, Vec<u8>, Vec<u8>), String>>();

        // Check first stage stdin size before spawning thread
        if let Some(first) = stages.first() {
            if let Some(ref stdin_str) = first.stdin {
                if stdin_str.len() > self.max_stdin_bytes {
                    return Err(EvalError::TypeError(format!(
                        "execPipelineStreamingWithTimeout: stage 1 stdin size {} exceeds limit {}",
                        stdin_str.len(),
                        self.max_stdin_bytes
                    )));
                }
            }
        }

        // Capture limits for use inside the thread
        let max_intermediate = self.max_intermediate_buffer;

        // PID tracking for kill-on-timeout
        let current_pid = std::sync::Arc::new(std::sync::Mutex::new(None::<u32>));
        let pid_for_kill = current_pid.clone();

        std::thread::spawn(move || {
            let mut previous_stdout: Option<Vec<u8>> = None;
            let mut combined_stderr = Vec::new();
            let mut last_code = 0;
            let mut last_success = false;

            for (idx, stage) in stages.iter().enumerate() {
                let mut proc = std::process::Command::new(&stage.program);
                proc.args(&stage.args);
                if let Some(cwd) = &stage.cwd {
                    proc.current_dir(cwd);
                }
                for (k, v) in &stage.env {
                    proc.env(k, v);
                }

                let is_last = idx == last_idx;

                let stage_stdin: Option<&[u8]> = if idx == 0 {
                    if let Some(s) = &stage.stdin {
                        Some(s.as_bytes())
                    } else {
                        previous_stdout.as_deref()
                    }
                } else {
                    previous_stdout.as_deref()
                };

                if stage_stdin.is_some() {
                    proc.stdin(std::process::Stdio::piped());
                } else {
                    proc.stdin(std::process::Stdio::null());
                }
                proc.stdout(std::process::Stdio::piped());
                proc.stderr(std::process::Stdio::piped());

                let mut child = match proc.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = line_tx.send(Err(format!("stage {idx}: spawn: {e}")));
                        return;
                    }
                };

                // Track PID for kill-on-timeout
                *pid_for_kill.lock().unwrap() = Some(child.id());

                // Write stdin for this stage
                if let Some(data) = stage_stdin
                    && let Some(mut stdin_pipe) = child.stdin.take()
                {
                    if let Err(e) = stdin_pipe.write_all(data) {
                        let _ = line_tx.send(Err(format!("stage {idx}: stdin write: {e}")));
                        return;
                    }
                }

                if is_last {
                    // Final stage: stream stdout line by line
                    let stdout = match child.stdout.take() {
                        Some(s) => s,
                        None => {
                            let _ = line_tx.send(Err("no stdout on final stage".to_string()));
                            return;
                        }
                    };
                    let reader = std::io::BufReader::new(stdout);
                    for line in reader.lines() {
                        match line {
                            Ok(l) => {
                                if line_tx.send(Ok(l)).is_err() {
                                    break; // Receiver dropped (timeout)
                                }
                            }
                            Err(e) => {
                                let _ = line_tx.send(Err(format!("read: {e}")));
                                break;
                            }
                        }
                    }
                    match child.wait_with_output() {
                        Ok(output) => {
                            last_code = output.status.code().unwrap_or(-1);
                            last_success = output.status.success();
                            combined_stderr.extend_from_slice(&output.stderr);
                            let _ = result_tx.send(Ok((
                                last_code,
                                last_success,
                                output.stdout,
                                combined_stderr.clone(),
                            )));
                        }
                        Err(e) => {
                            let _ = result_tx.send(Err(format!("wait: {e}")));
                        }
                    }
                } else {
                    // Non-final stage: collect output for next stage
                    match child.wait_with_output() {
                        Ok(output) => {
                            if output.stdout.len() > max_intermediate {
                                let _ = line_tx.send(Err(format!(
                                    "stage {} output {} exceeds intermediate buffer limit {}",
                                    idx + 1,
                                    output.stdout.len(),
                                    max_intermediate
                                )));
                                return;
                            }
                            last_code = output.status.code().unwrap_or(-1);
                            last_success = output.status.success();
                            previous_stdout = Some(output.stdout);
                            combined_stderr.extend_from_slice(&output.stderr);
                        }
                        Err(e) => {
                            let _ = line_tx.send(Err(format!("stage {idx}: wait: {e}")));
                            return;
                        }
                    }
                }
            }

            *pid_for_kill.lock().unwrap() = None;
        });

        let deadline = Instant::now() + Duration::from_millis(timeout_ms);

        // Main loop: receive lines with timeout, call callback
        let mut line_count: usize = 0;
        loop {
            let now = Instant::now();
            if now >= deadline {
                // Timeout — kill the process
                if let Some(pid) = *current_pid.lock().unwrap() {
                    kill_process_by_pid(pid);
                }
                return Ok(Value::None);
            }

            let remaining = deadline - now;
            match line_rx.recv_timeout(remaining) {
                Ok(Ok(line)) => {
                    line_count += 1;
                    if line_count > self.max_stream_lines {
                        if let Some(pid) = *current_pid.lock().unwrap() {
                            kill_process_by_pid(pid);
                        }
                        return Err(EvalError::TypeError(format!(
                            "execPipelineStreamingWithTimeout: exceeded max stream lines ({})",
                            self.max_stream_lines
                        )));
                    }
                    self.apply(
                        callback.clone(),
                        vec![Value::String(std::rc::Rc::new(line))],
                    )?;
                }
                Ok(Err(e)) => {
                    // Don't block on result_rx — the reader thread may not have
                    // sent on it. The channel will be dropped when we return.
                    return Err(EvalError::TypeError(e));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if let Some(pid) = *current_pid.lock().unwrap() {
                        kill_process_by_pid(pid);
                    }
                    return Ok(Value::None);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }

        // Collect final process result (with timeout safety net)
        match result_rx.recv_timeout(std::time::Duration::from_millis(500)) {
            Ok(Ok((code, success, stdout, stderr))) => Ok(Value::Some(Box::new(
                Value::ProcessResult(std::rc::Rc::new(crate::value::ProcessResultValue::new(
                    code,
                    success,
                    String::from_utf8_lossy(&stdout).to_string(),
                    String::from_utf8_lossy(&stderr).to_string(),
                ))),
            ))),
            Ok(Err(e)) => Err(EvalError::TypeError(e)),
            Err(_) => Err(EvalError::TypeError(
                "execPipelineStreamingWithTimeout: internal error".to_string(),
            )),
        }
    }

    /// Read a file line by line, calling the callback for each line.
    /// 逐行读取文件，每行调用回调。
    fn builtin_read_file_lines(
        &mut self,
        path: &Value,
        callback: &Value,
    ) -> Result<Value, EvalError> {
        use std::io::BufRead;
        let path_str = match path {
            Value::String(s) => s.as_str(),
            _ => {
                return Err(EvalError::TypeError(
                    "readFileLines expects a String path".to_string(),
                ));
            }
        };

        // Canonicalize to resolve .. components and symlinks
        let canonical = std::path::Path::new(path_str)
            .canonicalize()
            .map_err(|e| EvalError::TypeError(format!("readFileLines: {e}")))?;

        let file = std::fs::File::open(&canonical)
            .map_err(|e| EvalError::TypeError(format!("readFileLines: {e}")))?;
        let reader = std::io::BufReader::new(file);

        let mut line_count: usize = 0;
        for line in reader.lines() {
            line_count += 1;
            if line_count > self.max_stream_lines {
                return Err(EvalError::TypeError(format!(
                    "readFileLines: exceeded max stream lines ({})",
                    self.max_stream_lines
                )));
            }
            let line = line.map_err(|e| EvalError::TypeError(format!("readFileLines: {e}")))?;
            self.apply(
                callback.clone(),
                vec![Value::String(std::rc::Rc::new(line))],
            )?;
        }

        Ok(Value::Unit)
    }

    /// Read a file line by line (typed Path variant).
    /// 逐行读取文件（Path 类型变体）。
    fn builtin_read_file_lines_path(
        &mut self,
        path: &Value,
        callback: &Value,
    ) -> Result<Value, EvalError> {
        use std::io::BufRead;
        let path = match path {
            Value::Path(p) => p.as_path(),
            _ => {
                return Err(EvalError::TypeError(
                    "readFileLinesPath expects a Path".to_string(),
                ));
            }
        };

        // Canonicalize to resolve .. components and symlinks
        let canonical = path
            .canonicalize()
            .map_err(|e| EvalError::TypeError(format!("readFileLinesPath: {e}")))?;

        let file = std::fs::File::open(&canonical)
            .map_err(|e| EvalError::TypeError(format!("readFileLinesPath: {e}")))?;
        let reader = std::io::BufReader::new(file);

        let mut line_count: usize = 0;
        for line in reader.lines() {
            line_count += 1;
            if line_count > self.max_stream_lines {
                return Err(EvalError::TypeError(format!(
                    "readFileLinesPath: exceeded max stream lines ({})",
                    self.max_stream_lines
                )));
            }
            let line = line.map_err(|e| EvalError::TypeError(format!("readFileLinesPath: {e}")))?;
            self.apply(
                callback.clone(),
                vec![Value::String(std::rc::Rc::new(line))],
            )?;
        }

        Ok(Value::Unit)
    }

    /// Run all deferred functions in reverse order.
    fn run_defers(&mut self) -> Result<(), EvalError> {
        while let Some(f) = self.defer_stack.pop() {
            self.apply(f, vec![])?;
        }
        Ok(())
    }

    fn builtin_event_map(&mut self, event: &Value, func: &Value) -> Result<Value, EvalError> {
        let source = match event {
            Value::Event(e) => Rc::clone(e),
            _ => {
                return Err(EvalError::TypeError(
                    "eventMap expects an Event".to_string(),
                ));
            }
        };
        Ok(Value::Event(Rc::new(EventValue {
            kind: EventKind::Mapped {
                source,
                func: func.clone(),
            },
        })))
    }

    fn builtin_event_filter(
        &mut self,
        event: &Value,
        predicate: &Value,
    ) -> Result<Value, EvalError> {
        let source = match event {
            Value::Event(e) => Rc::clone(e),
            _ => {
                return Err(EvalError::TypeError(
                    "eventFilter expects an Event".to_string(),
                ));
            }
        };
        Ok(Value::Event(Rc::new(EventValue {
            kind: EventKind::Filtered {
                source,
                predicate: predicate.clone(),
            },
        })))
    }

    /// Register a signal handler. Installs the OS signal handler on first registration
    /// and stores the callback for later dispatch by check_signals().
    /// 注册信号处理程序。首次注册时安装 OS 信号处理程序，并存储回调供 check_signals() 调度。
    fn builtin_on_signal(
        &mut self,
        signal_name: &Value,
        callback: &Value,
    ) -> Result<Value, EvalError> {
        let name = match signal_name {
            Value::String(s) => s.as_str(),
            _ => {
                return Err(EvalError::TypeError(
                    "io.onSignal: first arg must be a signal name (String)".to_string(),
                ));
            }
        };

        // Validate signal name
        let _idx = signal_index(name).ok_or_else(|| {
            EvalError::TypeError(format!(
                "io.onSignal: unknown signal '{}'. Supported: INT, TERM, HUP, USR1, USR2",
                name
            ))
        })?;

        // Validate callback is callable and accepts zero arguments
        // (signal handlers are dispatched with no arguments)
        match callback {
            Value::Builtin(b) => {
                if b.arity != 0 {
                    return Err(EvalError::TypeError(format!(
                        "io.onSignal: callback must accept 0 arguments, got {}",
                        b.arity
                    )));
                }
            }
            Value::Closure { params, .. } => {
                if !params.is_empty() {
                    return Err(EvalError::TypeError(format!(
                        "io.onSignal: callback must accept 0 arguments, got {}",
                        params.len()
                    )));
                }
            }
            Value::BuiltinFn(_, _) => {} // BuiltinFn arity is checked elsewhere
            _ => {
                return Err(EvalError::TypeError(
                    "io.onSignal: second arg must be a function".to_string(),
                ));
            }
        }

        // Install OS signal handler only once per signal name
        if !self.signal_handlers.contains_key(name) {
            if let Err(e) = install_signal_handler(name) {
                return Err(EvalError::TypeError(format!("io.onSignal: {e}")));
            }
        }

        // Store the handler callback (last registration wins)
        self.signal_handlers
            .insert(name.to_string(), callback.clone());

        Ok(Value::Unit)
    }

    /// Check for pending signals and dispatch handlers. Called at safe points
    /// in the evaluation loop. Returns the signal name if a handler was invoked.
    /// 检查待处理的信号并分派处理程序。在求值循环的安全点调用。
    fn check_signals(&mut self) -> Result<(), EvalError> {
        // Collect the names of registered signals that have pending flags.
        // Avoid cloning the HashMap — only clone handler Values that need dispatch.
        let mut pending: Vec<String> = Vec::new();
        for name in self.signal_handlers.keys() {
            if let Some(idx) = signal_index(name) {
                if SIGNAL_FLAGS[idx].load(Ordering::Acquire) {
                    pending.push(name.clone());
                }
            }
        }
        // Dispatch handlers for pending signals (outside the borrow of signal_handlers)
        for name in pending {
            // Clear the flag and dispatch
            if let Some(idx) = signal_index(&name) {
                SIGNAL_FLAGS[idx].store(false, Ordering::Release);
            }
            if let Some(handler) = self.signal_handlers.get(&name) {
                self.apply(handler.clone(), vec![])?;
            }
        }
        Ok(())
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
            defer_stack: Vec::new(),
            extra_builtins: self.extra_builtins.clone(),
            signal_handlers: self.signal_handlers.clone(),
            max_stream_lines: self.max_stream_lines,
            max_stdin_bytes: self.max_stdin_bytes,
            max_intermediate_buffer: self.max_intermediate_buffer,
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
                if command.has_effect_config() {
                    format!(
                        "<command:{} {} arg(s), configured>",
                        command.program(),
                        command.args().len()
                    )
                } else {
                    format!(
                        "<command:{} {} arg(s)>",
                        command.program(),
                        command.args().len()
                    )
                }
            }
            Value::Pipeline(pipeline) => {
                format!("<pipeline:{} command(s)>", pipeline.commands().len())
            }
            Value::Redirect(redirect) => {
                format!("<redirect:{}:path>", redirect.stream_name())
            }
            Value::Task(task) => {
                format!(
                    "<task:{}->{}>",
                    task.target().kind_name(),
                    task.output().type_name()
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
            Value::Event(_) => "Event(..)".to_string(),
            Value::Live(_) => "Live(..)".to_string(),
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
            effectful: false,
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
