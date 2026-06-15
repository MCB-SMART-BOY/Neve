//! Runtime values.
//! 运行时值。
//!
//! This module defines all value types that can exist during Neve program execution.
//! 本模块定义了 Neve 程序执行过程中可能存在的所有值类型。
//!
//! Note: AST compat types (AstEnv, AstClosure) are used here as internal
//! implementation details. External callers should prefer HIR evaluator.

#![allow(deprecated)]
use crate::Environment;
use crate::ast_eval::AstEnv;
use neve_common::{Int, int_to_f64};
use neve_hir::{Expr, Param};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;
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
    /// The inner state of the thunk, wrapped in `Rc<RefCell>` for shared mutable access.
    /// Thunk 的内部状态，用 `Rc<RefCell>` 包装以实现共享可变访问。
    inner: Rc<RefCell<ThunkState>>,
}

/// The state of a thunk.
/// Thunk 的状态。
#[derive(Clone)]
pub enum ThunkState {
    /// Unevaluated thunk with AST expression.
    /// 带有 AST 表达式的未求值 thunk。
    AstUnevaluated {
        expr: neve_syntax::Expr,
        env: Rc<crate::ast_eval::AstEnv>,
    },
    /// Unevaluated thunk with HIR expression.
    /// 带有 HIR 表达式的未求值 thunk。
    HirUnevaluated { expr: Expr, env: Environment },
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
    pub fn new_ast(expr: neve_syntax::Expr, env: Rc<crate::ast_eval::AstEnv>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ThunkState::AstUnevaluated { expr, env })),
        }
    }

    /// Create a new unevaluated thunk from a HIR expression.
    /// 从 HIR 表达式创建新的未求值 thunk。
    pub fn new_hir(expr: Expr, env: Environment) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ThunkState::HirUnevaluated { expr, env })),
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
            ThunkState::AstUnevaluated { .. } | ThunkState::HirUnevaluated { .. } => {
                write!(f, "<thunk:unevaluated>")
            }
            ThunkState::Evaluating => write!(f, "<thunk:evaluating>"),
            ThunkState::Evaluated(v) => write!(f, "<thunk:{:?}>", v),
        }
    }
}

/// Opaque command runtime object.
/// 不透明的命令运行时对象。
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandValue {
    program: Rc<String>,
    args: Rc<Vec<String>>,
    cwd: Option<Rc<String>>,
    stdin: Option<Rc<String>>,
    env: Rc<HashMap<String, String>>,
    redirects: Rc<Vec<RedirectValue>>,
}

impl CommandValue {
    /// Create a new command runtime object.
    /// 创建新的命令运行时对象。
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self::new_with_options_and_redirects(program, args, None, None, HashMap::new(), Vec::new())
    }

    /// Create a configured command runtime object.
    /// 创建带配置的命令运行时对象。
    pub fn new_with_options(
        program: impl Into<String>,
        args: Vec<String>,
        cwd: Option<String>,
        stdin: Option<String>,
        env: HashMap<String, String>,
    ) -> Self {
        Self::new_with_options_and_redirects(program, args, cwd, stdin, env, Vec::new())
    }

    /// Create a configured command runtime object with embedded redirects.
    /// 创建带配置和内嵌重定向的命令运行时对象。
    pub fn new_with_options_and_redirects(
        program: impl Into<String>,
        args: Vec<String>,
        cwd: Option<String>,
        stdin: Option<String>,
        env: HashMap<String, String>,
        redirects: Vec<RedirectValue>,
    ) -> Self {
        Self {
            program: Rc::new(program.into()),
            args: Rc::new(args),
            cwd: cwd.map(Rc::new),
            stdin: stdin.map(Rc::new),
            env: Rc::new(env),
            redirects: Rc::new(redirects),
        }
    }

    pub fn program(&self) -> &str {
        self.program.as_ref()
    }

    pub fn args(&self) -> &[String] {
        self.args.as_ref()
    }

    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref().map(|value| value.as_str())
    }

    pub fn stdin(&self) -> Option<&str> {
        self.stdin.as_deref().map(|value| value.as_str())
    }

    pub fn env(&self) -> &HashMap<String, String> {
        self.env.as_ref()
    }

    pub fn redirects(&self) -> &[RedirectValue] {
        self.redirects.as_ref()
    }

    pub fn has_embedded_redirects(&self) -> bool {
        !self.redirects.is_empty()
    }

    pub fn has_effect_config(&self) -> bool {
        self.cwd.is_some()
            || self.stdin.is_some()
            || !self.env.is_empty()
            || self.has_embedded_redirects()
    }
}

/// Opaque process-result runtime object.
/// 不透明的进程结果运行时对象。
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessResultValue {
    code: i32,
    success: bool,
    stdout: Rc<String>,
    stderr: Rc<String>,
}

impl ProcessResultValue {
    /// Create a new process-result runtime object.
    /// 创建新的进程结果运行时对象。
    pub fn new(
        code: i32,
        success: bool,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
    ) -> Self {
        Self {
            code,
            success,
            stdout: Rc::new(stdout.into()),
            stderr: Rc::new(stderr.into()),
        }
    }

    pub fn code(&self) -> i32 {
        self.code
    }

    pub fn is_success(&self) -> bool {
        self.success
    }

    pub fn stdout(&self) -> &str {
        self.stdout.as_ref()
    }

    pub fn stderr(&self) -> &str {
        self.stderr.as_ref()
    }
}

/// Opaque pipeline runtime object.
/// 不透明的管道运行时对象。
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipelineValue {
    commands: Rc<Vec<Rc<CommandValue>>>,
    redirects: Rc<Vec<RedirectValue>>,
}

impl PipelineValue {
    /// Create a new pipeline runtime object.
    /// 创建新的管道运行时对象。
    pub fn new(commands: Vec<Rc<CommandValue>>) -> Self {
        Self::new_with_redirects(commands, Vec::new())
    }

    /// Create a new pipeline runtime object with embedded boundary redirects.
    /// 创建带内嵌边界重定向的新管道运行时对象。
    pub fn new_with_redirects(
        commands: Vec<Rc<CommandValue>>,
        redirects: Vec<RedirectValue>,
    ) -> Self {
        Self {
            commands: Rc::new(commands),
            redirects: Rc::new(redirects),
        }
    }

    pub fn commands(&self) -> &[Rc<CommandValue>] {
        self.commands.as_ref()
    }

    pub fn redirects(&self) -> &[RedirectValue] {
        self.redirects.as_ref()
    }

    pub fn has_embedded_redirects(&self) -> bool {
        !self.redirects.is_empty()
    }
}

/// Redirect kind for the minimal runtime object bridge.
/// 最小重定向运行时对象桥接使用的重定向种类。
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RedirectStream {
    Stdout,
    Stderr,
    Stdin,
}

