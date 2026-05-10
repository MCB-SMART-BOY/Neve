//! IO operations for the standard library.
//! 标准库的 IO 操作。
//!
//! These are impure operations that interact with the file system.
//! They are primarily used during package builds and configuration generation.
//! 这些是与文件系统交互的非纯操作。
//! 主要用于包构建和配置生成期间。


// === Spawn registry for non-blocking task execution ===

use std::sync::{Arc, Mutex, OnceLock};

/// Global spawn registry for non-blocking task handles.
static SPAWN_REGISTRY: OnceLock<Mutex<HashMap<i64, Arc<Mutex<SpawnState>>>>> = OnceLock::new();

fn spawn_registry() -> &'static Mutex<HashMap<i64, Arc<Mutex<SpawnState>>>> {
    SPAWN_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

static NEXT_SPAWN_ID: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// State of a spawned (background) task.
#[derive(Clone, Debug)]
pub enum SpawnState {
    Running,
    Done(Result<(i32, bool, String, String), String>),
    Cancelled,
}

use neve_eval::value::{
    BuiltinFn, CommandValue, EventKind, EventValue, PipelineValue, ProcessResultValue,
    RedirectValue, TaskTargetValue, TaskValue, Value,
};
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Maximum stdin payload size in bytes for blocking process execution (10 MB).
/// 阻塞进程执行中 stdin 负载的最大字节数。
pub(crate) const MAX_STDIN_BYTES: usize = 10 * 1024 * 1024;
/// Maximum stdout/stderr size in bytes for blocking process execution (50 MB).
/// 阻塞进程执行中 stdout/stderr 的最大字节数。
pub(crate) const MAX_OUTPUT_BYTES: usize = 50 * 1024 * 1024;

/// Script arguments set by the CLI before evaluation.
/// CLI 在求值前设置的脚本参数。
static SCRIPT_ARGS: std::sync::RwLock<Vec<String>> = std::sync::RwLock::new(Vec::new());

/// Set script arguments (called by CLI before evaluation).
/// 设置脚本参数（由 CLI 在求值前调用）。
pub fn set_script_args(args: Vec<String>) {
    if let Ok(mut guard) = SCRIPT_ARGS.write() {
        *guard = args;
    }
}

mod event;
/// Returns all IO builtins.
/// 返回所有 IO 内置函数。
mod fs;
mod process;

