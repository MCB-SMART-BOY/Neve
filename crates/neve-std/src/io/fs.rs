//! File system operations — new additions live here.
//! Legacy functions remain in io/mod.rs.

use neve_eval::value::{
    BuiltinFn, CommandValue, RedirectValue, TaskValue, Value,
};
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;
use std::time::Duration;

/// New builtins for the fs submodule.
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // File reading / 文件读取
        (
            "io.lines",
            Value::Builtin(BuiltinFn {
                name: "io.lines",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => {
                        let content = std::fs::read_to_string(path.as_str())
                            .map_err(|e| format!("io.lines: {e}"))?;
                        let lines: Vec<Value> = content
                            .lines()
                            .map(|l| Value::String(Rc::new(l.to_string())))
                            .collect();
                        Ok(Value::List(Rc::new(lines)))
                    }
                    Value::Path(path) => {
                        let content = std::fs::read_to_string(path.as_path())
                            .map_err(|e| format!("io.lines: {e}"))?;
                        let lines: Vec<Value> = content
                            .lines()
                            .map(|l| Value::String(Rc::new(l.to_string())))
                            .collect();
                        Ok(Value::List(Rc::new(lines)))
                    }
                    _ => Err("io.lines expects a String or Path".to_string()),
                },
            }),
        ),
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
                    let guard = super::SCRIPT_ARGS
                        .read()
                        .map_err(|e| format!("io.args: {e}"))?;
                    let raw_args = guard.clone();
                    let mut positional: Vec<Value> = Vec::new();
                    let mut flags: HashMap<String, Value> = HashMap::new();
                    let mut i = 0;
                    while i < raw_args.len() {
                        let arg = &raw_args[i];
                        if arg == "--" {
                            // Rest are positional
                            for a in &raw_args[i+1..] {
                                positional.push(Value::String(Rc::new(a.clone())));
                            }
                            break;
                        }
                        if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") {
                            let flag = arg[1..].to_string();
                            // Check if next arg is a value (doesn't start with -)
                            if i + 1 < raw_args.len() && !raw_args[i+1].starts_with('-') {
                                i += 1;
                                let val = &raw_args[i];
                                // Try Int, then String
                                if let Ok(n) = val.parse::<i64>() {
                                    flags.insert(flag, Value::Int(n.into()));
                                } else {
                                    flags.insert(flag, Value::String(Rc::new(val.clone())));
                                }
                            } else {
                                flags.insert(flag, Value::Bool(true));
                            }
                        } else {
                            positional.push(Value::String(Rc::new(arg.clone())));
                        }
                        i += 1;
                    }
                    // Build result: Tuple(positionals..., flags Record)
                    let mut elements = positional;
                    let flags_map: HashMap<String, Value> = flags;
                    elements.push(Value::Record(Rc::new(flags_map)));
                    Ok(Value::Tuple(Rc::new(elements)))
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
                        let argv = super::list_to_string_vec(argv, "io.command args")?;
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
                        let command = super::command_value_from_options(options, "io.commandWith")?;
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
                        let redirects = super::list_to_redirect_vec(
                            redirects,
                            "io.commandWithRedirects redirects",
                        )?;
                        let command = super::command_value_with_redirects(
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
                    Value::Command(command) => super::execute_command_value(command),
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
                    Value::Command(command) => super::execute_command_lines(command),
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
                        let commands =
                            super::list_to_command_vec(commands, "io.pipeline commands")?;
                        let pipeline =
                            super::pipeline_value_from_commands(commands, "io.pipeline")?;
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
                        let redirects = super::list_to_redirect_vec(
                            redirects,
                            "io.pipelineWithRedirects redirects",
                        )?;
                        let pipeline = super::pipeline_value_with_redirects(
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
                    Value::Pipeline(pipeline) => super::execute_pipeline_value(pipeline),
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
                    Value::Task(task) => super::await_task(task),
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
                        let tasks = super::list_to_task_vec(tasks, "io.awaitTasks tasks")?;
                        super::await_tasks(&tasks)
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
                        super::await_task_with_timeout(task, timeout_ms)
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
                        let hash = super::sha256_hex(&content);
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
                        let hash = super::sha256_hex(&content);
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
                        let hash = super::sha256_hex(s.as_bytes());
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
            "io.execCommandStreamingWithTimeout",
            Value::Builtin(BuiltinFn {
                name: "io.execCommandStreamingWithTimeout",
                arity: 3,
                func: |_args| {
                    Err("io.execCommandStreamingWithTimeout is evaluator-owned".to_string())
                },
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
            "io.execPipelineStreamingWithTimeout",
            Value::Builtin(BuiltinFn {
                name: "io.execPipelineStreamingWithTimeout",
                arity: 3,
                func: |_args| {
                    Err("io.execPipelineStreamingWithTimeout is evaluator-owned".to_string())
                },
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
                        super::atomic_write(path.as_str(), content.as_bytes())
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
                        super::atomic_write_path(path.as_path(), content.as_bytes())
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
                    Value::List(entries) => super::atomic_write_all(entries)
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
    ]
}