/// Opaque redirect runtime object.
/// 不透明的重定向运行时对象。
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedirectValue {
    stream: RedirectStream,
    path: Rc<PathBuf>,
}

impl RedirectValue {
    /// Create a stdout-to-path redirect runtime object.
    /// 创建 stdout 到路径的重定向运行时对象。
    pub fn stdout_path(path: impl Into<PathBuf>) -> Self {
        Self {
            stream: RedirectStream::Stdout,
            path: Rc::new(path.into()),
        }
    }

    /// Create a stderr-to-path redirect runtime object.
    /// 创建 stderr 到路径的重定向运行时对象。
    pub fn stderr_path(path: impl Into<PathBuf>) -> Self {
        Self {
            stream: RedirectStream::Stderr,
            path: Rc::new(path.into()),
        }
    }

    /// Create a stdin-from-path redirect runtime object.
    /// 创建 stdin 从路径读取的重定向运行时对象。
    pub fn stdin_path(path: impl Into<PathBuf>) -> Self {
        Self {
            stream: RedirectStream::Stdin,
            path: Rc::new(path.into()),
        }
    }

    pub fn stream(&self) -> &RedirectStream {
        &self.stream
    }

    pub fn stream_name(&self) -> &'static str {
        match self.stream {
            RedirectStream::Stdout => "stdout",
            RedirectStream::Stderr => "stderr",
            RedirectStream::Stdin => "stdin",
        }
    }

    pub fn path(&self) -> &PathBuf {
        self.path.as_ref()
    }
}

/// Task output kind for the minimal runtime object bridge.
/// 最小任务运行时对象桥接使用的任务输出种类。
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskOutputKind {
    ProcessResult,
}

impl TaskOutputKind {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::ProcessResult => "ProcessResult",
        }
    }
}

/// Task target for the minimal runtime object bridge.
/// 最小任务运行时对象桥接使用的任务目标。
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskTargetValue {
    Command(Rc<CommandValue>),
    Pipeline(Rc<PipelineValue>),
}

impl TaskTargetValue {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Command(_) => "command",
            Self::Pipeline(_) => "pipeline",
        }
    }
}

/// Opaque task runtime object.
/// 不透明的任务运行时对象。
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskValue {
    output: TaskOutputKind,
    target: TaskTargetValue,
}

impl TaskValue {
    /// Create a command-backed task that will produce a process result.
    /// 创建一个基于命令且产出进程结果的任务对象。
    pub fn command_process_result(command: Rc<CommandValue>) -> Self {
        Self {
            output: TaskOutputKind::ProcessResult,
            target: TaskTargetValue::Command(command),
        }
    }

    /// Create a pipeline-backed task that will produce a process result.
    /// 创建一个基于管道且产出进程结果的任务对象。
    pub fn pipeline_process_result(pipeline: Rc<PipelineValue>) -> Self {
        Self {
            output: TaskOutputKind::ProcessResult,
            target: TaskTargetValue::Pipeline(pipeline),
        }
    }

    pub fn output(&self) -> &TaskOutputKind {
        &self.output
    }