pub fn builtins() -> Vec<(&'static str, Value)> {
    let mut bindings = vec![
        // === Modern aliases (preferred short names) ===
        (
            "io.read",
            Value::Builtin(BuiltinFn {
                name: "io.read",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(p) => std::fs::read_to_string(p.as_path())
                        .map(|s| Value::String(Rc::new(s)))
                        .map_err(|e| format!("io.read: {e}")),
                    Value::String(s) => std::fs::read_to_string(s.as_str())
                        .map(|s| Value::String(Rc::new(s)))
                        .map_err(|e| format!("io.read: {e}")),
                    _ => Err("io.read expects a Path or String".to_string()),
                },
            }),
        ),
        (
            "io.write",
            Value::Builtin(BuiltinFn {
                name: "io.write",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Path(p), Value::String(s)) => std::fs::write(p.as_path(), s.as_bytes())
                        .map(|_| Value::Unit)
                        .map_err(|e| format!("io.write: {e}")),
                    (Value::String(p), Value::String(s)) => {
                        std::fs::write(p.as_str(), s.as_bytes())
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.write: {e}"))
                    }
                    _ => Err("io.write expects (Path|String, String)".to_string()),
                },
            }),
        ),
        (
            "io.run",
            Value::Builtin(BuiltinFn {
                name: "io.run",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Command(command) => execute_command_value(command),
                    _ => Err("io.run expects a Command".to_string()),
                },
            }),
        ),
        (
            "io.shell",
            Value::Builtin(BuiltinFn {
                name: "io.shell",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(cmd) => {
                        let output = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(cmd.as_str())
                            .output()
                            .map_err(|e| format!("io.shell: {e}"))?;
                        Ok(Value::ProcessResult(Rc::new(ProcessResultValue::new(
                            output.status.code().unwrap_or(-1),
                            output.status.success(),
                            String::from_utf8_lossy(&output.stdout).to_string(),
                            String::from_utf8_lossy(&output.stderr).to_string(),
                        ))))
                    }
                    _ => Err("io.shell expects a String".to_string()),
                },
            }),
        ),
        // Interactive / 交互输入
        (
            "io.input",
            Value::Builtin(BuiltinFn {
                name: "io.input",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(prompt) => read_input(prompt, false),
                    _ => Err("io.input expects a String prompt".to_string()),
                },
            }),
        ),
        (
            "io.readPassword",
            Value::Builtin(BuiltinFn {
                name: "io.readPassword",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(prompt) => read_input(prompt, true),
                    _ => Err("io.readPassword expects a String prompt".to_string()),
                },
            }),
        ),
        // Glob / 文件匹配
        (
            "io.glob",
            Value::Builtin(BuiltinFn {
                name: "io.glob",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(pattern) => {
                        let paths: Result<Vec<_>, _> = glob::glob(pattern.as_str())
                            .map_err(|e| format!("io.glob: invalid pattern: {e}"))?
                            .map(|entry| {
                                entry
                                    .map(|p| Value::Path(Rc::new(p)))
                                    .map_err(|e| format!("io.glob: {e}"))
                            })
                            .collect();
                        paths.map(|v| Value::List(Rc::new(v)))
                    }
                    _ => Err("io.glob expects a String pattern".to_string()),
                },
            }),
        ),
        // Signals / 信号处理
        (
            "io.onSignal",
            Value::Builtin(BuiltinFn {
                name: "io.onSignal",
                arity: 2,
                func: |_args| Err("io.onSignal is evaluator-owned".to_string()),
            }),
        ),
        // Defer / 延迟执行
        (
            "io.defer",
            Value::Builtin(BuiltinFn {
                name: "io.defer",
                arity: 1,
                func: |_args| Err("io.defer is evaluator-owned".to_string()),
            }),
        ),
        // Temporal / 时序约束 (evaluator-owned)
        (
            "io.retry",
            Value::Builtin(BuiltinFn {
                name: "io.retry",
                arity: 3,
                func: |_args| Err("io.retry is evaluator-owned".to_string()),
            }),
        ),
        (
            "io.ensure",
            Value::Builtin(BuiltinFn {
                name: "io.ensure",
                arity: 3,
                func: |_args| Err("io.ensure is evaluator-owned".to_string()),
            }),
        ),
        // Output / 输出 (also available as global print/println)
        (
            "print",
            Value::Builtin(BuiltinFn {
                name: "print",
                arity: 1,
                func: |args| {
                    use std::io::Write;
                    let stdout = std::io::stdout();
                    let mut handle = stdout.lock();
                    let output = format_value_for_output(&args[0]);
                    handle
                        .write_all(output.as_bytes())
                        .map(|_| Value::Unit)
                        .map_err(|e| format!("print: {e}"))
                },
            }),
        ),
        (
            "println",
            Value::Builtin(BuiltinFn {
                name: "println",
                arity: 1,
                func: |args| {
                    use std::io::Write;
                    let stdout = std::io::stdout();
                    let mut handle = stdout.lock();
                    let output = format_value_for_output(&args[0]);
                    handle
                        .write_all(output.as_bytes())
                        .and_then(|_| handle.write_all(b"\n"))
                        .map(|_| Value::Unit)
                        .map_err(|e| format!("println: {e}"))
                },
            }),
        ),
        (
            "io.print",
            Value::Builtin(BuiltinFn {
                name: "io.print",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => {
                        use std::io::Write;
                        let stdout = std::io::stdout();
                        let mut handle = stdout.lock();
                        handle
                            .write_all(s.as_bytes())
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.print: {e}"))
                    }
                    _ => Err("io.print expects a String".to_string()),
                },
            }),
        ),
        (
            "io.println",
            Value::Builtin(BuiltinFn {
                name: "io.println",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => {
                        use std::io::Write;
                        let stdout = std::io::stdout();
                        let mut handle = stdout.lock();
                        handle
                            .write_all(s.as_bytes())
                            .and_then(|_| handle.write_all(b"\n"))
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.println: {e}"))
                    }
                    _ => Err("io.println expects a String".to_string()),
                },
            }),
        ),
        // === TTY / terminal ===
        (
            "io.isTTY",
            Value::Builtin(BuiltinFn {
                name: "io.isTTY", arity: 1,
                func: |args| {
                    let fd: i32 = match &args[0] {
                        Value::Int(n) => n.clone().try_into().map_err(|_| "io.isTTY: fd must be a valid integer".to_string())?,
                        _ => return Err("io.isTTY expects an Int (file descriptor)".to_string()),
                    };
                    #[cfg(unix)]
                    unsafe {
                        Ok(Value::Bool(libc::isatty(fd) != 0))
                    }
                    #[cfg(not(unix))]
                    {
                        let _ = fd;
                        Ok(Value::Bool(false))
                    }
                },
            }),
        ),
        (
            "io.terminalSize",
            Value::Builtin(BuiltinFn {
                name: "io.terminalSize", arity: 0,
                func: |_args| {
                    #[cfg(unix)]
                    unsafe {
                        let mut winsize: libc::winsize = std::mem::zeroed();
                        if libc::ioctl(1, libc::TIOCGWINSZ, &mut winsize) == 0 {
                            let mut fields = HashMap::new();
                            fields.insert("rows".to_string(), Value::Int((winsize.ws_row as i64).into()));
                            fields.insert("cols".to_string(), Value::Int((winsize.ws_col as i64).into()));
                            Ok(Value::Some(Box::new(Value::Record(Rc::new(fields)))))
                        } else {
                            Ok(Value::None)
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        Ok(Value::None)
                    }
                },
            }),
        ),
        
        // === Non-blocking task spawn/poll/cancel ===
        (
            "io.spawnWithTimeout",
            Value::Builtin(BuiltinFn {
                name: "io.spawnWithTimeout", arity: 2,
                func: |args| {
                    let task = match &args[0] {
                        Value::Task(t) => t.clone(),
                        _ => return Err("io.spawnWithTimeout expects a Task".to_string()),
                    };
                    let timeout_ms: u64 = match &args[1] {
                        Value::Int(n) => n.clone().try_into().map_err(|_| "io.spawnWithTimeout: timeout must be non-negative".to_string())?,
                        _ => return Err("io.spawnWithTimeout expects an Int (timeout in ms)".to_string()),
                    };
                    let state = Arc::new(Mutex::new(SpawnState::Running));
                    let state_clone = state.clone();
                    match task.target() {
                        neve_eval::value::TaskTargetValue::Command(cmd) => {
                            let program = cmd.program().to_string();
                            let args_list = cmd.args().to_vec();
                            let cwd = cmd.cwd().map(|s| s.to_string());
                            let env = cmd.env().clone();
                            let stdin_data = cmd.stdin().map(|s| s.to_string());
                            std::thread::spawn(move || {
                                let mut c = std::process::Command::new(&program);
                                c.args(&args_list);
                                if let Some(ref wd) = cwd { c.current_dir(wd); }
                                for (k, v) in &env { c.env(k, v); }
                                if stdin_data.is_some() { c.stdin(std::process::Stdio::piped()); }
                                c.stdout(std::process::Stdio::piped());
                                c.stderr(std::process::Stdio::piped());
                                let result = (|| {
                                    let mut child = c.spawn().map_err(|e| format!("spawn: {e}"))?;
                                    if let Some(ref data) = stdin_data {
                                        use std::io::Write;
                                        if let Some(mut pipe) = child.stdin.take() {
                                            pipe.write_all(data.as_bytes()).map_err(|e| format!("stdin: {e}"))?;
                                        }
                                    }
                                    let output = child.wait_with_output().map_err(|e| format!("wait: {e}"))?;
                                    Ok((output.status.code().unwrap_or(-1), output.status.success(), String::from_utf8_lossy(&output.stdout).to_string(), String::from_utf8_lossy(&output.stderr).to_string()))
                                })();
                                *state_clone.lock().unwrap() = SpawnState::Done(result);
                            });
                        }
                        neve_eval::value::TaskTargetValue::Pipeline(pipeline) => {
                            let stages: Vec<StageData> = pipeline.commands().iter().map(|cmd| StageData {
                                program: cmd.program().to_string(),
                                args: cmd.args().to_vec(),
                                cwd: cmd.cwd().map(|s| s.to_string()),
                                env: cmd.env().clone(),
                                stdin: cmd.stdin().map(|s| s.to_string()),
                            }).collect();
                            std::thread::spawn(move || {
                                let result = run_pipeline_stages(&stages);
                                *state_clone.lock().unwrap() = SpawnState::Done(result);
                            });
                        }
                    }
                    let id = NEXT_SPAWN_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    spawn_registry().lock().unwrap().insert(id, state.clone());
                    // Timeout watcher
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
                        let mut s = state.lock().unwrap();
                        if matches!(&*s, SpawnState::Running) {
                            *s = SpawnState::Cancelled;
                        }
                    });
                    Ok(Value::Int(id.into()))
                },
            }),
        ),
        (
            "io.spawn",
            Value::Builtin(BuiltinFn {
                name: "io.spawn", arity: 1,
                func: |args| {
                    let task = match &args[0] {
                        Value::Task(t) => t.clone(),
                        _ => return Err("io.spawn expects a Task".to_string()),
                    };
                    let state = Arc::new(Mutex::new(SpawnState::Running));
                    let state_clone = state.clone();
                    match task.target() {
                        neve_eval::value::TaskTargetValue::Command(cmd) => {
                            let program = cmd.program().to_string();
                            let args_list = cmd.args().to_vec();
                            let cwd = cmd.cwd().map(|s| s.to_string());
                            let env = cmd.env().clone();
                            let stdin_data = cmd.stdin().map(|s| s.to_string());
                            std::thread::spawn(move || {
                                let mut c = std::process::Command::new(&program);
                                c.args(&args_list);
                                if let Some(ref wd) = cwd { c.current_dir(wd); }
                                for (k, v) in &env { c.env(k, v); }
                                if stdin_data.is_some() { c.stdin(std::process::Stdio::piped()); }
                                c.stdout(std::process::Stdio::piped());
                                c.stderr(std::process::Stdio::piped());
                                let result = (|| {
                                    let mut child = c.spawn().map_err(|e| format!("spawn: {e}"))?;
                                    if let Some(ref data) = stdin_data {
                                        use std::io::Write;
                                        if let Some(mut pipe) = child.stdin.take() {
                                            pipe.write_all(data.as_bytes()).map_err(|e| format!("stdin: {e}"))?;
                                        }
                                    }
                                    let output = child.wait_with_output().map_err(|e| format!("wait: {e}"))?;
                                    Ok((output.status.code().unwrap_or(-1), output.status.success(), String::from_utf8_lossy(&output.stdout).to_string(), String::from_utf8_lossy(&output.stderr).to_string()))
                                })();
                                *state_clone.lock().unwrap() = SpawnState::Done(result);
                            });
                        }
                        neve_eval::value::TaskTargetValue::Pipeline(pipeline) => {
                            // Extract all stage data before moving to thread
                            let stages: Vec<StageData> = pipeline.commands().iter().map(|cmd| {
                                StageData {
                                    program: cmd.program().to_string(),
                                    args: cmd.args().to_vec(),
                                    cwd: cmd.cwd().map(|s| s.to_string()),
                                    env: cmd.env().clone(),
                                    stdin: cmd.stdin().map(|s| s.to_string()),
                                }
                            }).collect();
                            std::thread::spawn(move || {
                                let result = run_pipeline_stages(&stages);
                                *state_clone.lock().unwrap() = SpawnState::Done(result);
                            });
                        }
                    }
                    let id = NEXT_SPAWN_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    spawn_registry().lock().unwrap().insert(id, state);
                    Ok(Value::Int(id.into()))
                },
            }),
        ),
        (
            "io.poll",
            Value::Builtin(BuiltinFn {
                name: "io.poll", arity: 1,
                func: |args| {
                    let id: i64 = match &args[0] {
                        Value::Int(n) => n.clone().try_into().map_err(|_| "io.poll: invalid ID".to_string())?,
                        _ => return Err("io.poll expects an Int (spawn ID)".to_string()),
                    };
                    let state = {
                        let registry = spawn_registry().lock().unwrap();
                        registry.get(&id).ok_or(format!("io.poll: no task with ID {id}"))?.clone()
                    };
                    let mut s = state.lock().unwrap();
                    match &*s {
                        SpawnState::Running => Ok(Value::None),
                        SpawnState::Done(Ok((code, success, stdout, stderr))) => {
                            let result = Value::Some(Box::new(Value::ProcessResult(Rc::new(
                                ProcessResultValue::new(*code, *success, stdout.clone(), stderr.clone())
                            ))));
                            *s = SpawnState::Cancelled;
                            spawn_registry().lock().unwrap().remove(&id);
                            Ok(result)
                        }
                        SpawnState::Done(Err(e)) => {
                            let err = e.clone();
                            spawn_registry().lock().unwrap().remove(&id);
                            Err(err)
                        }
                        SpawnState::Cancelled => Err(format!("io.poll: task {id} already consumed")),
                    }
                },
            }),
        ),
        (
            "io.cancel",
            Value::Builtin(BuiltinFn {
                name: "io.cancel", arity: 1,
                func: |args| {
                    let id: i64 = match &args[0] {
                        Value::Int(n) => n.clone().try_into().map_err(|_| "io.cancel: invalid ID".to_string())?,
                        _ => return Err("io.cancel expects an Int (spawn ID)".to_string()),
                    };
                    if let Some(state) = spawn_registry().lock().unwrap().remove(&id) {
                        *state.lock().unwrap() = SpawnState::Cancelled;
                    }
                    Ok(Value::Unit)
                },
            }),
        ),
    ];
    bindings.extend(fs::builtins());
    bindings.extend(process::builtins());
    bindings.extend(event::builtins());
    bindings
}

