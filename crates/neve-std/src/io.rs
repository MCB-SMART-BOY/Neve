//! IO operations for the standard library.
//! 标准库的 IO 操作。
//!
//! These are impure operations that interact with the file system.
//! They are primarily used during package builds and configuration generation.
//! 这些是与文件系统交互的非纯操作。
//! 主要用于包构建和配置生成期间。

use neve_eval::value::{
    BuiltinFn, CommandValue, EventKind, EventValue, LiveValue, PipelineValue, ProcessResultValue,
    RedirectValue, TaskTargetValue, TaskValue, Value,
};
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

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

/// Returns all IO builtins.
/// 返回所有 IO 内置函数。
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // Events / 事件
        // Reactive / 反应式
        (
            "io.reactive",
            Value::Builtin(BuiltinFn {
                name: "io.reactive",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Event(event) => Ok(Value::Live(Rc::new(LiveValue {
                        event: Rc::clone(event),
                        current: Rc::new(std::cell::RefCell::new(None)),
                        cancelled: Rc::new(std::cell::Cell::new(false)),
                    }))),
                    _ => Err("io.reactive expects an Event".to_string()),
                },
            }),
        ),
        (
            "io.liveNext",
            Value::Builtin(BuiltinFn {
                name: "io.liveNext",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Live(live) => {
                        if live.cancelled.get() {
                            return Err("io.liveNext: live cancelled".to_string());
                        }
                        // Poll the source event
                        let val = crate::io::poll_event(&live.event)?;
                        // Cache the value
                        *live.current.borrow_mut() = Some(val.clone());
                        Ok(val)
                    }
                    _ => Err("io.liveNext expects a Live value".to_string()),
                },
            }),
        ),
        (
            "io.liveCurrent",
            Value::Builtin(BuiltinFn {
                name: "io.liveCurrent",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Live(live) => {
                        let current = live.current.borrow();
                        Ok(match current.as_ref() {
                            Some(v) => Value::Some(Box::new(v.clone())),
                            None => Value::None,
                        })
                    }
                    _ => Err("io.liveCurrent expects a Live value".to_string()),
                },
            }),
        ),
        (
            "io.liveCancel",
            Value::Builtin(BuiltinFn {
                name: "io.liveCancel",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Live(live) => {
                        live.cancelled.set(true);
                        Ok(Value::Unit)
                    }
                    _ => Err("io.liveCancel expects a Live value".to_string()),
                },
            }),
        ),
        (
            "io.eventMap",
            Value::Builtin(BuiltinFn {
                name: "io.eventMap",
                arity: 2,
                func: |_args| Err("io.eventMap is evaluator-owned".to_string()),
            }),
        ),
        (
            "io.eventFilter",
            Value::Builtin(BuiltinFn {
                name: "io.eventFilter",
                arity: 2,
                func: |_args| Err("io.eventFilter is evaluator-owned".to_string()),
            }),
        ),
        (
            "io.watchFile",
            Value::Builtin(BuiltinFn {
                name: "io.watchFile",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => {
                        let path = std::path::PathBuf::from(path.as_str());
                        Ok(Value::Event(Rc::new(EventValue {
                            kind: EventKind::FileWatch { path },
                        })))
                    }
                    Value::Path(path) => Ok(Value::Event(Rc::new(EventValue {
                        kind: EventKind::FileWatch {
                            path: std::path::PathBuf::from(path.as_ref()),
                        },
                    }))),
                    _ => Err("io.watchFile expects a String or Path".to_string()),
                },
            }),
        ),
        (
            "io.eventNext",
            Value::Builtin(BuiltinFn {
                name: "io.eventNext",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Event(event) => poll_event(event),
                    _ => Err("io.eventNext expects an Event".to_string()),
                },
            }),
        ),
        (
            "io.every",
            Value::Builtin(BuiltinFn {
                name: "io.every",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Int(ms) => {
                        let ms: u64 = ms
                            .try_into()
                            .map_err(|_| "io.every: interval must be non-negative".to_string())?;
                        Ok(Value::Event(Rc::new(EventValue {
                            kind: EventKind::Timer { interval_ms: ms },
                        })))
                    }
                    _ => Err("io.every expects an Int (milliseconds)".to_string()),
                },
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
        // File reading / 文件读取
        (
            "io.readFile",
            Value::Builtin(BuiltinFn {
                name: "io.readFile",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => std::fs::read_to_string(path.as_str())
                        .map(|s| Value::String(Rc::new(s)))
                        .map_err(|e| format!("io.readFile: {e}")),
                    _ => Err("io.readFile expects a string path".to_string()),
                },
            }),
        ),
        (
            "io.readFilePath",
            Value::Builtin(BuiltinFn {
                name: "io.readFilePath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => std::fs::read_to_string(path.as_path())
                        .map(|s| Value::String(Rc::new(s)))
                        .map_err(|e| format!("io.readFilePath: {e}")),
                    _ => Err("io.readFilePath expects a Path".to_string()),
                },
            }),
        ),
        (
            "io.readFileBytesPath",
            Value::Builtin(BuiltinFn {
                name: "io.readFileBytesPath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => std::fs::read(path.as_path())
                        .map(|bytes| Value::Bytes(Rc::new(bytes)))
                        .map_err(|e| format!("io.readFileBytesPath: {e}")),
                    _ => Err("io.readFileBytesPath expects a Path".to_string()),
                },
            }),
        ),
        (
            "io.readDir",
            Value::Builtin(BuiltinFn {
                name: "io.readDir",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => {
                        let entries: Result<Vec<_>, _> = std::fs::read_dir(path.as_str())
                            .map_err(|e| format!("io.readDir: {e}"))?
                            .map(|entry| {
                                entry
                                    .map(|e| {
                                        Value::String(Rc::new(
                                            e.file_name().to_string_lossy().to_string(),
                                        ))
                                    })
                                    .map_err(|e| format!("io.readDir: {e}"))
                            })
                            .collect();
                        entries.map(|v| Value::List(Rc::new(v)))
                    }
                    _ => Err("io.readDir expects a string path".to_string()),
                },
            }),
        ),
        (
            "io.readDirPath",
            Value::Builtin(BuiltinFn {
                name: "io.readDirPath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => {
                        let entries: Result<Vec<_>, _> = std::fs::read_dir(path.as_path())
                            .map_err(|e| format!("io.readDirPath: {e}"))?
                            .map(|entry| {
                                entry
                                    .map(|e| {
                                        Value::String(Rc::new(
                                            e.file_name().to_string_lossy().to_string(),
                                        ))
                                    })
                                    .map_err(|e| format!("io.readDirPath: {e}"))
                            })
                            .collect();
                        entries.map(|v| Value::List(Rc::new(v)))
                    }
                    _ => Err("io.readDirPath expects a Path".to_string()),
                },
            }),
        ),
        (
            "io.readDirEntryPaths",
            Value::Builtin(BuiltinFn {
                name: "io.readDirEntryPaths",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => {
                        let entries: Result<Vec<_>, _> = std::fs::read_dir(path.as_path())
                            .map_err(|e| format!("io.readDirEntryPaths: {e}"))?
                            .map(|entry| {
                                entry
                                    .map(|e| Value::Path(Rc::new(e.path())))
                                    .map_err(|e| format!("io.readDirEntryPaths: {e}"))
                            })
                            .collect();
                        entries.map(|v| Value::List(Rc::new(v)))
                    }
                    _ => Err("io.readDirEntryPaths expects a Path".to_string()),
                },
            }),
        ),
        (
            "io.writeFile",
            Value::Builtin(BuiltinFn {
                name: "io.writeFile",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => {
                        std::fs::write(path.as_str(), content.as_bytes())
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.writeFile: {e}"))
                    }
                    _ => Err("io.writeFile expects (String, String)".to_string()),
                },
            }),
        ),
        (
            "io.writeFilePath",
            Value::Builtin(BuiltinFn {
                name: "io.writeFilePath",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Path(path), Value::String(content)) => {
                        std::fs::write(path.as_path(), content.as_bytes())
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.writeFilePath: {e}"))
                    }
                    _ => Err("io.writeFilePath expects (Path, String)".to_string()),
                },
            }),
        ),
        (
            "io.writeFileBytesPath",
            Value::Builtin(BuiltinFn {
                name: "io.writeFileBytesPath",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Path(path), Value::Bytes(bytes)) => {
                        std::fs::write(path.as_path(), bytes.as_ref())
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.writeFileBytesPath: {e}"))
                    }
                    _ => Err("io.writeFileBytesPath expects (Path, Bytes)".to_string()),
                },
            }),
        ),
        (
            "io.appendFile",
            Value::Builtin(BuiltinFn {
                name: "io.appendFile",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path.as_str())
                        .and_then(|mut f| f.write_all(content.as_bytes()))
                        .map(|_| Value::Unit)
                        .map_err(|e| format!("io.appendFile: {e}")),
                    _ => Err("io.appendFile expects (String, String)".to_string()),
                },
            }),
        ),
        (
            "io.appendFilePath",
            Value::Builtin(BuiltinFn {
                name: "io.appendFilePath",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Path(path), Value::String(content)) => std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path.as_path())
                        .and_then(|mut f| f.write_all(content.as_bytes()))
                        .map(|_| Value::Unit)
                        .map_err(|e| format!("io.appendFilePath: {e}")),
                    _ => Err("io.appendFilePath expects (Path, String)".to_string()),
                },
            }),
        ),
        (
            "io.appendFileBytesPath",
            Value::Builtin(BuiltinFn {
                name: "io.appendFileBytesPath",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Path(path), Value::Bytes(bytes)) => std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path.as_path())
                        .and_then(|mut f| f.write_all(bytes.as_ref()))
                        .map(|_| Value::Unit)
                        .map_err(|e| format!("io.appendFileBytesPath: {e}")),
                    _ => Err("io.appendFileBytesPath expects (Path, Bytes)".to_string()),
                },
            }),
        ),
        (
            "io.createDirAll",
            Value::Builtin(BuiltinFn {
                name: "io.createDirAll",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => std::fs::create_dir_all(path.as_str())
                        .map(|_| Value::Unit)
                        .map_err(|e| format!("io.createDirAll: {e}")),
                    _ => Err("io.createDirAll expects a string path".to_string()),
                },
            }),
        ),
        (
            "io.createDirAllPath",
            Value::Builtin(BuiltinFn {
                name: "io.createDirAllPath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => std::fs::create_dir_all(path.as_path())
                        .map(|_| Value::Unit)
                        .map_err(|e| format!("io.createDirAllPath: {e}")),
                    _ => Err("io.createDirAllPath expects a Path".to_string()),
                },
            }),
        ),
        (
            "io.removeDirAll",
            Value::Builtin(BuiltinFn {
                name: "io.removeDirAll",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => std::fs::remove_dir_all(path.as_str())
                        .map(|_| Value::Unit)
                        .map_err(|e| format!("io.removeDirAll: {e}")),
                    _ => Err("io.removeDirAll expects a string path".to_string()),
                },
            }),
        ),
        (
            "io.removeDirAllPath",
            Value::Builtin(BuiltinFn {
                name: "io.removeDirAllPath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => std::fs::remove_dir_all(path.as_path())
                        .map(|_| Value::Unit)
                        .map_err(|e| format!("io.removeDirAllPath: {e}")),
                    _ => Err("io.removeDirAllPath expects a Path".to_string()),
                },
            }),
        ),
        // File checks / 文件检查
        (
            "io.pathExists",
            Value::Builtin(BuiltinFn {
                name: "io.pathExists",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => {
                        Ok(Value::Bool(std::path::Path::new(path.as_str()).exists()))
                    }
                    _ => Err("io.pathExists expects a string path".to_string()),
                },
            }),
        ),
        (
            "io.pathExistsPath",
            Value::Builtin(BuiltinFn {
                name: "io.pathExistsPath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => Ok(Value::Bool(path.exists())),
                    _ => Err("io.pathExistsPath expects a Path".to_string()),
                },
            }),
        ),
        (
            "io.isDir",
            Value::Builtin(BuiltinFn {
                name: "io.isDir",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => {
                        Ok(Value::Bool(std::path::Path::new(path.as_str()).is_dir()))
                    }
                    _ => Err("io.isDir expects a string path".to_string()),
                },
            }),
        ),
        (
            "io.isDirPath",
            Value::Builtin(BuiltinFn {
                name: "io.isDirPath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => Ok(Value::Bool(path.is_dir())),
                    _ => Err("io.isDirPath expects a Path".to_string()),
                },
            }),
        ),
        (
            "io.isFile",
            Value::Builtin(BuiltinFn {
                name: "io.isFile",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => {
                        Ok(Value::Bool(std::path::Path::new(path.as_str()).is_file()))
                    }
                    _ => Err("io.isFile expects a string path".to_string()),
                },
            }),
        ),
        (
            "io.isFilePath",
            Value::Builtin(BuiltinFn {
                name: "io.isFilePath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => Ok(Value::Bool(path.is_file())),
                    _ => Err("io.isFilePath expects a Path".to_string()),
                },
            }),
        ),
        // Environment / 环境变量
        (
            "io.getEnv",
            Value::Builtin(BuiltinFn {
                name: "io.getEnv",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(name) => match std::env::var(name.as_str()) {
                        Ok(val) => Ok(Value::Some(Box::new(Value::String(Rc::new(val))))),
                        Err(_) => Ok(Value::None),
                    },
                    _ => Err("io.getEnv expects a string".to_string()),
                },
            }),
        ),
        (
            "io.env",
            Value::Builtin(BuiltinFn {
                name: "io.env",
                arity: 0,
                func: |_args| {
                    let mut fields = Vec::new();
                    for (key, value) in std::env::vars() {
                        fields.push((key, Value::String(Rc::new(value))));
                    }
                    Ok(Value::Record(Rc::new(
                        fields.into_iter().collect::<HashMap<_, _>>(),
                    )))
                },
            }),
        ),
        (
            "io.sleep",
            Value::Builtin(BuiltinFn {
                name: "io.sleep",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Int(ms) => {
                        let ms: u64 = ms
                            .try_into()
                            .map_err(|_| "io.sleep: timeout must be non-negative".to_string())?;
                        std::thread::sleep(Duration::from_millis(ms));
                        Ok(Value::Unit)
                    }
                    _ => Err("io.sleep expects an integer (milliseconds)".to_string()),
                },
            }),
        ),
        (
            "io.which",
            Value::Builtin(BuiltinFn {
                name: "io.which",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(cmd) => {
                        let path = std::env::var_os("PATH").unwrap_or_default();
                        let found = std::env::split_paths(&path).find_map(|dir| {
                            let full = dir.join(cmd.as_str());
                            if full.is_file() { Some(full) } else { None }
                        });
                        Ok(match found {
                            Some(p) => Value::Some(Box::new(Value::String(Rc::new(
                                p.to_string_lossy().to_string(),
                            )))),
                            None => Value::None,
                        })
                    }
                    _ => Err("io.which expects a command name string".to_string()),
                },
            }),
        ),
        (
            "io.currentDir",
            Value::Builtin(BuiltinFn {
                name: "io.currentDir",
                arity: 0,
                func: |_args| {
                    std::env::current_dir()
                        .map(|p| Value::String(Rc::new(p.to_string_lossy().to_string())))
                        .map_err(|e| format!("io.currentDir: {e}"))
                },
            }),
        ),
        (
            "io.currentDirPath",
            Value::Builtin(BuiltinFn {
                name: "io.currentDirPath",
                arity: 0,
                func: |_args| {
                    std::env::current_dir()
                        .map(|p| Value::Path(Rc::new(p)))
                        .map_err(|e| format!("io.currentDirPath: {e}"))
                },
            }),
        ),
        (
            "io.homeDirPath",
            Value::Builtin(BuiltinFn {
                name: "io.homeDirPath",
                arity: 0,
                func: |_args| {
                    Ok(std::env::var("HOME")
                        .map(|p| Value::Some(Box::new(Value::Path(Rc::new(p.into())))))
                        .unwrap_or(Value::None))
                },
            }),
        ),
        (
            "io.args",
            Value::Builtin(BuiltinFn {
                name: "io.args",
                arity: 0,
                func: |_args| {
                    let guard = SCRIPT_ARGS.read().map_err(|e| format!("io.args: {e}"))?;
                    let args: Vec<Value> = guard
                        .iter()
                        .map(|arg| Value::String(Rc::new(arg.clone())))
                        .collect();
                    Ok(Value::List(Rc::new(args)))
                },
            }),
        ),
        (
            "io.command",
            Value::Builtin(BuiltinFn {
                name: "io.command",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(program), Value::List(argv)) => {
                        let argv = list_to_string_vec(argv, "io.command args")?;
                        Ok(Value::Command(Rc::new(CommandValue::new(
                            program.as_str(),
                            argv,
                        ))))
                    }
                    _ => Err("io.command expects (String, List<String>)".to_string()),
                },
            }),
        ),
        (
            "io.commandWith",
            Value::Builtin(BuiltinFn {
                name: "io.commandWith",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Record(options) => {
                        let command = command_value_from_options(options, "io.commandWith")?;
                        Ok(Value::Command(Rc::new(command)))
                    }
                    _ => Err("io.commandWith expects a record options object".to_string()),
                },
            }),
        ),
        (
            "io.commandWithRedirects",
            Value::Builtin(BuiltinFn {
                name: "io.commandWithRedirects",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Command(command), Value::List(redirects)) => {
                        let redirects =
                            list_to_redirect_vec(redirects, "io.commandWithRedirects redirects")?;
                        let command = command_value_with_redirects(
                            command,
                            &redirects,
                            "io.commandWithRedirects",
                        )?;
                        Ok(Value::Command(Rc::new(command)))
                    }
                    _ => {
                        Err("io.commandWithRedirects expects (Command, List<Redirect>)".to_string())
                    }
                },
            }),
        ),
        (
            "io.execCommand",
            Value::Builtin(BuiltinFn {
                name: "io.execCommand",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Command(command) => execute_command_value(command),
                    _ => Err("io.execCommand expects a Command".to_string()),
                },
            }),
        ),
        (
            "io.execCommandLines",
            Value::Builtin(BuiltinFn {
                name: "io.execCommandLines",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Command(command) => execute_command_lines(command),
                    _ => Err("io.execCommandLines expects a Command".to_string()),
                },
            }),
        ),
        (
            "io.pipeline",
            Value::Builtin(BuiltinFn {
                name: "io.pipeline",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(commands) => {
                        let commands = list_to_command_vec(commands, "io.pipeline commands")?;
                        let pipeline = pipeline_value_from_commands(commands, "io.pipeline")?;
                        Ok(Value::Pipeline(Rc::new(pipeline)))
                    }
                    _ => Err("io.pipeline expects List<Command>".to_string()),
                },
            }),
        ),
        (
            "io.pipelineWithRedirects",
            Value::Builtin(BuiltinFn {
                name: "io.pipelineWithRedirects",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Pipeline(pipeline), Value::List(redirects)) => {
                        let redirects =
                            list_to_redirect_vec(redirects, "io.pipelineWithRedirects redirects")?;
                        let pipeline = pipeline_value_with_redirects(
                            pipeline,
                            &redirects,
                            "io.pipelineWithRedirects",
                        )?;
                        Ok(Value::Pipeline(Rc::new(pipeline)))
                    }
                    _ => Err(
                        "io.pipelineWithRedirects expects (Pipeline, List<Redirect>)".to_string(),
                    ),
                },
            }),
        ),
        (
            "io.execPipeline",
            Value::Builtin(BuiltinFn {
                name: "io.execPipeline",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Pipeline(pipeline) => execute_pipeline_value(pipeline),
                    _ => Err("io.execPipeline expects a Pipeline".to_string()),
                },
            }),
        ),
        (
            "io.redirectStdoutPath",
            Value::Builtin(BuiltinFn {
                name: "io.redirectStdoutPath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => Ok(Value::Redirect(Rc::new(RedirectValue::stdout_path(
                        path.as_path(),
                    )))),
                    _ => Err("io.redirectStdoutPath expects a Path".to_string()),
                },
            }),
        ),
        (
            "io.redirectStderrPath",
            Value::Builtin(BuiltinFn {
                name: "io.redirectStderrPath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => Ok(Value::Redirect(Rc::new(RedirectValue::stderr_path(
                        path.as_path(),
                    )))),
                    _ => Err("io.redirectStderrPath expects a Path".to_string()),
                },
            }),
        ),
        (
            "io.redirectStdinPath",
            Value::Builtin(BuiltinFn {
                name: "io.redirectStdinPath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => Ok(Value::Redirect(Rc::new(RedirectValue::stdin_path(
                        path.as_path(),
                    )))),
                    _ => Err("io.redirectStdinPath expects a Path".to_string()),
                },
            }),
        ),
        (
            "io.taskCommand",
            Value::Builtin(BuiltinFn {
                name: "io.taskCommand",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Command(command) => Ok(Value::Task(Rc::new(
                        TaskValue::command_process_result(Rc::clone(command)),
                    ))),
                    _ => Err("io.taskCommand expects a Command".to_string()),
                },
            }),
        ),
        (
            "io.taskPipeline",
            Value::Builtin(BuiltinFn {
                name: "io.taskPipeline",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Pipeline(pipeline) => Ok(Value::Task(Rc::new(
                        TaskValue::pipeline_process_result(Rc::clone(pipeline)),
                    ))),
                    _ => Err("io.taskPipeline expects a Pipeline".to_string()),
                },
            }),
        ),
        (
            "io.awaitTask",
            Value::Builtin(BuiltinFn {
                name: "io.awaitTask",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Task(task) => await_task(task),
                    _ => Err("io.awaitTask expects a Task[ProcessResult]".to_string()),
                },
            }),
        ),
        (
            "io.awaitTasks",
            Value::Builtin(BuiltinFn {
                name: "io.awaitTasks",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(tasks) => {
                        let tasks = list_to_task_vec(tasks, "io.awaitTasks tasks")?;
                        await_tasks(&tasks)
                    }
                    _ => Err("io.awaitTasks expects List<Task[ProcessResult]>".to_string()),
                },
            }),
        ),
        (
            "io.awaitTaskWithTimeout",
            Value::Builtin(BuiltinFn {
                name: "io.awaitTaskWithTimeout",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Task(task), Value::Int(timeout_ms)) => {
                        let timeout_ms: u64 = timeout_ms.try_into().map_err(|_| {
                            "io.awaitTaskWithTimeout: timeout must be non-negative".to_string()
                        })?;
                        await_task_with_timeout(task, timeout_ms)
                    }
                    _ => {
                        Err("io.awaitTaskWithTimeout expects (Task[ProcessResult], Int)"
                            .to_string())
                    }
                },
            }),
        ),
        (
            "io.processSuccess",
            Value::Builtin(BuiltinFn {
                name: "io.processSuccess",
                arity: 1,
                func: |args| match &args[0] {
                    Value::ProcessResult(result) => Ok(Value::Bool(result.is_success())),
                    _ => Err("io.processSuccess expects a ProcessResult".to_string()),
                },
            }),
        ),
        (
            "io.processStdout",
            Value::Builtin(BuiltinFn {
                name: "io.processStdout",
                arity: 1,
                func: |args| match &args[0] {
                    Value::ProcessResult(result) => {
                        Ok(Value::String(Rc::new(result.stdout().to_string())))
                    }
                    _ => Err("io.processStdout expects a ProcessResult".to_string()),
                },
            }),
        ),
        (
            "io.processCode",
            Value::Builtin(BuiltinFn {
                name: "io.processCode",
                arity: 1,
                func: |args| match &args[0] {
                    Value::ProcessResult(result) => Ok(Value::Int(result.code().into())),
                    _ => Err("io.processCode expects a ProcessResult".to_string()),
                },
            }),
        ),
        (
            "io.processStderr",
            Value::Builtin(BuiltinFn {
                name: "io.processStderr",
                arity: 1,
                func: |args| match &args[0] {
                    Value::ProcessResult(result) => {
                        Ok(Value::String(Rc::new(result.stderr().to_string())))
                    }
                    _ => Err("io.processStderr expects a ProcessResult".to_string()),
                },
            }),
        ),
        (
            "io.homeDir",
            Value::Builtin(BuiltinFn {
                name: "io.homeDir",
                arity: 0,
                func: |_args| {
                    Ok(std::env::var("HOME")
                        .map(|p| Value::Some(Box::new(Value::String(Rc::new(p)))))
                        .unwrap_or(Value::None))
                },
            }),
        ),
        // Hashing (useful for content-addressed store)
        // 哈希（用于内容寻址存储）
        (
            "io.hashFile",
            Value::Builtin(BuiltinFn {
                name: "io.hashFile",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => {
                        let content = std::fs::read(path.as_str())
                            .map_err(|e| format!("io.hashFile: {e}"))?;
                        let hash = sha256_hex(&content);
                        Ok(Value::String(Rc::new(hash)))
                    }
                    _ => Err("io.hashFile expects a string path".to_string()),
                },
            }),
        ),
        (
            "io.hashFilePath",
            Value::Builtin(BuiltinFn {
                name: "io.hashFilePath",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(path) => {
                        let content = std::fs::read(path.as_path())
                            .map_err(|e| format!("io.hashFilePath: {e}"))?;
                        let hash = sha256_hex(&content);
                        Ok(Value::String(Rc::new(hash)))
                    }
                    _ => Err("io.hashFilePath expects a Path".to_string()),
                },
            }),
        ),
        (
            "io.hashString",
            Value::Builtin(BuiltinFn {
                name: "io.hashString",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(s) => {
                        let hash = sha256_hex(s.as_bytes());
                        Ok(Value::String(Rc::new(hash)))
                    }
                    _ => Err("io.hashString expects a string".to_string()),
                },
            }),
        ),
        // System info / 系统信息
        (
            "io.currentSystem",
            Value::Builtin(BuiltinFn {
                name: "io.currentSystem",
                arity: 0,
                func: |_args| {
                    let arch = std::env::consts::ARCH;
                    let os = std::env::consts::OS;
                    Ok(Value::String(Rc::new(format!("{}-{}", arch, os))))
                },
            }),
        ),
        (
            "io.execCommandStreaming",
            Value::Builtin(BuiltinFn {
                name: "io.execCommandStreaming",
                arity: 2,
                func: |_args| Err("io.execCommandStreaming is evaluator-owned".to_string()),
            }),
        ),
        (
            "io.execPipelineStreaming",
            Value::Builtin(BuiltinFn {
                name: "io.execPipelineStreaming",
                arity: 2,
                func: |_args| Err("io.execPipelineStreaming is evaluator-owned".to_string()),
            }),
        ),
        (
            "io.readFileLines",
            Value::Builtin(BuiltinFn {
                name: "io.readFileLines",
                arity: 2,
                func: |_args| Err("io.readFileLines is evaluator-owned".to_string()),
            }),
        ),
        (
            "io.readFileLinesPath",
            Value::Builtin(BuiltinFn {
                name: "io.readFileLinesPath",
                arity: 2,
                func: |_args| Err("io.readFileLinesPath is evaluator-owned".to_string()),
            }),
        ),
        // Atomic file operations / 原子文件操作
        (
            "io.atomicWrite",
            Value::Builtin(BuiltinFn {
                name: "io.atomicWrite",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => {
                        atomic_write(path.as_str(), content.as_bytes())
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.atomicWrite: {e}"))
                    }
                    _ => Err("io.atomicWrite expects (String, String)".to_string()),
                },
            }),
        ),
        (
            "io.atomicWritePath",
            Value::Builtin(BuiltinFn {
                name: "io.atomicWritePath",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Path(path), Value::String(content)) => {
                        atomic_write_path(path.as_path(), content.as_bytes())
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.atomicWritePath: {e}"))
                    }
                    _ => Err("io.atomicWritePath expects (Path, String)".to_string()),
                },
            }),
        ),
        (
            "io.atomicWriteAll",
            Value::Builtin(BuiltinFn {
                name: "io.atomicWriteAll",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(entries) => atomic_write_all(entries)
                        .map(|_| Value::Unit)
                        .map_err(|e| format!("io.atomicWriteAll: {e}")),
                    _ => Err(
                        "io.atomicWriteAll expects List[{path: String, content: String}]"
                            .to_string(),
                    ),
                },
            }),
        ),
        (
            "io.copy",
            Value::Builtin(BuiltinFn {
                name: "io.copy",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(src), Value::String(dst)) => {
                        std::fs::copy(src.as_str(), dst.as_str())
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.copy: {e}"))
                    }
                    _ => Err("io.copy expects (String, String)".to_string()),
                },
            }),
        ),
        (
            "io.copyPath",
            Value::Builtin(BuiltinFn {
                name: "io.copyPath",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Path(src), Value::Path(dst)) => {
                        std::fs::copy(src.as_path(), dst.as_path())
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.copyPath: {e}"))
                    }
                    _ => Err("io.copyPath expects (Path, Path)".to_string()),
                },
            }),
        ),
        (
            "io.move",
            Value::Builtin(BuiltinFn {
                name: "io.move",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(src), Value::String(dst)) => {
                        std::fs::rename(src.as_str(), dst.as_str())
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.move: {e}"))
                    }
                    _ => Err("io.move expects (String, String)".to_string()),
                },
            }),
        ),
        (
            "io.movePath",
            Value::Builtin(BuiltinFn {
                name: "io.movePath",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::Path(src), Value::Path(dst)) => {
                        std::fs::rename(src.as_path(), dst.as_path())
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.movePath: {e}"))
                    }
                    _ => Err("io.movePath expects (Path, Path)".to_string()),
                },
            }),
        ),
        // Process execution / 进程执行
    ]
}