    pub fn target(&self) -> &TaskTargetValue {
        &self.target
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
    Int(Int),
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

    // ===== Runtime object skeletons 运行时对象骨架 =====
    /// Path runtime object / Path 运行时对象
    Path(Rc<PathBuf>),
    /// Bytes runtime object / Bytes 运行时对象
    Bytes(Rc<Vec<u8>>),
    /// Command runtime object / Command 运行时对象
    Command(Rc<CommandValue>),
    /// Pipeline runtime object / Pipeline 运行时对象
    Pipeline(Rc<PipelineValue>),
    /// Redirect runtime object / Redirect 运行时对象
    Redirect(Rc<RedirectValue>),
    /// Task runtime object / Task 运行时对象
    Task(Rc<TaskValue>),
    /// Event runtime object / Event 运行时对象
    Event(Rc<EventValue>),
    /// Live reactive value
    Live(Rc<LiveValue>),
    /// Stream runtime object / Stream 运行时对象
    Stream(Rc<StreamValue>),
    /// ProcessResult runtime object / ProcessResult 运行时对象
    ProcessResult(Rc<ProcessResultValue>),

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
    /// Enum/variant constructor / 枚举变体构造器
    VariantCtor { name: String, arity: usize },

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
            Value::Live(_) => write!(f, "Live(..)"),
            Value::Stream(s) => write!(f, "{:?}", s),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Char(c) => write!(f, "'{}'", c),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Unit => write!(f, "()"),
            Value::Path(path) => write!(f, "Path({path:?})"),
            Value::Bytes(bytes) => write!(f, "Bytes({} bytes)", bytes.len()),
            Value::Command(command) => {
                write!(
                    f,
                    "Command(program={:?}, args={:?}",
                    command.program(),
                    command.args()
                )?;
                if let Some(cwd) = command.cwd() {
                    write!(f, ", cwd={cwd:?}")?;
                }
                if let Some(stdin) = command.stdin() {
                    write!(f, ", stdin={stdin:?}")?;
                }
                if !command.env().is_empty() {
                    let mut keys: Vec<&String> = command.env().keys().collect();
                    keys.sort();
                    write!(f, ", env={{")?;
                    for (idx, key) in keys.into_iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        let value = command.env().get(key).expect("env key exists");
                        write!(f, "{key:?}: {value:?}")?;
                    }
                    write!(f, "}}")?;
                }
                if command.has_embedded_redirects() {
                    write!(f, ", redirects=[")?;
                    for (idx, redirect) in command.redirects().iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        write!(
                            f,
                            "Redirect(stream={}, path={:?})",
                            redirect.stream_name(),
                            redirect.path()
                        )?;
                    }
                    write!(f, "]")?;
                }
                write!(f, ")")
            }
            Value::Pipeline(pipeline) => {
                write!(f, "Pipeline(commands=[")?;
                for (idx, command) in pipeline.commands().iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    write!(
                        f,
                        "Command(program={:?}, args={:?})",
                        command.program(),
                        command.args()
                    )?;
                }
                if pipeline.has_embedded_redirects() {
                    write!(f, "], redirects=[")?;
                    for (idx, redirect) in pipeline.redirects().iter().enumerate() {
                        if idx > 0 {
                            write!(f, ", ")?;
                        }
                        write!(
                            f,
                            "Redirect(stream={}, path={:?})",
                            redirect.stream_name(),
                            redirect.path()
                        )?;
                    }
                    write!(f, "]")?;
                }
                write!(f, "])")
            }
            Value::Redirect(redirect) => write!(
                f,
                "Redirect(stream={}, path={:?})",
                redirect.stream_name(),
                redirect.path()
            ),
            Value::Task(task) => write!(
                f,
                "{}",
                match task.target() {
                    TaskTargetValue::Command(command) => format!(
                        "Task(output={}, command=Command(program={:?}, args={:?}))",
                        task.output().type_name(),
                        command.program(),
                        command.args()
                    ),
                    TaskTargetValue::Pipeline(pipeline) => format!(
                        "Task(output={}, pipeline=Pipeline(commands={} command(s)))",
                        task.output().type_name(),
                        pipeline.commands().len()
                    ),
                }
            ),
            Value::ProcessResult(result) => write!(
                f,
                "ProcessResult(code={}, success={}, stdout={:?}, stderr={:?})",
                result.code(),
                result.is_success(),
                result.stdout(),
                result.stderr()
            ),
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
            Value::VariantCtor { name, arity } => write!(f, "<variant:{}:{}>", name, arity),
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
            Value::Event(e) => write!(f, "Event({:?})", e.kind),
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
            (Value::Path(a), Value::Path(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Command(a), Value::Command(b)) => a == b,
            (Value::Pipeline(a), Value::Pipeline(b)) => a == b,
            (Value::Redirect(a), Value::Redirect(b)) => a == b,
            (Value::Task(a), Value::Task(b)) => a == b,
            (Value::ProcessResult(a), Value::ProcessResult(b)) => a == b,
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
            // Streams are never equal (identity-based)
            // 流永远不相等（基于标识）
            (Value::Stream(_), Value::Stream(_)) => false,
            // Closures and builtins are never equal
            // 闭包和内置函数永远不相等
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Value {
    /// Construct an opaque command runtime object.
    /// 构造一个不透明的命令运行时对象。
    #[cfg(test)]
    pub(crate) fn command_object(program: impl Into<String>, args: Vec<String>) -> Self {
        Self::Command(Rc::new(CommandValue::new(program, args)))
    }

    /// Construct an opaque configured command runtime object.
    /// 构造一个不透明的带配置命令运行时对象。
    #[cfg(test)]
    pub(crate) fn command_object_with_options(
        program: impl Into<String>,
        args: Vec<String>,
        cwd: Option<String>,
        stdin: Option<String>,
        env: HashMap<String, String>,
    ) -> Self {
        Self::Command(Rc::new(CommandValue::new_with_options(
            program, args, cwd, stdin, env,
        )))
    }

    /// Construct an opaque configured command runtime object with embedded redirects.
    /// 构造一个带内嵌重定向的不透明命令运行时对象。
    #[cfg(test)]
    pub(crate) fn command_object_with_options_and_redirects(
        program: impl Into<String>,
        args: Vec<String>,
        cwd: Option<String>,
        stdin: Option<String>,
        env: HashMap<String, String>,
        redirects: Vec<RedirectValue>,
    ) -> Self {
        Self::Command(Rc::new(CommandValue::new_with_options_and_redirects(
            program, args, cwd, stdin, env, redirects,
        )))
    }

    /// Construct an opaque pipeline runtime object.
    /// 构造一个不透明的管道运行时对象。
    #[cfg(test)]
    pub(crate) fn pipeline_object(commands: Vec<CommandValue>) -> Self {
        Self::Pipeline(Rc::new(PipelineValue::new(
            commands.into_iter().map(Rc::new).collect(),
        )))
    }

    /// Construct an opaque pipeline runtime object with embedded boundary redirects.
    /// 构造一个带内嵌边界重定向的不透明管道运行时对象。
    #[cfg(test)]
    pub(crate) fn pipeline_object_with_redirects(
        commands: Vec<CommandValue>,
        redirects: Vec<RedirectValue>,
    ) -> Self {
        Self::Pipeline(Rc::new(PipelineValue::new_with_redirects(
            commands.into_iter().map(Rc::new).collect(),
            redirects,
        )))
    }

    /// Construct an opaque redirect runtime object.
    /// 构造一个不透明的重定向运行时对象。
    #[cfg(test)]
    pub(crate) fn redirect_stdout_path_object(path: impl Into<PathBuf>) -> Self {
        Self::Redirect(Rc::new(RedirectValue::stdout_path(path)))
    }

    /// Construct an opaque stderr-to-path redirect runtime object.
    /// 构造一个不透明的 stderr 到路径重定向运行时对象。
    #[cfg(test)]
    pub(crate) fn redirect_stderr_path_object(path: impl Into<PathBuf>) -> Self {
        Self::Redirect(Rc::new(RedirectValue::stderr_path(path)))
    }

    /// Construct an opaque stdin-from-path redirect runtime object.
    /// 构造一个不透明的 stdin 从路径读取重定向运行时对象。
    #[cfg(test)]
    pub(crate) fn redirect_stdin_path_object(path: impl Into<PathBuf>) -> Self {
        Self::Redirect(Rc::new(RedirectValue::stdin_path(path)))
    }

    /// Construct an opaque process-result runtime object.
    /// 构造一个不透明的进程结果运行时对象。
    #[cfg(test)]
    pub(crate) fn process_result_object(
        code: i32,
        success: bool,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
    ) -> Self {
        Self::ProcessResult(Rc::new(ProcessResultValue::new(
            code, success, stdout, stderr,
        )))
    }

    /// Construct an opaque task runtime object backed by a command.
    /// 构造一个基于命令的不透明任务运行时对象。
    #[cfg(test)]
    pub(crate) fn task_command_process_result_object(command: CommandValue) -> Self {
        Self::Task(Rc::new(TaskValue::command_process_result(Rc::new(command))))
    }

    /// Construct an opaque task runtime object backed by a pipeline.
    /// 构造一个基于管道的不透明任务运行时对象。
    #[cfg(test)]
    pub(crate) fn task_pipeline_process_result_object(pipeline: PipelineValue) -> Self {
        Self::Task(Rc::new(TaskValue::pipeline_process_result(Rc::new(
            pipeline,
        ))))
    }
}

/// An event source — produces a stream of values over time.
#[derive(Debug, Clone)]
pub struct EventValue {
    pub kind: EventKind,
}

#[derive(Debug, Clone)]
pub enum EventKind {
    Timer {
        interval_ms: u64,
    },
    FileWatch {
        path: std::path::PathBuf,
    },
    /// Chained: apply a function to each value from the source event.
    Mapped {
        source: Rc<EventValue>,
        /// The transformation function (a -> b).
        func: Value,
    },
    /// Chained: only pass through values that satisfy a predicate.
    Filtered {
        source: Rc<EventValue>,
        /// The filter predicate (a -> Bool).
        predicate: Value,
    },
}

/// A live reactive value that updates when its source event fires.
#[derive(Debug, Clone)]
pub struct LiveValue {
    pub event: Rc<EventValue>,
    pub current: Rc<std::cell::RefCell<Option<Value>>>,
    pub cancelled: Rc<std::cell::Cell<bool>>,
}

/// Opaque stream runtime object.
/// 不透明的流运行时对象。
#[derive(Clone)]
pub struct StreamValue {
    pub(crate) inner: Rc<RefCell<StreamState>>,
}

/// Type alias for the line-oriented stream channel receiver.
pub type LinesChannelRx = std::sync::mpsc::Receiver<Result<String, String>>;

/// Type alias for the byte-oriented stream channel receiver.
pub type BytesChannelRx = std::sync::mpsc::Receiver<Result<Vec<u8>, String>>;

/// Transform applied to a wrapped stream.
/// 应用于包装流的变换。
#[derive(Clone)]
pub enum StreamTransform {
    /// Skip the first N elements, then pass through.
    Drop { remaining: usize },
    /// Pass through the first N elements, then stop.
    Take { remaining: usize },
    /// Apply a function to each element.
    Map { func: Value },
    /// Keep only elements for which the predicate returns true.
    Filter { predicate: Value },
    /// Timeout: stop returning elements after a deadline.
    Timeout { deadline: std::time::Instant },
}

/// The internal state of a stream.
/// 流的内部状态。
#[derive(Clone)]
pub enum StreamState {
    /// Iterator-based: wraps an eager iterator (for list sources).
    Iterator {
        iter: Rc<RefCell<Box<dyn Iterator<Item = Value>>>>,
    },
    /// Channel-based (lines): producer thread sends `String` lines.
    LinesChannel {
        rx: Rc<RefCell<LinesChannelRx>>,
        cancelled: Rc<std::sync::atomic::AtomicBool>,
    },
    /// Channel-based (bytes): producer thread sends `Vec<u8>` chunks.
    BytesChannel {
        rx: Rc<RefCell<BytesChannelRx>>,
        cancelled: Rc<std::sync::atomic::AtomicBool>,
    },
    /// Wrapped: transforms an upstream stream (pure, lazy).
    /// Used by streamTake, streamDrop, streamMap, streamFilter.
    Wrapped {
        source: Rc<StreamValue>,
        transform: Box<StreamTransform>,
    },
    /// Exhausted or consumed.
    Done,
}

impl StreamValue {
    /// Create a stream from a list of values (iterator-based).
    pub fn from_list(values: Vec<Value>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StreamState::Iterator {
                iter: Rc::new(RefCell::new(Box::new(values.into_iter()))),
            })),
        }
    }

    /// Create a stream from a lines channel (producer sends `String`).
    pub fn from_lines_channel(rx: LinesChannelRx) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StreamState::LinesChannel {
                rx: Rc::new(RefCell::new(rx)),
                cancelled: Rc::new(std::sync::atomic::AtomicBool::new(false)),
            })),
        }
    }

    /// Create a stream from a bytes channel (producer sends `Vec<u8>`).
    pub fn from_bytes_channel(rx: BytesChannelRx) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StreamState::BytesChannel {
                rx: Rc::new(RefCell::new(rx)),
                cancelled: Rc::new(std::sync::atomic::AtomicBool::new(false)),
            })),
        }
    }

    /// Create a wrapped stream that transforms an upstream source.
    pub fn from_wrapped(source: Rc<StreamValue>, transform: StreamTransform) -> Self {
        Self {
            inner: Rc::new(RefCell::new(StreamState::Wrapped {
                source,
                transform: Box::new(transform),
            })),
        }
    }

    /// Get the next element from the stream. Returns None at end or on cancel.
    pub fn next(&self) -> Result<Option<Value>, String> {
        let mut state = self.inner.borrow_mut();
        match &mut *state {
            StreamState::Iterator { iter } => Ok(iter.borrow_mut().next()),
            StreamState::LinesChannel { rx: _, cancelled }
            | StreamState::BytesChannel { rx: _, cancelled } => {
                if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                    *state = StreamState::Done;
                    return Ok(None);
                }
                // Drop the borrow before recv (recv may block)
                drop(state);
                let mut inner_guard = self.inner.borrow_mut();
                match &mut *inner_guard {
                    StreamState::LinesChannel { rx, cancelled } => {
                        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                            return Ok(None);
                        }
                        match rx.borrow_mut().recv() {
                            Ok(Ok(s)) => Ok(Some(Value::String(Rc::new(s)))),
                            Ok(Err(e)) => Err(e),
                            Err(_) => Ok(None), // Channel closed
                        }
                    }
                    StreamState::BytesChannel { rx, cancelled } => {
                        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                            return Ok(None);
                        }
                        match rx.borrow_mut().recv() {
                            Ok(Ok(v)) => Ok(Some(Value::Bytes(Rc::new(v)))),
                            Ok(Err(e)) => Err(e),
                            Err(_) => Ok(None), // Channel closed
                        }
                    }
                    _ => Ok(None),
                }
            }
            StreamState::Wrapped { source, transform } => {
                // Clone everything we need, then drop the RefCell borrow.
                let source = source.clone();
                let xf = transform.clone();
                drop(state);

                match &*xf {
                    StreamTransform::Take { remaining } if *remaining == 0 => {
                        *self.inner.borrow_mut() = StreamState::Done;
                        Ok(None)
                    }
                    StreamTransform::Take { remaining } => {
                        match source.next() {
                            Ok(Some(v)) => {
                                // Update remaining counter (drop guard before returning)
                                {
                                    let mut s = self.inner.borrow_mut();
                                    if let StreamState::Wrapped { transform: t, .. } = &mut *s
                                        && let StreamTransform::Take { remaining: r } = &mut **t
                                    {
                                        *r = *remaining - 1;
                                    }
                                }
                                Ok(Some(v))
                            }
                            other => other,
                        }
                    }
                    StreamTransform::Drop { remaining } if *remaining == 0 => {
                        // All skipped, pass through directly
                        source.next()
                    }
                    StreamTransform::Drop { remaining } => {
                        // Skip one element from source
                        match source.next() {
                            Ok(Some(_)) => {
                                // Decrement counter (drop guard before recursion)
                                {
                                    let mut s = self.inner.borrow_mut();
                                    if let StreamState::Wrapped { transform: t, .. } = &mut *s
                                        && let StreamTransform::Drop { remaining: r } = &mut **t
                                    {
                                        *r = *remaining - 1;
                                    }
                                }
                                // Recurse to process next element
                                self.next()
                            }
                            other => other,
                        }
                    }
                    StreamTransform::Map { func } => match source.next() {
                        Ok(Some(v)) => Self::try_apply_function(func, v).map(Some),
                        other => other,
                    },
                    StreamTransform::Filter { predicate } => {
                        // Loop until predicate matches or source ends
                        loop {
                            match source.next() {
                                Ok(Some(v)) => {
                                    match Self::try_apply_function(predicate, v.clone()) {
                                        Ok(Value::Bool(true)) => return Ok(Some(v)),
                                        Ok(_) => continue, // skipped
                                        Err(e) => return Err(e),
                                    }
                                }
                                other => return other,
                            }
                        }
                    }
                    StreamTransform::Timeout { deadline } => {
                        if std::time::Instant::now() >= *deadline {
                            return Ok(None); // timeout expired
                        }
                        source.next()
                    }
                }
            }
            StreamState::Done => Ok(None),
        }
    }

    /// Try to apply a function value to a single argument.
    /// Supports Builtin, BuiltinFn, and Closure (since we're on the evaluator thread).
    pub fn try_apply_function(func: &Value, arg: Value) -> Result<Value, String> {
        match func {
            Value::Builtin(b) => (b.func)(&[arg]),
            Value::BuiltinFn(_, f) => f(vec![arg]),
            Value::Closure { .. } | Value::AstClosure(_) => Err(
                "stream transform: closures require evaluator context (not yet supported)"
                    .to_string(),
            ),
            _ => Err(format!(
                "stream transform: cannot apply {:?} as a function",
                func
            )),
        }
    }

    /// Cancel the stream (signal producer to stop).
    pub fn cancel(&self) {
        match &*self.inner.borrow() {
            StreamState::LinesChannel { cancelled, .. }
            | StreamState::BytesChannel { cancelled, .. } => {
                cancelled.store(true, std::sync::atomic::Ordering::Release);
            }
            StreamState::Wrapped { source, .. } => {
                // Cancel the upstream source first
                source.cancel();
            }
            _ => {}
        }
        // Mark this stream as done
        *self.inner.borrow_mut() = StreamState::Done;
    }
}