/// Format any Value for human-readable output (used by print/println).
pub(crate) fn format_value_for_output(value: &Value) -> String {
    match value {
        Value::Unit => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.to_string(),
        Value::Char(c) => c.to_string(),
        Value::None => "None".to_string(),
        Value::Some(v) => format!("Some({})", format_value_for_output(v)),
        Value::Ok(v) => format!("Ok({})", format_value_for_output(v)),
        Value::Err(e) => format!("Err({})", format_value_for_output(e)),
        Value::List(items) => {
            let items: Vec<String> = items.iter().map(format_value_for_output).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Tuple(elems) => {
            let items: Vec<String> = elems.iter().map(format_value_for_output).collect();
            format!("({})", items.join(", "))
        }
        Value::Record(fields) => {
            let items: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{} = {}", k, format_value_for_output(v)))
                .collect();
            format!("#{{ {} }}", items.join(", "))
        }
        Value::Path(p) => p.to_string_lossy().to_string(),
        Value::ProcessResult(r) => {
            if r.is_success() {
                r.stdout().trim_end().to_string()
            } else {
                r.stderr().trim_end().to_string()
            }
        }
        _ => format!("{:?}", value),
    }
}

/// Poll an event source for its next value.
pub(crate) fn poll_event(event: &EventValue) -> Result<Value, String> {
    match &event.kind {
        EventKind::Timer { interval_ms } => {
            std::thread::sleep(std::time::Duration::from_millis(*interval_ms));
            Ok(Value::Int((*interval_ms as i64).into()))
        }
        EventKind::FileWatch { path } => {
            use std::io::Read;
            let mut content = String::new();
            std::fs::File::open(path)
                .and_then(|mut f| f.read_to_string(&mut content))
                .map_err(|e| format!("poll: {e}"))?;
            Ok(Value::String(Rc::new(content)))
        }
        EventKind::Mapped { source, func } => {
            // Poll the source event, then apply the transformation function
            let source_val = poll_event(source)?;
            apply_value_function(func, &[source_val])
        }
        EventKind::Filtered { source, predicate } => {
            // Poll the source event repeatedly until predicate returns true
            // Limit to 1000 attempts to prevent infinite loops
            for _ in 0..1000 {
                let source_val = poll_event(source)?;
                match apply_value_function(predicate, &[source_val.clone()]) {
                    Ok(Value::Bool(true)) => return Ok(source_val),
                    Ok(_) => continue, // Predicate returned non-true, try again
                    Err(e) => return Err(e),
                }
            }
            Err("poll: filtered event exceeded max attempts (1000)".to_string())
        }
    }
}

/// Apply a Value function to arguments. Handles Builtin and BuiltinFn.
/// Closures are not supported in this context (requires evaluator).
pub(crate) fn apply_value_function(func: &Value, args: &[Value]) -> Result<Value, String> {
    match func {
        Value::Builtin(b) => (b.func)(args),
        Value::BuiltinFn(_, f) => f(args.to_vec()),
        _ => Err(format!(
            "poll: cannot apply {:?} as a function (only builtins supported in event chains)",
            func
        )),
    }
}

/// Call a builtin/closure function value with arguments.

/// Read user input, optionally without echo.
fn read_input(prompt: &str, no_echo: bool) -> Result<Value, String> {
    use std::io::{BufRead, Write};
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(prompt.as_bytes())
        .map_err(|e| format!("input: {e}"))?;
    handle.flush().map_err(|e| format!("input: {e}"))?;

    let line = if no_echo {
        rpassword::read_password().map_err(|e| format!("input: {e}"))?
    } else {
        let stdin = std::io::stdin();
        let mut line = String::new();
        stdin
            .lock()
            .read_line(&mut line)
            .map_err(|e| format!("input: {e}"))?;
        line.trim_end().to_string()
    };
    Ok(Value::String(Rc::new(line)))
}

/// Compute SHA-256 hash and return as hex string.
/// 计算 SHA-256 哈希并返回十六进制字符串。
pub(crate) fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(data);
    format!("{:x}", digest)
}

/// Atomic write: write content to a temporary file then rename to target path.
/// This avoids partial writes and leaves no temp residue on success.
/// 原子写入：先写入临时文件，然后重命名为目标路径。
/// 避免部分写入，成功时不留下临时文件残留。
pub(crate) fn atomic_write(path: &str, content: &[u8]) -> Result<(), String> {
    let target = std::path::Path::new(path);
    atomic_write_path(target, content)
}

pub(crate) fn atomic_write_path(target: &std::path::Path, content: &[u8]) -> Result<(), String> {
    // Generate unique temp path in same directory to ensure rename is on same filesystem
    let pid = std::process::id();
    let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));
    let stem = target
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("atomic");
    let ext = target
        .extension()
        .and_then(|s| s.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    let tmp_name = format!("{stem}.tmp.{pid}{ext}");
    let tmp_path = parent.join(&tmp_name);

    // Write to temp file
    std::fs::write(&tmp_path, content)
        .map_err(|e| format!("atomic_write: failed to write temp file: {e}"))?;

    // Atomic rename
    std::fs::rename(&tmp_path, target).map_err(|e| {
        // Clean up temp file on failure
        let _ = std::fs::remove_file(&tmp_path);
        format!("atomic_write: rename failed: {e}")
    })
}