/// Format any Value for human-readable output (used by print/println).
fn format_value_for_output(value: &Value) -> String {
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
        EventKind::Mapped { .. } | EventKind::Filtered { .. } => {
            Err("poll: chained events not yet supported".to_string())
        }
    }
}

/// Compute SHA-256 hash and return as hex string.
/// 计算 SHA-256 哈希并返回十六进制字符串。
fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(data);
    format!("{:x}", digest)
}

/// Atomic write: write content to a temporary file then rename to target path.
/// This avoids partial writes and leaves no temp residue on success.
/// 原子写入：先写入临时文件，然后重命名为目标路径。
/// 避免部分写入，成功时不留下临时文件残留。
fn atomic_write(path: &str, content: &[u8]) -> Result<(), String> {
    let target = std::path::Path::new(path);
    atomic_write_path(target, content)
}

fn atomic_write_path(target: &std::path::Path, content: &[u8]) -> Result<(), String> {
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
fn atomic_write_all(entries: &[Value]) -> Result<(), String> {
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

fn list_to_string_vec(items: &[Value], arg_name: &str) -> Result<Vec<String>, String> {
    items
        .iter()
        .enumerate()
        .map(|(idx, v)| match v {
            Value::String(s) => Ok(s.to_string()),
            _ => Err(format!("{arg_name}[{idx}] must be String")),
        })
        .collect()
}

fn list_to_command_vec(items: &[Value], arg_name: &str) -> Result<Vec<Rc<CommandValue>>, String> {
    items
        .iter()
        .enumerate()
        .map(|(idx, v)| match v {
            Value::Command(command) => Ok(Rc::clone(command)),
            _ => Err(format!("{arg_name}[{idx}] must be Command")),
        })
        .collect()
}

fn list_to_redirect_vec(items: &[Value], arg_name: &str) -> Result<Vec<RedirectValue>, String> {
    items
        .iter()
        .enumerate()
        .map(|(idx, v)| match v {
            Value::Redirect(redirect) => Ok((**redirect).clone()),
            _ => Err(format!("{arg_name}[{idx}] must be Redirect")),
        })
        .collect()
}

fn list_to_task_vec(items: &[Value], arg_name: &str) -> Result<Vec<Rc<TaskValue>>, String> {
    items
        .iter()
        .enumerate()
        .map(|(idx, v)| match v {
            Value::Task(task) => Ok(Rc::clone(task)),
            _ => Err(format!("{arg_name}[{idx}] must be Task[ProcessResult]")),
        })
        .collect()
}

fn output_to_process_result_value(output: std::process::Output) -> ProcessResultValue {
    let code = output.status.code().unwrap_or(-1);
    ProcessResultValue::new(
        code,
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn execute_command_lines(command: &CommandValue) -> Result<Value, String> {
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

fn execute_command_value(command: &CommandValue) -> Result<Value, String> {
    let result = execute_command_value_to_process_result(command, "io.execCommand")?;
    Ok(Value::ProcessResult(Rc::new(result)))
}

fn execute_pipeline_value(pipeline: &PipelineValue) -> Result<Value, String> {
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
                use std::io::Write;
                if let Err(e) = pipe.write_all(stdin_text.as_bytes()) {
                    let _ = tx.send(Err(format!("io.awaitTaskWithTimeout: {e}")));
                    return;
                }
            }

            match child.wait_with_output() {
                Ok(output) => {
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
                #[cfg(unix)]
                {
                    let _ = std::process::Command::new("kill")
                        .arg("-9")
                        .arg(pid.to_string())
                        .output();
                }
            }
            Ok(Value::None)
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("io.awaitTaskWithTimeout: internal error".to_string())
        }
    }
}

fn await_task_with_timeout(task: &TaskValue, timeout_ms: u64) -> Result<Value, String> {
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
        use std::io::Write;
        pipe.write_all(stdin_text.as_bytes())
            .map_err(|e| format!("io.awaitTaskWithTimeout: failed writing stdin: {e}"))?;
    }

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let output = child.wait_with_output();
        let result = match output {
            Ok(out) => Ok(RawProcessResult {
                code: out.status.code().unwrap_or(-1),
                success: out.status.success(),
                stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            }),
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
            #[cfg(unix)]
            {
                let _ = std::process::Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .output();
            }
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .output();
            }
            Ok(Value::None)
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("io.awaitTaskWithTimeout: internal error".to_string())
        }
    }
}