impl fmt::Debug for StreamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.inner.borrow() {
            StreamState::Iterator { .. } => write!(f, "<stream:iterator>"),
            StreamState::LinesChannel { .. } => write!(f, "<stream:lines>"),
            StreamState::BytesChannel { .. } => write!(f, "<stream:bytes>"),
            StreamState::Wrapped { transform, .. } => match &**transform {
                StreamTransform::Take { remaining } => {
                    write!(f, "<stream:take({})>", remaining)
                }
                StreamTransform::Drop { remaining } => {
                    write!(f, "<stream:drop({})>", remaining)
                }
                StreamTransform::Map { .. } => write!(f, "<stream:map>"),
                StreamTransform::Filter { .. } => write!(f, "<stream:filter>"),
                StreamTransform::Timeout { .. } => write!(f, "<stream:timeout>"),
            },
            StreamState::Done => write!(f, "<stream:done>"),
        }
    }
}

impl PartialEq for StreamValue {
    fn eq(&self, _other: &Self) -> bool {
        false // Streams are never equal (identity-based)
    }
}

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
    pub fn as_int(&self) -> Option<&Int> {
        match self {
            Value::Int(n) => Some(n),
            _ => None,
        }
    }

    /// Try to get as float.
    /// 尝试获取浮点数值。
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(n) => int_to_f64(n),
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