/// Batch atomic write using two-phase commit.
/// All entries are written to temp files first (phase 1),
/// then all are renamed (phase 2). If any rename fails, all renames are rolled back.
/// 批量原子写入，使用两阶段提交。
/// 第一阶段所有条目写入临时文件，第二阶段全部重命名。
/// 如果任何重命名失败，所有重命名都会被回滚。
pub(crate) fn atomic_write_all(entries: &[Value]) -> Result<(), String> {
    if entries.is_empty() {
        return Ok(());
    }

    struct PendingEntry {
        target: std::path::PathBuf,
        tmp_path: std::path::PathBuf,
    }

    let pid = std::process::id();
    let mut pending: Vec<PendingEntry> = Vec::with_capacity(entries.len());

    // Phase 1: Write all entries to temp files
    for (i, entry) in entries.iter().enumerate() {
        let fields = match entry {
            Value::Record(fields) => fields,
            _ => {
                return Err(format!(
                    "atomicWriteAll: entries[{i}] must be a Record with 'path' and 'content' fields"
                ));
            }
        };

        let path = fields
            .get("path")
            .and_then(|v| match v {
                Value::String(s) => Some(s.as_str().to_string()),
                _ => None,
            })
            .ok_or_else(|| format!("atomicWriteAll: entries[{i}] missing 'path' string field"))?;

        let content = fields
            .get("content")
            .and_then(|v| match v {
                Value::String(s) => Some(s.as_bytes().to_vec()),
                _ => None,
            })
            .ok_or_else(|| {
                format!("atomicWriteAll: entries[{i}] missing 'content' string field")
            })?;

        let target = std::path::PathBuf::from(&path);
        let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));
        let stem = target
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("atomic");
        let ext = target
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();
        let tmp_name = format!("{stem}.tmp.{pid}.{i}{ext}");
        let tmp_path = parent.join(&tmp_name);

        std::fs::write(&tmp_path, &content).map_err(|e| {
            // Clean up all temp files written so far
            for p in &pending {
                let _ = std::fs::remove_file(&p.tmp_path);
            }
            let _ = std::fs::remove_file(&tmp_path);
            format!("atomicWriteAll: write temp file failed for entries[{i}]: {e}")
        })?;

        pending.push(PendingEntry { target, tmp_path });
    }

    // Phase 2: Rename all temp files to targets
    // Track which ones have been renamed so we can roll back
    let mut renamed: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();
    for entry in &pending {
        match std::fs::rename(&entry.tmp_path, &entry.target) {
            Ok(()) => {
                renamed.push((entry.target.clone(), entry.tmp_path.clone()));
            }
            Err(e) => {
                // Rollback: rename already-committed files back to temp
                for (target, tmp_path) in &renamed {
                    let _ = std::fs::rename(target, tmp_path);
                }
                // Clean up remaining temp files
                for p in &pending {
                    let _ = std::fs::remove_file(&p.tmp_path);
                }
                return Err(format!("atomicWriteAll: rename failed: {e}"));
            }
        }
    }

    Ok(())
}

pub(crate) fn list_to_string_vec(items: &[Value], arg_name: &str) -> Result<Vec<String>, String> {
    items
        .iter()
        .enumerate()
        .map(|(idx, v)| match v {
            Value::String(s) => Ok(s.to_string()),
            _ => Err(format!("{arg_name}[{idx}] must be String")),
        })
        .collect()
}

pub(crate) fn list_to_command_vec(
    items: &[Value],
    arg_name: &str,
) -> Result<Vec<Rc<CommandValue>>, String> {
    items
        .iter()
        .enumerate()
        .map(|(idx, v)| match v {
            Value::Command(command) => Ok(Rc::clone(command)),
            _ => Err(format!("{arg_name}[{idx}] must be Command")),
        })
        .collect()
}

pub(crate) fn list_to_redirect_vec(
    items: &[Value],
    arg_name: &str,
) -> Result<Vec<RedirectValue>, String> {
    items
        .iter()
        .enumerate()
        .map(|(idx, v)| match v {
            Value::Redirect(redirect) => Ok((**redirect).clone()),
            _ => Err(format!("{arg_name}[{idx}] must be Redirect")),
        })
        .collect()
}

pub(crate) fn list_to_task_vec(
    items: &[Value],
    arg_name: &str,
) -> Result<Vec<Rc<TaskValue>>, String> {
    items
        .iter()
        .enumerate()
        .map(|(idx, v)| match v {
            Value::Task(task) => Ok(Rc::clone(task)),
            _ => Err(format!("{arg_name}[{idx}] must be Task[ProcessResult]")),
        })
        .collect()
}

pub(crate) fn output_to_process_result_value(output: std::process::Output) -> ProcessResultValue {
    let code = output.status.code().unwrap_or(-1);
    ProcessResultValue::new(
        code,
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

pub(crate) fn execute_command_lines(command: &CommandValue) -> Result<Value, String> {
    let mut cmd = configured_process_command(command);
    let output = cmd
        .output()
        .map_err(|e| format!("io.execCommandLines: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<Value> = stdout
        .lines()
        .map(|line| Value::String(Rc::new(line.to_string())))
        .collect();
    Ok(Value::List(Rc::new(lines)))
}

pub(crate) fn execute_command_value(command: &CommandValue) -> Result<Value, String> {
    let result = execute_command_value_to_process_result(command, "io.execCommand")?;
    Ok(Value::ProcessResult(Rc::new(result)))
}

pub(crate) fn execute_pipeline_value(pipeline: &PipelineValue) -> Result<Value, String> {
    let result = execute_pipeline_value_to_process_result(pipeline, "io.execPipeline")?;
    Ok(Value::ProcessResult(Rc::new(result)))
}

struct RawCommandTarget {
    program: String,
    args: Vec<String>,
    cwd: Option<String>,
    stdin: Option<String>,
    env: HashMap<String, String>,
}

struct RawProcessResult {
    code: i32,
    success: bool,
    stdout: String,
    stderr: String,
}

fn await_pipeline_with_timeout(pipeline: &PipelineValue, timeout_ms: u64) -> Result<Value, String> {
    let commands: Vec<RawCommandTarget> = pipeline
        .commands()
        .iter()
        .map(|cmd| RawCommandTarget {
            program: cmd.program().to_string(),
            args: cmd.args().to_vec(),
            cwd: cmd.cwd().map(|s| s.to_string()),
            stdin: cmd.stdin().map(|s| s.to_string()),
            env: cmd.env().clone(),
        })
        .collect();

    if commands.is_empty() {
        return Err("io.awaitTaskWithTimeout: pipeline requires at least one command".to_string());
    }

    let current_pid = std::sync::Arc::new(std::sync::Mutex::new(None::<u32>));
    let pid_for_kill = current_pid.clone();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut previous_stdout: Option<String> = None;
        let mut combined_stderr = String::new();
        let mut last_code = 0;
        let mut last_success = false;

        for (idx, raw) in commands.iter().enumerate() {
            let mut cmd = std::process::Command::new(&raw.program);
            cmd.args(&raw.args);
            if let Some(cwd) = &raw.cwd {
                cmd.current_dir(cwd);
            }
            for (key, value) in &raw.env {
                cmd.env(key, value);
            }
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Err(format!("io.awaitTaskWithTimeout: {e}")));
                    return;
                }
            };

            // Track current pid for kill-on-timeout
            *pid_for_kill.lock().unwrap() = Some(child.id());

            // Feed stdin from previous stage or initial input
            let stage_stdin = if idx == 0 {
                raw.stdin.as_deref().or(previous_stdout.as_deref())
            } else {
                previous_stdout.as_deref()
            };

            if let Some(stdin_text) = stage_stdin
                && let Some(mut pipe) = child.stdin.take()
            {
                if idx == 0 && stdin_text.len() > MAX_STDIN_BYTES {
                    let _ = tx.send(Err(format!("io.awaitTaskWithTimeout: stdin exceeds maximum size of {MAX_STDIN_BYTES} bytes")));
                    return;
                }
                use std::io::Write;
                if let Err(e) = pipe.write_all(stdin_text.as_bytes()) {
                    let _ = tx.send(Err(format!("io.awaitTaskWithTimeout: {e}")));
                    return;
                }
            }

            match child.wait_with_output() {
                Ok(output) => {
                    if output.stdout.len() > MAX_OUTPUT_BYTES {
                        let _ = tx.send(Err(format!("io.awaitTaskWithTimeout: stdout exceeds maximum size of {MAX_OUTPUT_BYTES} bytes")));
                        return;
                    }
                    if output.stderr.len() > MAX_OUTPUT_BYTES {
                        let _ = tx.send(Err(format!("io.awaitTaskWithTimeout: stderr exceeds maximum size of {MAX_OUTPUT_BYTES} bytes")));
                        return;
                    }
                    previous_stdout = Some(String::from_utf8_lossy(&output.stdout).to_string());
                    combined_stderr.push_str(&String::from_utf8_lossy(&output.stderr));
                    last_code = output.status.code().unwrap_or(-1);
                    last_success = output.status.success();
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("io.awaitTaskWithTimeout: {e}")));
                    return;
                }
            }
        }

        *pid_for_kill.lock().unwrap() = None;
        let _ = tx.send(Ok(RawProcessResult {
            code: last_code,
            success: last_success,
            stdout: previous_stdout.unwrap_or_default(),
            stderr: combined_stderr,
        }));
    });

    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(result) => match result {
            Ok(raw_result) => Ok(Value::Some(Box::new(Value::ProcessResult(Rc::new(
                ProcessResultValue::new(
                    raw_result.code,
                    raw_result.success,
                    raw_result.stdout,
                    raw_result.stderr,
                ),
            ))))),
            Err(e) => Err(e),
        },
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // Kill the currently running process
            if let Some(pid) = *current_pid.lock().unwrap() {
                neve_common::kill_process(pid);
            }
            Ok(Value::None)
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("io.awaitTaskWithTimeout: internal error".to_string())
        }
    }
}