fn await_task(task: &TaskValue) -> Result<Value, String> {
    let result = await_task_to_process_result(task, "io.awaitTask")?;
    Ok(Value::ProcessResult(Rc::new(result)))
}

fn await_tasks(tasks: &[Rc<TaskValue>]) -> Result<Value, String> {
    let mut results = Vec::with_capacity(tasks.len());
    for (idx, task) in tasks.iter().enumerate() {
        let result = await_task_to_process_result(task, &format!("io.awaitTasks[{idx}]"))?;
        results.push(Value::ProcessResult(Rc::new(result)));
    }
    Ok(Value::List(Rc::new(results)))
}

fn await_task_to_process_result(
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

fn execute_pipeline_value_to_process_result(
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

fn execute_pipeline_value_to_process_result_with_input(
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

fn execute_pipeline_value_with_redirects_to_process_result(
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

fn command_value_from_options(
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

fn command_value_with_redirects(
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

fn pipeline_value_with_redirects(
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

fn pipeline_value_from_commands(
    commands: Vec<Rc<CommandValue>>,
    fn_name: &str,
) -> Result<PipelineValue, String> {
    validate_pipeline_command_topology(&commands, fn_name)?;
    Ok(PipelineValue::new(commands))
}

fn validate_pipeline_command_topology(
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

fn execute_command_value_to_process_result(
    command: &CommandValue,
    fn_name: &str,
) -> Result<ProcessResultValue, String> {
    execute_command_value_to_process_result_with_input(command, command.stdin(), fn_name)
}

fn execute_command_value_to_process_result_with_input(
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
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("{fn_name}: {e}"))?;
        if let Some(mut pipe) = child.stdin.take() {
            pipe.write_all(stdin_text.as_bytes())
                .map_err(|e| format!("{fn_name}: failed writing stdin: {e}"))?;
        }

        child
            .wait_with_output()
            .map(output_to_process_result_value)
            .map_err(|e| format!("{fn_name}: {e}"))
    } else {
        cmd.output()
            .map(output_to_process_result_value)
            .map_err(|e| format!("{fn_name}: {e}"))
    }
}

fn execute_command_value_with_redirects_to_process_result_with_input(
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

        child
            .wait_with_output()
            .map(output_to_process_result_value)
            .map_err(|e| format!("{fn_name}: {e}"))
    } else {
        cmd.stdin(std::process::Stdio::null());
        cmd.spawn()
            .map_err(|e| format!("{fn_name}: {e}"))?
            .wait_with_output()
            .map(output_to_process_result_value)
            .map_err(|e| format!("{fn_name}: {e}"))
    }
}

fn configured_process_command(command: &CommandValue) -> std::process::Command {
    let mut cmd = std::process::Command::new(command.program());
    cmd.args(command.args());

    if let Some(cwd) = command.cwd() {
        cmd.current_dir(cwd);
    }
    for (k, v) in command.env() {
        cmd.env(k, v);
    }

    cmd
}

fn resolve_redirect_path(command: &CommandValue, redirect: &RedirectValue) -> std::path::PathBuf {
    let path = redirect.path();
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

fn record_string_required(
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

fn record_string_optional(
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

fn record_string_list_optional(
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

fn record_env_optional(
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