#[derive(Debug, Clone)]
enum KeyState {
    InProgress(usize),
    Done(String),
}

#[derive(Debug, Default)]
struct KeyCtx {
    next_id: usize,
    nodes: HashMap<usize, KeyState>,
}

impl KeyCtx {
    fn new() -> Self {
        Self {
            next_id: 0,
            nodes: HashMap::new(),
        }
    }

    fn key_for_ptr<F>(&mut self, ptr: usize, build: F) -> String
    where
        F: FnOnce(&mut Self) -> String,
    {
        if let Some(state) = self.nodes.get(&ptr) {
            return match state {
                KeyState::InProgress(id) => format!("<cycle#{id}>"),
                KeyState::Done(value) => value.clone(),
            };
        }

        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(ptr, KeyState::InProgress(id));
        let value = build(self);
        self.nodes.insert(ptr, KeyState::Done(value.clone()));
        value
    }

    fn value_key(&mut self, value: &Value) -> String {
        match value {
            Value::Live(_) => "Live(..)".to_string(),
            Value::Stream(s) => format!("Stream({:?})", s),
            Value::Int(n) => format!("Int({n})"),
            Value::Float(f) => format!("Float({})", canonical_float(*f)),
            Value::Bool(b) => format!("Bool({b})"),
            Value::Char(c) => format!("Char('{}')", escape_char(*c)),
            Value::String(s) => format!("String(\"{}\")", escape_string(s)),
            Value::Unit => "Unit".to_string(),
            Value::Path(path) => {
                format!("Path(\"{}\")", escape_string(&path.to_string_lossy()))
            }
            Value::Bytes(bytes) => format!("Bytes({})", hex_bytes(bytes)),
            Value::Command(command) => {
                let args = command
                    .args()
                    .iter()
                    .map(|arg| format!("\"{}\"", escape_string(arg)))
                    .collect::<Vec<_>>()
                    .join(",");
                let cwd = match command.cwd() {
                    Some(cwd) => format!("Some(\"{}\")", escape_string(cwd)),
                    None => "None".to_string(),
                };
                let stdin = match command.stdin() {
                    Some(stdin) => format!("Some(\"{}\")", escape_string(stdin)),
                    None => "None".to_string(),
                };
                let mut env_keys: Vec<&String> = command.env().keys().collect();
                env_keys.sort();
                let env = env_keys
                    .into_iter()
                    .map(|key| {
                        let value = command.env().get(key).expect("env key exists");
                        format!("\"{}\"=>\"{}\"", escape_string(key), escape_string(value))
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let redirects = if command.has_embedded_redirects() {
                    let redirects = command
                        .redirects()
                        .iter()
                        .map(|redirect| {
                            format!(
                                "Redirect{{stream={},path=\"{}\"}}",
                                redirect.stream_name(),
                                escape_string(&redirect.path().to_string_lossy())
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    format!(",redirects=[{}]", redirects)
                } else {
                    String::new()
                };
                format!(
                    "Command{{program=\"{}\",args=[{}],cwd={},stdin={},env={{{}}}{}}}",
                    escape_string(command.program()),
                    args,
                    cwd,
                    stdin,
                    env,
                    redirects
                )
            }
            Value::Pipeline(pipeline) => {
                let commands = pipeline
                    .commands()
                    .iter()
                    .map(|command| {
                        let command_value = Value::Command(Rc::clone(command));
                        self.value_key(&command_value)
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let redirects = if pipeline.has_embedded_redirects() {
                    let redirects = pipeline
                        .redirects()
                        .iter()
                        .map(|redirect| {
                            format!(
                                "Redirect{{stream={},path=\"{}\"}}",
                                redirect.stream_name(),
                                escape_string(&redirect.path().to_string_lossy())
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    format!(",redirects=[{}]", redirects)
                } else {
                    String::new()
                };
                format!("Pipeline{{commands=[{}]{}}}", commands, redirects)
            }
            Value::Redirect(redirect) => format!(
                "Redirect{{stream={},path=\"{}\"}}",
                redirect.stream_name(),
                escape_string(&redirect.path().to_string_lossy())
            ),
            Value::Task(task) => {
                format!(
                    "Task{{output={},{}={}}}",
                    task.output().type_name(),
                    task.target().kind_name(),
                    match task.target() {
                        TaskTargetValue::Command(command) => {
                            let command_value = Value::Command(Rc::clone(command));
                            self.value_key(&command_value)
                        }
                        TaskTargetValue::Pipeline(pipeline) => {
                            let pipeline_value = Value::Pipeline(Rc::clone(pipeline));
                            self.value_key(&pipeline_value)
                        }
                    }
                )
            }
            Value::ProcessResult(result) => format!(
                "ProcessResult{{code={},success={},stdout=\"{}\",stderr=\"{}\"}}",
                result.code(),
                result.is_success(),
                escape_string(result.stdout()),
                escape_string(result.stderr())
            ),
            Value::List(items) => {
                let ptr = Rc::as_ptr(items) as usize;
                self.key_for_ptr(ptr, |ctx| {
                    let parts: Vec<String> = items.iter().map(|v| ctx.value_key(v)).collect();
                    format!("List[{}]", parts.join(","))
                })
            }
            Value::Tuple(items) => {
                let ptr = Rc::as_ptr(items) as usize;
                self.key_for_ptr(ptr, |ctx| {
                    let parts: Vec<String> = items.iter().map(|v| ctx.value_key(v)).collect();
                    format!("Tuple({})", parts.join(","))
                })
            }
            Value::Record(fields) => {
                let ptr = Rc::as_ptr(fields) as usize;
                self.key_for_ptr(ptr, |ctx| {
                    let mut keys: Vec<&String> = fields.keys().collect();
                    keys.sort();
                    let parts: Vec<String> = keys
                        .into_iter()
                        .map(|k| {
                            let v = fields.get(k).expect("record key exists");
                            format!("{}={}", escape_string(k), ctx.value_key(v))
                        })
                        .collect();
                    format!("Record{{{}}}", parts.join(","))
                })
            }
            Value::Map(map) => {
                let ptr = Rc::as_ptr(map) as usize;
                self.key_for_ptr(ptr, |ctx| {
                    let mut keys: Vec<&String> = map.keys().collect();
                    keys.sort();
                    let parts: Vec<String> = keys
                        .into_iter()
                        .map(|k| {
                            let v = map.get(k).expect("map key exists");
                            format!("{}=>{}", escape_string(k), ctx.value_key(v))
                        })
                        .collect();
                    format!("Map{{{}}}", parts.join(","))
                })
            }
            Value::Set(set) => {
                let ptr = Rc::as_ptr(set) as usize;
                self.key_for_ptr(ptr, |_ctx| {
                    let mut items: Vec<&String> = set.iter().collect();
                    items.sort();
                    let parts: Vec<String> = items.into_iter().map(|k| escape_string(k)).collect();
                    format!("Set{{{}}}", parts.join(","))
                })
            }
            Value::Closure { params, body, env } => {
                let mut param_parts = Vec::new();
                for param in params {
                    param_parts.push(format!(
                        "{}:{}",
                        escape_string(&param.name),
                        self.ty_key(&param.ty)
                    ));
                }
                let body_key = format!("HirExpr({})", escape_string(&format!("{:?}", body)));
                let env_key = self.env_key(env);
                format!(
                    "Closure{{params=[{}],body={},env={}}}",
                    param_parts.join(","),
                    body_key,
                    env_key
                )
            }
            Value::AstClosure(closure) => {
                let ptr = Rc::as_ptr(closure) as usize;
                self.key_for_ptr(ptr, |ctx| {
                    let params_key = format!(
                        "AstParams({})",
                        escape_string(&format!("{:?}", closure.params))
                    );
                    let body_key =
                        format!("AstExpr({})", escape_string(&format!("{:?}", closure.body)));
                    let env_key = ctx.ast_env_key(&closure.env);
                    format!(
                        "AstClosure{{params={},body={},env={}}}",
                        params_key, body_key, env_key
                    )
                })
            }
            Value::Builtin(b) => format!("Builtin({})", escape_string(b.name)),
            Value::BuiltinFn(name, _) => format!("BuiltinFn({})", escape_string(name)),
            Value::VariantCtor { name, arity } => {
                format!("VariantCtor({},{arity})", escape_string(name))
            }
            Value::Variant(tag, payload) => {
                format!(
                    "Variant({},{})",
                    escape_string(tag),
                    self.value_key(payload)
                )
            }
            Value::Some(v) => format!("Some({})", self.value_key(v)),
            Value::None => "None".to_string(),
            Value::Ok(v) => format!("Ok({})", self.value_key(v)),
            Value::Err(v) => format!("Err({})", self.value_key(v)),
            Value::Event(_e) => "Event(..)".to_string(),
            Value::Thunk(thunk) => {
                let ptr = Rc::as_ptr(&thunk.inner) as usize;
                self.key_for_ptr(ptr, |ctx| match &*thunk.state() {
                    ThunkState::Evaluated(v) => format!("Thunk(Evaluated,{})", ctx.value_key(v)),
                    ThunkState::Evaluating => "Thunk(Evaluating)".to_string(),
                    ThunkState::AstUnevaluated { expr, env } => {
                        let expr_key =
                            format!("AstExpr({})", escape_string(&format!("{:?}", expr)));
                        let env_key = ctx.ast_env_key(env);
                        format!("Thunk(AstUnevaluated,{expr_key},{env_key})")
                    }
                    ThunkState::HirUnevaluated { expr, env } => {
                        let expr_key = format!("Expr({})", escape_string(&format!("{:?}", expr)));
                        let env_key = ctx.env_key(env);
                        format!("Thunk(HirUnevaluated,{expr_key},{env_key})")
                    }
                })
            }
        }
    }

    fn env_key(&mut self, env: &Environment) -> String {
        let ptr = env.bindings_ptr();
        self.key_for_ptr(ptr, |ctx| {
            let mut bindings = env.bindings_snapshot();
            bindings.sort_by_key(|(id, _)| id.0);
            let parts: Vec<String> = bindings
                .into_iter()
                .map(|(id, value)| format!("{}={}", id.0, ctx.value_key(&value)))
                .collect();
            let parent_key = env.parent_ref().map(|parent| ctx.env_key(parent));
            if let Some(parent) = parent_key {
                format!("Env{{{}}}|Parent({parent})", parts.join(","))
            } else {
                format!("Env{{{}}}", parts.join(","))
            }
        })
    }

    fn ast_env_key(&mut self, env: &Rc<AstEnv>) -> String {
        let ptr = Rc::as_ptr(env) as usize;
        self.key_for_ptr(ptr, |ctx| {
            let mut bindings = env.bindings_snapshot();
            bindings.sort_by(|a, b| a.0.cmp(&b.0));
            let parts: Vec<String> = bindings
                .into_iter()
                .map(|(name, value, is_public)| {
                    let vis = if is_public { "pub" } else { "priv" };
                    format!("{}:{vis}={}", escape_string(&name), ctx.value_key(&value))
                })
                .collect();
            let parent_key = env
                .parent_rc()
                .as_ref()
                .map(|parent| ctx.ast_env_key(parent));
            if let Some(parent) = parent_key {
                format!("AstEnv{{{}}}|Parent({parent})", parts.join(","))
            } else {
                format!("AstEnv{{{}}}", parts.join(","))
            }
        })
    }

    fn ty_key(&mut self, ty: &neve_hir::Ty) -> String {
        use neve_hir::TyKind;
        match &ty.kind {
            TyKind::Int => "Int".to_string(),
            TyKind::Float => "Float".to_string(),
            TyKind::Bool => "Bool".to_string(),
            TyKind::Char => "Char".to_string(),
            TyKind::String => "String".to_string(),
            TyKind::Unit => "Unit".to_string(),
            TyKind::Var(id) => format!("Var({id})"),
            TyKind::Param(id, name) => format!("Param({id},{})", escape_string(name)),
            TyKind::SelfType => "Self".to_string(),
            TyKind::SelfAssoc(name) => format!("SelfAssoc({})", escape_string(name)),
            TyKind::Named(def, args) => {
                let args_key: Vec<String> = args.iter().map(|t| self.ty_key(t)).collect();
                format!("Named({:?},[{}])", def, args_key.join(","))
            }
            TyKind::Fn(params, ret) => {
                let params_key: Vec<String> = params.iter().map(|t| self.ty_key(t)).collect();
                format!("Fn([{}],{})", params_key.join(","), self.ty_key(ret))
            }
            TyKind::Tuple(elems) => {
                let elems_key: Vec<String> = elems.iter().map(|t| self.ty_key(t)).collect();
                format!("Tuple([{}])", elems_key.join(","))
            }
            TyKind::Record(fields) => {
                let mut entries: Vec<(String, String)> = fields
                    .iter()
                    .map(|(name, ty)| (name.clone(), self.ty_key(ty)))
                    .collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                let parts: Vec<String> = entries
                    .into_iter()
                    .map(|(name, key)| format!("{}={}", escape_string(&name), key))
                    .collect();
                format!("Record{{{}}}", parts.join(","))
            }
            TyKind::DynamicRecord(fields) => {
                let mut entries: Vec<(String, String)> = fields
                    .iter()
                    .map(|(name, ty)| (name.clone(), self.ty_key(ty)))
                    .collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                let parts: Vec<String> = entries
                    .into_iter()
                    .map(|(name, key)| format!("{}={}", escape_string(&name), key))
                    .collect();
                format!("DynamicRecord{{{}}}", parts.join(","))
            }
            TyKind::SafeRecordBase(fields) => {
                let mut entries: Vec<(String, String)> = fields
                    .iter()
                    .map(|(name, ty)| (name.clone(), self.ty_key(ty)))
                    .collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                let parts: Vec<String> = entries
                    .into_iter()
                    .map(|(name, key)| format!("{}={}", escape_string(&name), key))
                    .collect();
                format!("SafeRecordBase{{{}}}", parts.join(","))
            }
            TyKind::Forall(params, ty) => {
                let params_key = params
                    .iter()
                    .map(|p| escape_string(p))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("Forall([{}],{})", params_key, self.ty_key(ty))
            }
            TyKind::Unknown => "Unknown".to_string(),
        }
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn escape_string(value: &str) -> String {
    value.chars().flat_map(|c| c.escape_default()).collect()
}

fn escape_char(value: char) -> String {
    value.escape_default().to_string()
}

fn canonical_float(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_positive() {
            "Inf".to_string()
        } else {
            "-Inf".to_string()
        };
    }
    if value == 0.0 {
        return "0.0".to_string();
    }
    value.to_string()
}

/// Generate a stable, deterministic key for any runtime value.
/// 为任意运行时值生成稳定、确定性的键。
pub fn stable_key(value: &Value) -> String {
    let mut ctx = KeyCtx::new();
    ctx.value_key(value)
}

#[cfg(test)]
mod tests {
    use super::{CommandValue, Value, stable_key};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[test]
    fn command_runtime_object_has_stable_non_record_key() {
        let command = Value::command_object("echo", vec!["hello".to_string()]);
        let command_key = stable_key(&command);

        let command_like_record = Value::Record(Rc::new(HashMap::from([
            (
                "program".to_string(),
                Value::String(Rc::new("echo".to_string())),
            ),
            (
                "args".to_string(),
                Value::List(Rc::new(vec![Value::String(Rc::new("hello".to_string()))])),
            ),
        ])));
        let record_key = stable_key(&command_like_record);

        assert_eq!(
            command_key,
            "Command{program=\"echo\",args=[\"hello\"],cwd=None,stdin=None,env={}}".to_string()
        );
        assert_ne!(command_key, record_key);
    }

    #[test]
    fn configured_command_runtime_object_tracks_effect_config_in_key() {
        let command = Value::command_object_with_options(
            "echo",
            vec!["hello".to_string()],
            Some("/tmp".to_string()),
            Some("stdin-text".to_string()),
            HashMap::from([("LANG".to_string(), "C".to_string())]),
        );

        assert_eq!(
            stable_key(&command),
            "Command{program=\"echo\",args=[\"hello\"],cwd=Some(\"/tmp\"),stdin=Some(\"stdin-text\"),env={\"LANG\"=>\"C\"}}"
        );
    }

    #[test]
    fn redirected_command_runtime_object_tracks_embedded_redirects_in_key() {
        let command = Value::command_object_with_options_and_redirects(
            "echo",
            vec!["hello".to_string()],
            None,
            None,
            HashMap::new(),
            vec![
                super::RedirectValue::stdout_path("/tmp/neve.out"),
                super::RedirectValue::stderr_path("/tmp/neve.err"),
            ],
        );

        assert_eq!(
            stable_key(&command),
            "Command{program=\"echo\",args=[\"hello\"],cwd=None,stdin=None,env={},redirects=[Redirect{stream=stdout,path=\"/tmp/neve.out\"},Redirect{stream=stderr,path=\"/tmp/neve.err\"}]}"
        );
    }

    #[test]
    fn pipeline_runtime_object_has_stable_non_list_key() {
        let pipeline = Value::pipeline_object(vec![
            CommandValue::new("printf", vec!["hello".to_string()]),
            CommandValue::new("cat", Vec::new()),
        ]);
        let pipeline_key = stable_key(&pipeline);
        let list_key = stable_key(&Value::List(Rc::new(vec![
            Value::command_object("printf", vec!["hello".to_string()]),
            Value::command_object("cat", Vec::new()),
        ])));

        assert_eq!(
            pipeline_key,
            "Pipeline{commands=[Command{program=\"printf\",args=[\"hello\"],cwd=None,stdin=None,env={}},Command{program=\"cat\",args=[],cwd=None,stdin=None,env={}}]}"
        );
        assert_ne!(pipeline_key, list_key);
    }

    #[test]
    fn redirected_pipeline_runtime_object_tracks_embedded_redirects_in_key() {
        let pipeline = Value::pipeline_object_with_redirects(
            vec![
                CommandValue::new("printf", vec!["hello".to_string()]),
                CommandValue::new("cat", Vec::new()),
            ],
            vec![
                super::RedirectValue::stdout_path("/tmp/neve.out"),
                super::RedirectValue::stderr_path("/tmp/neve.err"),
            ],
        );

        assert_eq!(
            stable_key(&pipeline),
            "Pipeline{commands=[Command{program=\"printf\",args=[\"hello\"],cwd=None,stdin=None,env={}},Command{program=\"cat\",args=[],cwd=None,stdin=None,env={}}],redirects=[Redirect{stream=stdout,path=\"/tmp/neve.out\"},Redirect{stream=stderr,path=\"/tmp/neve.err\"}]}"
        );
    }

    #[test]
    fn redirect_runtime_object_has_stable_non_path_key() {
        let redirect = Value::redirect_stdout_path_object("/tmp/neve.out");
        let redirect_key = stable_key(&redirect);
        let path_key = stable_key(&Value::Path(Rc::new(PathBuf::from("/tmp/neve.out"))));

        assert_eq!(
            redirect_key,
            "Redirect{stream=stdout,path=\"/tmp/neve.out\"}"
        );
        assert_ne!(redirect_key, path_key);
    }

    #[test]
    fn redirect_stderr_runtime_object_has_distinct_stable_key() {
        let stdout_redirect = Value::redirect_stdout_path_object("/tmp/neve.out");
        let stderr_redirect = Value::redirect_stderr_path_object("/tmp/neve.out");

        assert_eq!(
            stable_key(&stderr_redirect),
            "Redirect{stream=stderr,path=\"/tmp/neve.out\"}"
        );
        assert_ne!(stable_key(&stdout_redirect), stable_key(&stderr_redirect));
    }

    #[test]
    fn redirect_stdin_runtime_object_has_distinct_stable_key() {
        let stdout_redirect = Value::redirect_stdout_path_object("/tmp/neve.in");
        let stdin_redirect = Value::redirect_stdin_path_object("/tmp/neve.in");

        assert_eq!(
            stable_key(&stdin_redirect),
            "Redirect{stream=stdin,path=\"/tmp/neve.in\"}"
        );
        assert_ne!(stable_key(&stdout_redirect), stable_key(&stdin_redirect));
    }

    #[test]
    fn task_runtime_object_has_stable_non_command_key() {
        let task = Value::task_command_process_result_object(CommandValue::new(
            "printf",
            vec!["hello".to_string()],
        ));
        let task_key = stable_key(&task);
        let command_key = stable_key(&Value::command_object("printf", vec!["hello".to_string()]));

        assert_eq!(
            task_key,
            "Task{output=ProcessResult,command=Command{program=\"printf\",args=[\"hello\"],cwd=None,stdin=None,env={}}}"
        );
        assert_ne!(task_key, command_key);
    }

    #[test]
    fn pipeline_task_runtime_object_has_stable_non_pipeline_key() {
        let task = Value::task_pipeline_process_result_object(super::PipelineValue::new(vec![
            Rc::new(CommandValue::new("printf", vec!["hello".to_string()])),
            Rc::new(CommandValue::new("cat", Vec::new())),
        ]));
        let task_key = stable_key(&task);
        let pipeline_key = stable_key(&Value::pipeline_object(vec![
            CommandValue::new("printf", vec!["hello".to_string()]),
            CommandValue::new("cat", Vec::new()),
        ]));

        assert_eq!(
            task_key,
            "Task{output=ProcessResult,pipeline=Pipeline{commands=[Command{program=\"printf\",args=[\"hello\"],cwd=None,stdin=None,env={}},Command{program=\"cat\",args=[],cwd=None,stdin=None,env={}}]}}"
        );
        assert_ne!(task_key, pipeline_key);
    }

    #[test]
    fn process_result_runtime_object_is_not_equal_to_record() {
        let process_result = Value::process_result_object(0, true, "stdout", "stderr");
        let process_result_like_record = Value::Record(Rc::new(HashMap::from([
            ("code".to_string(), Value::Int(0.into())),
            ("success".to_string(), Value::Bool(true)),
            (
                "stdout".to_string(),
                Value::String(Rc::new("stdout".to_string())),
            ),
            (
                "stderr".to_string(),
                Value::String(Rc::new("stderr".to_string())),
            ),
        ])));

        assert_ne!(process_result, process_result_like_record);
    }

    #[test]
    fn opaque_runtime_objects_compare_by_payload() {
        let left = Value::command_object("echo", vec!["hello".to_string()]);
        let right = Value::command_object("echo", vec!["hello".to_string()]);
        let configured = Value::command_object_with_options(
            "echo",
            vec!["hello".to_string()],
            Some("/tmp".to_string()),
            None,
            HashMap::new(),
        );
        let pipeline =
            Value::pipeline_object(vec![CommandValue::new("echo", vec!["hello".to_string()])]);
        let redirect = Value::redirect_stdout_path_object("/tmp/neve.out");
        let task = Value::task_command_process_result_object(CommandValue::new(
            "echo",
            vec!["hello".to_string()],
        ));
        let pipeline_task =
            Value::task_pipeline_process_result_object(super::PipelineValue::new(vec![
                Rc::new(CommandValue::new("echo", vec!["hello".to_string()])),
                Rc::new(CommandValue::new("cat", Vec::new())),
            ]));
        let different = Value::process_result_object(1, false, "", "boom");

        assert_eq!(left, right);
        assert_ne!(left, configured);
        assert_ne!(left, pipeline);
        assert_ne!(left, redirect);
        assert_ne!(left, task);
        assert_ne!(left, pipeline_task);
        assert_ne!(left, different);
    }
}