pub(crate) fn await_task_with_timeout(task: &TaskValue, timeout_ms: u64) -> Result<Value, String> {
    let raw_target = match task.target() {
        TaskTargetValue::Command(command) => RawCommandTarget {
            program: command.program().to_string(),
            args: command.args().to_vec(),
            cwd: command.cwd().map(|s| s.to_string()),
            stdin: command.stdin().map(|s| s.to_string()),
            env: command.env().clone(),
        },
        TaskTargetValue::Pipeline(pipeline) => {
            return await_pipeline_with_timeout(pipeline, timeout_ms);
        }
    };

    // Spawn the process first to get the pid for kill-on-timeout.
    let mut cmd = std::process::Command::new(&raw_target.program);
    cmd.args(&raw_target.args);
    if let Some(cwd) = &raw_target.cwd {
        cmd.current_dir(cwd);
    }
    for (key, value) in &raw_target.env {
        cmd.env(key, value);
    }
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("io.awaitTaskWithTimeout: {e}"))?;
    let pid = child.id();

    // Write stdin if provided
    if let Some(stdin_text) = &raw_target.stdin
        && let Some(mut pipe) = child.stdin.take()
    {
        if stdin_text.len() > MAX_STDIN_BYTES {
            return Err(format!("io.awaitTaskWithTimeout: stdin exceeds maximum size of {MAX_STDIN_BYTES} bytes"));
        }
        use std::io::Write;
        pipe.write_all(stdin_text.as_bytes())
            .map_err(|e| format!("io.awaitTaskWithTimeout: failed writing stdin: {e}"))?;
    }

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let output = child.wait_with_output();
        let result = match output {
            Ok(out) => {
                if out.stdout.len() > MAX_OUTPUT_BYTES {
                    let _ = tx.send(Err(format!("io.awaitTaskWithTimeout: stdout exceeds maximum size of {MAX_OUTPUT_BYTES} bytes")));
                    return;
                }
                if out.stderr.len() > MAX_OUTPUT_BYTES {
                    let _ = tx.send(Err(format!("io.awaitTaskWithTimeout: stderr exceeds maximum size of {MAX_OUTPUT_BYTES} bytes")));
                    return;
                }
                Ok(RawProcessResult {
                    code: out.status.code().unwrap_or(-1),
                    success: out.status.success(),
                    stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                })
            }
            Err(e) => Err(format!("io.awaitTaskWithTimeout: {e}")),
        };
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(result) => match result {
            Ok(raw_result) => Ok(Value::Some(Box::new(Value::ProcessResult(Rc::new(
                ProcessResultValue::new(
                    raw_result.code,
                    raw_result.success,
                    raw_result.stdout,
                    raw_result.stderr,
                ),
            ))))),
            Err(e) => Err(e),
        },
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // Kill the process on timeout via platform kill command
            neve_common::kill_process(pid);
            Ok(Value::None)
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("io.awaitTaskWithTimeout: internal error".to_string())
        }
    }
}

pub(crate) fn await_task(task: &TaskValue) -> Result<Value, String> {
    let result = await_task_to_process_result(task, "io.awaitTask")?;
    Ok(Value::ProcessResult(Rc::new(result)))
}

pub(crate) fn await_tasks(tasks: &[Rc<TaskValue>]) -> Result<Value, String> {
    let mut results = Vec::with_capacity(tasks.len());
    for (idx, task) in tasks.iter().enumerate() {
        let result = await_task_to_process_result(task, &format!("io.awaitTasks[{idx}]"))?;
        results.push(Value::ProcessResult(Rc::new(result)));
    }
    Ok(Value::List(Rc::new(results)))
}

pub(crate) fn await_task_to_process_result(
    task: &TaskValue,
    fn_name: &str,
) -> Result<ProcessResultValue, String> {
    match task.output() {
        neve_eval::value::TaskOutputKind::ProcessResult => match task.target() {
            TaskTargetValue::Command(command) => {
                execute_command_value_to_process_result(command, fn_name)
            }
            TaskTargetValue::Pipeline(pipeline) => {
                execute_pipeline_value_to_process_result(pipeline, fn_name)
            }
        },
    }
}

pub(crate) fn execute_pipeline_value_to_process_result(
    pipeline: &PipelineValue,
    fn_name: &str,
) -> Result<ProcessResultValue, String> {
    if pipeline.has_embedded_redirects() {
        return execute_pipeline_value_with_redirects_to_process_result(
            pipeline,
            pipeline.redirects(),
            fn_name,
        );
    }
    execute_pipeline_value_to_process_result_with_input(pipeline, None, fn_name)
}

pub(crate) fn execute_pipeline_value_to_process_result_with_input(
    pipeline: &PipelineValue,
    initial_stdin: Option<&str>,
    fn_name: &str,
) -> Result<ProcessResultValue, String> {
    if pipeline.commands().is_empty() {
        return Err(format!("{fn_name}: requires a non-empty Pipeline"));
    }

    let mut previous_stdout = initial_stdin.map(str::to_owned);
    let mut combined_stderr = String::new();
    let mut last_result: Option<ProcessResultValue> = None;
    let last_stage_index = pipeline.commands().len() - 1;

    for (idx, command) in pipeline.commands().iter().enumerate() {
        if idx > 0 && command.stdin().is_some() {
            return Err(format!(
                "{fn_name}: pipeline stage {} cannot specify stdin",
                idx + 1
            ));
        }
        if idx > 0
            && command.redirects().iter().any(|redirect| {
                matches!(redirect.stream(), neve_eval::value::RedirectStream::Stdin)
            })
        {
            return Err(format!(
                "{fn_name}: pipeline stage {} cannot carry stdin redirect",
                idx + 1
            ));
        }

        if idx == 0
            && previous_stdout.is_some()
            && (command.stdin().is_some()
                || command.redirects().iter().any(|redirect| {
                    matches!(redirect.stream(), neve_eval::value::RedirectStream::Stdin)
                }))
        {
            return Err(format!(
                "{fn_name}: pipeline stage 1 cannot combine boundary stdin with stage-local stdin"
            ));
        }
        if idx < last_stage_index
            && command.redirects().iter().any(|redirect| {
                matches!(redirect.stream(), neve_eval::value::RedirectStream::Stdout)
            })
        {
            return Err(format!(
                "{fn_name}: pipeline stage {} cannot carry stdout redirect before final stage",
                idx + 1
            ));
        }

        let stage_stdin = if idx == 0 {
            previous_stdout.as_deref().or(command.stdin())
        } else {
            previous_stdout.as_deref()
        };
        let result =
            execute_command_value_to_process_result_with_input(command, stage_stdin, fn_name)?;
        previous_stdout = Some(result.stdout().to_string());
        combined_stderr.push_str(result.stderr());
        last_result = Some(result);
    }

    let last_result = last_result.expect("non-empty pipeline should produce a result");
    Ok(ProcessResultValue::new(
        last_result.code(),
        last_result.is_success(),
        last_result.stdout(),
        combined_stderr,
    ))
}

pub(crate) fn execute_pipeline_value_with_redirects_to_process_result(
    pipeline: &PipelineValue,
    redirects: &[RedirectValue],
    fn_name: &str,
) -> Result<ProcessResultValue, String> {
    if pipeline.commands().is_empty() {
        return Err(format!("{fn_name}: requires a non-empty Pipeline"));
    }
    if redirects.is_empty() {
        return Err(format!("{fn_name}: requires a non-empty List<Redirect>"));
    }

    let mut stdout_path = None;
    let mut stderr_path = None;
    let mut stdin_path = None;

    for redirect in redirects {
        let resolved = resolve_pipeline_redirect_path(pipeline, redirect);
        match redirect.stream() {
            neve_eval::value::RedirectStream::Stdout => {
                if stdout_path.replace(resolved).is_some() {
                    return Err(format!("{fn_name}: duplicate stdout redirect"));
                }
            }
            neve_eval::value::RedirectStream::Stderr => {
                if stderr_path.replace(resolved).is_some() {
                    return Err(format!("{fn_name}: duplicate stderr redirect"));
                }
            }
            neve_eval::value::RedirectStream::Stdin => {
                if stdin_path.replace(resolved).is_some() {
                    return Err(format!("{fn_name}: duplicate stdin redirect"));
                }
            }
        }
    }

    let final_stage = pipeline
        .commands()
        .last()
        .expect("non-empty pipeline should have a final stage");
    if stdout_path.is_some()
        && final_stage
            .redirects()
            .iter()
            .any(|redirect| matches!(redirect.stream(), neve_eval::value::RedirectStream::Stdout))
    {
        return Err(format!(
            "{fn_name}: final pipeline stage cannot combine boundary stdout with stage-local stdout redirect"
        ));
    }
    if stderr_path.is_some()
        && final_stage
            .redirects()
            .iter()
            .any(|redirect| matches!(redirect.stream(), neve_eval::value::RedirectStream::Stderr))
    {
        return Err(format!(
            "{fn_name}: final pipeline stage cannot combine boundary stderr with stage-local stderr redirect"
        ));
    }

    let stdin_text = match stdin_path {
        Some(path) => Some(std::fs::read_to_string(path).map_err(|e| format!("{fn_name}: {e}"))?),
        None => None,
    };

    let result = execute_pipeline_value_to_process_result_with_input(
        pipeline,
        stdin_text.as_deref(),
        fn_name,
    )?;

    let stdout = if let Some(path) = stdout_path {
        std::fs::write(path, result.stdout().as_bytes()).map_err(|e| format!("{fn_name}: {e}"))?;
        String::new()
    } else {
        result.stdout().to_string()
    };

    let stderr = if let Some(path) = stderr_path {
        std::fs::write(path, result.stderr().as_bytes()).map_err(|e| format!("{fn_name}: {e}"))?;
        String::new()
    } else {
        result.stderr().to_string()
    };

    Ok(ProcessResultValue::new(
        result.code(),
        result.is_success(),
        stdout,
        stderr,
    ))
}

pub(crate) fn command_value_from_options(
    options: &HashMap<String, Value>,
    fn_name: &str,
) -> Result<CommandValue, String> {
    let program = record_string_required(options, "program", fn_name)?;
    let args = record_string_list_optional(options, "args", fn_name)?.unwrap_or_default();
    let cwd = record_string_optional(options, "cwd", fn_name)?;
    let stdin = record_string_optional(options, "stdin", fn_name)?;
    let env = record_env_optional(options, "env", fn_name)?;

    Ok(CommandValue::new_with_options(
        program, args, cwd, stdin, env,
    ))
}

pub(crate) fn command_value_with_redirects(
    command: &CommandValue,
    redirects: &[RedirectValue],
    fn_name: &str,
) -> Result<CommandValue, String> {
    if redirects.is_empty() {
        return Err(format!("{fn_name}: requires a non-empty List<Redirect>"));
    }
    if command.has_embedded_redirects() {
        return Err(format!(
            "{fn_name}: command already carries embedded redirects"
        ));
    }

    let mut stdout_seen = false;
    let mut stderr_seen = false;
    let mut stdin_seen = false;

    for redirect in redirects {
        match redirect.stream() {
            neve_eval::value::RedirectStream::Stdout => {
                if std::mem::replace(&mut stdout_seen, true) {
                    return Err(format!("{fn_name}: duplicate stdout redirect"));
                }
            }
            neve_eval::value::RedirectStream::Stderr => {
                if std::mem::replace(&mut stderr_seen, true) {
                    return Err(format!("{fn_name}: duplicate stderr redirect"));
                }
            }
            neve_eval::value::RedirectStream::Stdin => {
                if std::mem::replace(&mut stdin_seen, true) {
                    return Err(format!("{fn_name}: duplicate stdin redirect"));
                }
            }
        }
    }

    if command.stdin().is_some() && stdin_seen {
        return Err(format!(
            "{fn_name}: command cannot combine redirect stdin with configured stdin"
        ));
    }

    Ok(CommandValue::new_with_options_and_redirects(
        command.program(),
        command.args().to_vec(),
        command.cwd().map(str::to_owned),
        command.stdin().map(str::to_owned),
        command.env().clone(),
        redirects.to_vec(),
    ))
}

pub(crate) fn pipeline_value_with_redirects(
    pipeline: &PipelineValue,
    redirects: &[RedirectValue],
    fn_name: &str,
) -> Result<PipelineValue, String> {
    if pipeline.commands().is_empty() {
        return Err(format!("{fn_name}: requires a non-empty Pipeline"));
    }
    if redirects.is_empty() {
        return Err(format!("{fn_name}: requires a non-empty List<Redirect>"));
    }
    if pipeline.has_embedded_redirects() {
        return Err(format!(
            "{fn_name}: pipeline already carries embedded redirects"
        ));
    }
    validate_pipeline_command_topology(pipeline.commands(), fn_name)?;

    let mut stdout_seen = false;
    let mut stderr_seen = false;
    let mut stdin_seen = false;

    for redirect in redirects {
        match redirect.stream() {
            neve_eval::value::RedirectStream::Stdout => {
                if std::mem::replace(&mut stdout_seen, true) {
                    return Err(format!("{fn_name}: duplicate stdout redirect"));
                }
            }
            neve_eval::value::RedirectStream::Stderr => {
                if std::mem::replace(&mut stderr_seen, true) {
                    return Err(format!("{fn_name}: duplicate stderr redirect"));
                }
            }
            neve_eval::value::RedirectStream::Stdin => {
                if std::mem::replace(&mut stdin_seen, true) {
                    return Err(format!("{fn_name}: duplicate stdin redirect"));
                }
            }
        }
    }

    let first_stage = pipeline
        .commands()
        .first()
        .expect("non-empty pipeline should have an initial stage");
    let final_stage = pipeline
        .commands()
        .last()
        .expect("non-empty pipeline should have a final stage");

    if stdin_seen
        && (first_stage.stdin().is_some()
            || first_stage.redirects().iter().any(|redirect| {
                matches!(redirect.stream(), neve_eval::value::RedirectStream::Stdin)
            }))
    {
        return Err(format!(
            "{fn_name}: pipeline stage 1 cannot combine boundary stdin with stage-local stdin"
        ));
    }
    if stdout_seen
        && final_stage
            .redirects()
            .iter()
            .any(|redirect| matches!(redirect.stream(), neve_eval::value::RedirectStream::Stdout))
    {
        return Err(format!(
            "{fn_name}: final pipeline stage cannot combine boundary stdout with stage-local stdout redirect"
        ));
    }
    if stderr_seen
        && final_stage
            .redirects()
            .iter()
            .any(|redirect| matches!(redirect.stream(), neve_eval::value::RedirectStream::Stderr))
    {
        return Err(format!(
            "{fn_name}: final pipeline stage cannot combine boundary stderr with stage-local stderr redirect"
        ));
    }

    Ok(PipelineValue::new_with_redirects(
        pipeline.commands().iter().map(Rc::clone).collect(),
        redirects.to_vec(),
    ))
}

pub(crate) fn pipeline_value_from_commands(
    commands: Vec<Rc<CommandValue>>,
    fn_name: &str,
) -> Result<PipelineValue, String> {
    validate_pipeline_command_topology(&commands, fn_name)?;
    Ok(PipelineValue::new(commands))
}

pub(crate) fn validate_pipeline_command_topology(
    commands: &[Rc<CommandValue>],
    fn_name: &str,
) -> Result<(), String> {
    if commands.is_empty() {
        return Err(format!("{fn_name}: requires a non-empty List<Command>"));
    }

    let last_stage_index = commands.len() - 1;
    for (idx, command) in commands.iter().enumerate() {
        if idx > 0 && command.stdin().is_some() {
            return Err(format!(
                "{fn_name}: pipeline stage {} cannot specify stdin",
                idx + 1
            ));
        }
        if idx > 0
            && command.redirects().iter().any(|redirect| {
                matches!(redirect.stream(), neve_eval::value::RedirectStream::Stdin)
            })
        {
            return Err(format!(
                "{fn_name}: pipeline stage {} cannot carry stdin redirect",
                idx + 1
            ));
        }
        if idx < last_stage_index
            && command.redirects().iter().any(|redirect| {
                matches!(redirect.stream(), neve_eval::value::RedirectStream::Stdout)
            })
        {
            return Err(format!(
                "{fn_name}: pipeline stage {} cannot carry stdout redirect before final stage",
                idx + 1
            ));
        }
    }

    Ok(())
}

pub(crate) fn execute_command_value_to_process_result(
    command: &CommandValue,
    fn_name: &str,
) -> Result<ProcessResultValue, String> {
    execute_command_value_to_process_result_with_input(command, command.stdin(), fn_name)
}

pub(crate) fn execute_command_value_to_process_result_with_input(
    command: &CommandValue,
    stdin_text: Option<&str>,
    fn_name: &str,
) -> Result<ProcessResultValue, String> {
    if command.has_embedded_redirects() {
        return execute_command_value_with_redirects_to_process_result_with_input(
            command,
            command.redirects(),
            stdin_text,
            fn_name,
        );
    }

    let mut cmd = configured_process_command(command);

    if let Some(stdin_text) = stdin_text {
        if stdin_text.len() > MAX_STDIN_BYTES {
            return Err(format!("{fn_name}: stdin exceeds maximum size of {MAX_STDIN_BYTES} bytes"));
        }
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("{fn_name}: {e}"))?;
        if let Some(mut pipe) = child.stdin.take() {
            pipe.write_all(stdin_text.as_bytes())
                .map_err(|e| format!("{fn_name}: failed writing stdin: {e}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("{fn_name}: {e}"))?;
        if output.stdout.len() > MAX_OUTPUT_BYTES {
            return Err(format!("{fn_name}: stdout exceeds maximum size of {MAX_OUTPUT_BYTES} bytes"));
        }
        if output.stderr.len() > MAX_OUTPUT_BYTES {
            return Err(format!("{fn_name}: stderr exceeds maximum size of {MAX_OUTPUT_BYTES} bytes"));
        }
        Ok(output_to_process_result_value(output))
    } else {
        let output = cmd
            .output()
            .map_err(|e| format!("{fn_name}: {e}"))?;
        if output.stdout.len() > MAX_OUTPUT_BYTES {
            return Err(format!("{fn_name}: stdout exceeds maximum size of {MAX_OUTPUT_BYTES} bytes"));
        }
        if output.stderr.len() > MAX_OUTPUT_BYTES {
            return Err(format!("{fn_name}: stderr exceeds maximum size of {MAX_OUTPUT_BYTES} bytes"));
        }
        Ok(output_to_process_result_value(output))
    }
}

pub(crate) fn execute_command_value_with_redirects_to_process_result_with_input(
    command: &CommandValue,
    redirects: &[RedirectValue],
    stdin_text: Option<&str>,
    fn_name: &str,
) -> Result<ProcessResultValue, String> {
    if redirects.is_empty() {
        return Err(format!("{fn_name}: requires a non-empty List<Redirect>"));
    }

    let mut stdout_path = None;
    let mut stderr_path = None;
    let mut stdin_path = None;

    for redirect in redirects {
        let resolved = resolve_redirect_path(command, redirect);
        match redirect.stream() {
            neve_eval::value::RedirectStream::Stdout => {
                if stdout_path.replace(resolved).is_some() {
                    return Err(format!("{fn_name}: duplicate stdout redirect"));
                }
            }
            neve_eval::value::RedirectStream::Stderr => {
                if stderr_path.replace(resolved).is_some() {
                    return Err(format!("{fn_name}: duplicate stderr redirect"));
                }
            }
            neve_eval::value::RedirectStream::Stdin => {
                if stdin_path.replace(resolved).is_some() {
                    return Err(format!("{fn_name}: duplicate stdin redirect"));
                }
            }
        }
    }

    if command.stdin().is_some() && stdin_path.is_some() {
        return Err(format!(
            "{fn_name}: command cannot combine redirect stdin with configured stdin"
        ));
    }

    if stdin_text.is_some() && stdin_path.is_some() {
        return Err(format!(
            "{fn_name}: command cannot combine redirect stdin with configured stdin"
        ));
    }

    let stdin_text = match stdin_path {
        Some(path) => Some(std::fs::read_to_string(path).map_err(|e| format!("{fn_name}: {e}"))?),
        None => stdin_text
            .map(str::to_owned)
            .or_else(|| command.stdin().map(str::to_owned)),
    };

    if let Some(ref text) = stdin_text {
        if text.len() > MAX_STDIN_BYTES {
            return Err(format!("{fn_name}: stdin exceeds maximum size of {MAX_STDIN_BYTES} bytes"));
        }
    }

    let mut cmd = configured_process_command(command);

    if let Some(path) = stdout_path {
        let file = std::fs::File::create(path).map_err(|e| format!("{fn_name}: {e}"))?;
        cmd.stdout(std::process::Stdio::from(file));
    } else {
        cmd.stdout(std::process::Stdio::piped());
    }

    if let Some(path) = stderr_path {
        let file = std::fs::File::create(path).map_err(|e| format!("{fn_name}: {e}"))?;
        cmd.stderr(std::process::Stdio::from(file));
    } else {
        cmd.stderr(std::process::Stdio::piped());
    }

    if let Some(stdin_text) = stdin_text.as_deref() {
        cmd.stdin(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("{fn_name}: {e}"))?;
        if let Some(mut pipe) = child.stdin.take() {
            pipe.write_all(stdin_text.as_bytes())
                .map_err(|e| format!("{fn_name}: failed writing stdin: {e}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("{fn_name}: {e}"))?;
        if output.stdout.len() > MAX_OUTPUT_BYTES {
            return Err(format!("{fn_name}: stdout exceeds maximum size of {MAX_OUTPUT_BYTES} bytes"));
        }
        if output.stderr.len() > MAX_OUTPUT_BYTES {
            return Err(format!("{fn_name}: stderr exceeds maximum size of {MAX_OUTPUT_BYTES} bytes"));
        }
        Ok(output_to_process_result_value(output))
    } else {
        cmd.stdin(std::process::Stdio::null());
        let output = cmd.spawn()
            .map_err(|e| format!("{fn_name}: {e}"))?
            .wait_with_output()
            .map_err(|e| format!("{fn_name}: {e}"))?;
        if output.stdout.len() > MAX_OUTPUT_BYTES {
            return Err(format!("{fn_name}: stdout exceeds maximum size of {MAX_OUTPUT_BYTES} bytes"));
        }
        if output.stderr.len() > MAX_OUTPUT_BYTES {
            return Err(format!("{fn_name}: stderr exceeds maximum size of {MAX_OUTPUT_BYTES} bytes"));
        }
        Ok(output_to_process_result_value(output))
    }
}

pub(crate) fn configured_process_command(command: &CommandValue) -> std::process::Command {
    let mut cmd = std::process::Command::new(command.program());
    cmd.args(command.args());

    if let Some(cwd) = command.cwd() {
        cmd.current_dir(cwd);
    }
    for (k, v) in command.env() {
        cmd.env(k, v);
    }
    // Strip dangerous environment variables that could cause arbitrary code execution
    cmd.env_remove("LD_PRELOAD");
    cmd.env_remove("LD_LIBRARY_PATH");
    cmd.env_remove("DYLD_INSERT_LIBRARIES");
    cmd.env_remove("DYLD_LIBRARY_PATH");

    cmd
}

pub(crate) fn resolve_redirect_path(
    command: &CommandValue,
    redirect: &RedirectValue,
) -> std::path::PathBuf {
    let path = redirect.path();
    if path.components().any(|c| c == std::path::Component::ParentDir) {
        return std::path::PathBuf::from("/dev/null/neve-blocked-traversal");
    }
    if path.is_relative()
        && let Some(cwd) = command.cwd()
    {
        return std::path::PathBuf::from(cwd).join(path);
    }
    path.clone()
}

fn resolve_pipeline_redirect_path(
    pipeline: &PipelineValue,
    redirect: &RedirectValue,
) -> std::path::PathBuf {
    let command = match redirect.stream() {
        neve_eval::value::RedirectStream::Stdin => pipeline
            .commands()
            .first()
            .expect("non-empty pipeline should have an initial stage"),
        neve_eval::value::RedirectStream::Stdout | neve_eval::value::RedirectStream::Stderr => {
            pipeline
                .commands()
                .last()
                .expect("non-empty pipeline should have a final stage")
        }
    };
    resolve_redirect_path(command, redirect)
}

pub(crate) fn record_string_required(
    options: &HashMap<String, Value>,
    key: &str,
    fn_name: &str,
) -> Result<String, String> {
    match options.get(key) {
        Some(Value::String(s)) => Ok(s.to_string()),
        Some(_) => Err(format!("{fn_name}.{key} must be String")),
        None => Err(format!("{fn_name} requires '{key}'")),
    }
}

pub(crate) fn record_string_optional(
    options: &HashMap<String, Value>,
    key: &str,
    fn_name: &str,
) -> Result<Option<String>, String> {
    match options.get(key) {
        Some(Value::String(s)) => Ok(Some(s.to_string())),
        Some(_) => Err(format!("{fn_name}.{key} must be String")),
        None => Ok(None),
    }
}

pub(crate) fn record_string_list_optional(
    options: &HashMap<String, Value>,
    key: &str,
    fn_name: &str,
) -> Result<Option<Vec<String>>, String> {
    match options.get(key) {
        Some(Value::List(items)) => {
            list_to_string_vec(items, &format!("{fn_name}.{key}")).map(Some)
        }
        Some(_) => Err(format!("{fn_name}.{key} must be List<String>")),
        None => Ok(None),
    }
}

pub(crate) fn record_env_optional(
    options: &HashMap<String, Value>,
    key: &str,
    fn_name: &str,
) -> Result<HashMap<String, String>, String> {
    match options.get(key) {
        Some(Value::Record(fields)) => {
            let mut env = HashMap::new();
            for (k, v) in fields.iter() {
                if let Value::String(val) = v {
                    env.insert(k.clone(), val.to_string());
                } else {
                    return Err(format!("{fn_name}.{key}.{k} must be String"));
                }
            }
            Ok(env)
        }
        Some(_) => Err(format!("{fn_name}.{key} must be Record<String, String>")),
        None => Ok(HashMap::new()),
    }
}



/// Kill a process by PID. Uses libc::kill on Unix, taskkill on Windows.
/// 通过 PID 终止进程。
// kill_process moved to neve_common::kill_process (M-2 unified kill mechanism)


/// Run pipeline stages sequentially in a background thread.
fn run_pipeline_stages(stages: &[StageData]) -> Result<(i32, bool, String, String), String> {
    if stages.is_empty() { return Err("empty pipeline".to_string()); }
    let mut previous_stdout: Option<Vec<u8>> = None;
    let mut combined_stderr = Vec::new();
    let mut last_code = 0;
    let mut last_success = false;
    let last_idx = stages.len() - 1;
    for (idx, stage) in stages.iter().enumerate() {
        let mut c = std::process::Command::new(&stage.program);
        c.args(&stage.args);
        if let Some(ref wd) = stage.cwd { c.current_dir(wd); }
        for (k, v) in &stage.env { c.env(k, v); }
        let stage_stdin = if idx == 0 { stage.stdin.as_ref().map(|s| s.as_bytes().to_vec()).or_else(|| previous_stdout.take()) } else { previous_stdout.take() };
        if stage_stdin.is_some() { c.stdin(std::process::Stdio::piped()); } else { c.stdin(std::process::Stdio::null()); }
        c.stdout(std::process::Stdio::piped());
        c.stderr(std::process::Stdio::piped());
        let mut child = c.spawn().map_err(|e| format!("stage {idx}: {e}"))?;
        if let Some(ref data) = stage_stdin {
            if idx == 0 && data.len() > MAX_STDIN_BYTES {
                return Err(format!("stdin exceeds maximum size of {MAX_STDIN_BYTES} bytes"));
            }
            use std::io::Write;
            if let Some(mut pipe) = child.stdin.take() { pipe.write_all(data).map_err(|e| format!("stdin: {e}"))?; }
        }
        let output = child.wait_with_output().map_err(|e| format!("wait: {e}"))?;
        if output.stdout.len() > MAX_OUTPUT_BYTES {
            return Err(format!("stdout exceeds maximum size of {MAX_OUTPUT_BYTES} bytes"));
        }
        if output.stderr.len() > MAX_OUTPUT_BYTES {
            return Err(format!("stderr exceeds maximum size of {MAX_OUTPUT_BYTES} bytes"));
        }
        last_code = output.status.code().unwrap_or(-1);
        last_success = output.status.success();
        combined_stderr.extend_from_slice(&output.stderr);
        if idx < last_idx { previous_stdout = Some(output.stdout); }
        else {
            let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr_str = String::from_utf8_lossy(&combined_stderr).to_string();
            return Ok((last_code, last_success, stdout_str, stderr_str));
        }
    }
    Err("unreachable".to_string())
}

struct StageData {
    program: String,
    args: Vec<String>,
    cwd: Option<String>,
    env: HashMap<String, String>,
    stdin: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn test_sha256_hex_known_vector() {
        let hash = sha256_hex(b"abc");
        assert_eq!(
            hash,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_sha256_hex_length() {
        let hash = sha256_hex(b"hello");
        assert_eq!(hash.len(), 64);
    }
}
