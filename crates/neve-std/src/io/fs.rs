//! File system operations — new additions live here.
//! Legacy functions remain in io/mod.rs.

use neve_eval::value::{
    BuiltinFn, CommandValue, RedirectValue, StreamTransform, StreamValue, TaskValue, Value,
};
use std::collections::HashMap;
use std::io::{BufRead, Write};
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
            "io.setEnv",
            Value::Builtin(BuiltinFn {
                name: "io.setEnv",
                arity: 2,
                func: |args| {
                    let key = match &args[0] {
                        Value::String(s) => s.as_str(),
                        _ => return Err("io.setEnv expects a String key".to_string()),
                    };
                    let val = match &args[1] {
                        Value::String(s) => s.as_str(),
                        _ => return Err("io.setEnv expects a String value".to_string()),
                    };
                    unsafe {
                        std::env::set_var(key, val);
                    }
                    Ok(Value::Unit)
                },
            }),
        ),
        (
            "io.unsetEnv",
            Value::Builtin(BuiltinFn {
                name: "io.unsetEnv",
                arity: 1,
                func: |args| {
                    let key = match &args[0] {
                        Value::String(s) => s.as_str(),
                        _ => return Err("io.unsetEnv expects a String key".to_string()),
                    };
                    unsafe {
                        std::env::remove_var(key);
                    }
                    Ok(Value::Unit)
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
                        // -- stops flag parsing
                        if arg == "--" {
                            for a in &raw_args[i + 1..] {
                                positional.push(Value::String(Rc::new(a.clone())));
                            }
                            break;
                        }
                        // Flag: starts with -, not just "-", not a negative number
                        if arg.starts_with('-') && arg.len() > 1 && arg != "-" {
                            // Negative number check: "-" followed by digits only → positional
                            let rest = &arg[1..];
                            if rest.chars().all(|c| c.is_ascii_digit()) {
                                positional.push(Value::String(Rc::new(arg.clone())));
                                i += 1;
                                continue;
                            }
                            // Compact form: -j8 → flag "j", value 8
                            if rest.len() > 1
                                && rest.chars().nth(1).is_some_and(|c| c.is_ascii_digit())
                            {
                                let flag = rest[..1].to_string();
                                let val = &rest[1..];
                                if let Ok(n) = val.parse::<i64>() {
                                    flags.insert(flag, Value::Int(n.into()));
                                } else {
                                    flags.insert(flag, Value::String(Rc::new(val.to_string())));
                                }
                                i += 1;
                                continue;
                            }
                            let flag = rest.to_string();
                            // Check if next arg is a value (doesn't start with -)
                            if i + 1 < raw_args.len() && !raw_args[i + 1].starts_with('-') {
                                i += 1;
                                let val = &raw_args[i];
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
                    // Build result: (List<String>, Record{flags})
                    let pos_list = Value::List(Rc::new(positional));
                    let flags_map: HashMap<String, Value> = flags;
                    Ok(Value::Tuple(Rc::new(vec![
                        pos_list,
                        Value::Record(Rc::new(flags_map)),
                    ])))
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
            "io.awaitAny",
            Value::Builtin(BuiltinFn {
                name: "io.awaitAny",
                arity: 1,
                func: |args| match &args[0] {
                    Value::List(tasks) => {
                        let tasks = super::list_to_task_vec(tasks, "io.awaitAny tasks")?;
                        super::await_any(&tasks)
                    }
                    _ => Err("io.awaitAny expects List<Task[ProcessResult]>".to_string()),
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
        // Unix file metadata / Unix 文件元数据
        (
            "io.walk",
            Value::Builtin(BuiltinFn {
                name: "io.walk",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(root) => {
                        let mut entries: Vec<Value> = Vec::new();
                        fn walk_dir(
                            dir: &std::path::Path,
                            entries: &mut Vec<Value>,
                        ) -> Result<(), String> {
                            for entry in
                                std::fs::read_dir(dir).map_err(|e| format!("io.walk: {e}"))?
                            {
                                let entry = entry.map_err(|e| format!("io.walk: {e}"))?;
                                let path = entry.path();
                                entries.push(Value::Path(Rc::new(path.clone())));
                                if path.is_dir() {
                                    walk_dir(&path, entries)?;
                                }
                            }
                            Ok(())
                        }
                        walk_dir(root.as_path(), &mut entries)?;
                        Ok(Value::List(Rc::new(entries)))
                    }
                    Value::String(s) => {
                        let root = std::path::PathBuf::from(s.as_str());
                        let mut entries: Vec<Value> = Vec::new();
                        fn walk_dir_str(
                            dir: &std::path::Path,
                            entries: &mut Vec<Value>,
                        ) -> Result<(), String> {
                            for entry in
                                std::fs::read_dir(dir).map_err(|e| format!("io.walk: {e}"))?
                            {
                                let entry = entry.map_err(|e| format!("io.walk: {e}"))?;
                                let path = entry.path();
                                entries.push(Value::Path(Rc::new(path.clone())));
                                if path.is_dir() {
                                    walk_dir_str(&path, entries)?;
                                }
                            }
                            Ok(())
                        }
                        walk_dir_str(&root, &mut entries)?;
                        Ok(Value::List(Rc::new(entries)))
                    }
                    _ => Err("io.walk expects a Path or String".to_string()),
                },
            }),
        ),
        (
            "io.chmod",
            Value::Builtin(BuiltinFn {
                name: "io.chmod",
                arity: 2,
                func: |args| {
                    let path = match &args[0] {
                        Value::Path(p) => p.as_path().to_path_buf(),
                        Value::String(s) => std::path::PathBuf::from(s.as_str()),
                        _ => return Err("io.chmod expects (Path|String, Int)".to_string()),
                    };
                    let mode: u32 = match &args[1] {
                        Value::Int(n) => n
                            .clone()
                            .try_into()
                            .map_err(|_| "io.chmod: mode must be non-negative".to_string())?,
                        _ => return Err("io.chmod expects (Path|String, Int)".to_string()),
                    };
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.chmod: {e}"))
                    }
                    #[cfg(not(unix))]
                    {
                        let _ = (path, mode);
                        Err("io.chmod is only supported on Unix".to_string())
                    }
                },
            }),
        ),
        (
            "io.chown",
            Value::Builtin(BuiltinFn {
                name: "io.chown",
                arity: 3,
                func: |args| {
                    let path = match &args[0] {
                        Value::Path(p) => p.as_path().to_path_buf(),
                        Value::String(s) => std::path::PathBuf::from(s.as_str()),
                        _ => return Err("io.chown expects (Path|String, Int, Int)".to_string()),
                    };
                    let uid: u32 = match &args[1] {
                        Value::Int(n) => n
                            .clone()
                            .try_into()
                            .map_err(|_| "io.chown: uid must be non-negative".to_string())?,
                        _ => return Err("io.chown expects (Path|String, Int, Int)".to_string()),
                    };
                    let gid: u32 = match &args[2] {
                        Value::Int(n) => n
                            .clone()
                            .try_into()
                            .map_err(|_| "io.chown: gid must be non-negative".to_string())?,
                        _ => return Err("io.chown expects (Path|String, Int, Int)".to_string()),
                    };
                    #[cfg(unix)]
                    {
                        let output = std::process::Command::new("chown")
                            .args([
                                format!("{}:{}", uid, gid),
                                path.to_string_lossy().to_string(),
                            ])
                            .output()
                            .map_err(|e| format!("io.chown: {e}"))?;
                        if output.status.success() {
                            Ok(Value::Unit)
                        } else {
                            Err(format!(
                                "io.chown: {}",
                                String::from_utf8_lossy(&output.stderr)
                            ))
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        let _ = (path, uid, gid);
                        Err("io.chown is only supported on Unix".to_string())
                    }
                },
            }),
        ),
        (
            "io.symlink",
            Value::Builtin(BuiltinFn {
                name: "io.symlink",
                arity: 2,
                func: |args| {
                    let (target, link) = match (&args[0], &args[1]) {
                        (Value::Path(t), Value::Path(l)) => {
                            (t.as_path().to_path_buf(), l.as_path().to_path_buf())
                        }
                        (Value::String(t), Value::String(l)) => (
                            std::path::PathBuf::from(t.as_str()),
                            std::path::PathBuf::from(l.as_str()),
                        ),
                        (Value::Path(t), Value::String(l)) => (
                            t.as_path().to_path_buf(),
                            std::path::PathBuf::from(l.as_str()),
                        ),
                        (Value::String(t), Value::Path(l)) => (
                            std::path::PathBuf::from(t.as_str()),
                            l.as_path().to_path_buf(),
                        ),
                        _ => {
                            return Err("io.symlink expects (Path|String, Path|String)".to_string());
                        }
                    };
                    #[cfg(unix)]
                    {
                        std::os::unix::fs::symlink(&target, &link)
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.symlink: {e}"))
                    }
                    #[cfg(windows)]
                    {
                        if target.is_dir() {
                            std::os::windows::fs::symlink_dir(&target, &link)
                        } else {
                            std::os::windows::fs::symlink_file(&target, &link)
                        }
                        .map(|_| Value::Unit)
                        .map_err(|e| format!("io.symlink: {e}"))
                    }
                },
            }),
        ),
        (
            "io.readlink",
            Value::Builtin(BuiltinFn {
                name: "io.readlink",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Path(p) => {
                        let target = std::fs::read_link(p.as_path())
                            .map_err(|e| format!("io.readlink: {e}"))?;
                        Ok(Value::Path(Rc::new(target)))
                    }
                    Value::String(s) => {
                        let target = std::fs::read_link(s.as_str())
                            .map_err(|e| format!("io.readlink: {e}"))?;
                        Ok(Value::Path(Rc::new(target)))
                    }
                    _ => Err("io.readlink expects a Path or String".to_string()),
                },
            }),
        ),
        #[cfg(unix)]
        (
            "io.tempDir",
            Value::Builtin(BuiltinFn {
                name: "io.tempDir",
                arity: 1,
                func: |args| {
                    let dir = tempfile::tempdir().map_err(|e| format!("io.tempDir: {e}"))?;
                    let dir_path = dir.keep();
                    let path_value = Value::Path(Rc::new(dir_path.clone()));
                    match &args[0] {
                        Value::BuiltinFn(_, _) | Value::Builtin(_) | Value::Closure { .. } => {
                            let result = match &args[0] {
                                Value::BuiltinFn(_name, func) => func(vec![path_value.clone()])
                                    .map_err(|e| format!("io.tempDir callback: {e}")),
                                _ => Err("io.tempDir: callback must be a function".to_string()),
                            };
                            let _ = std::fs::remove_dir_all(&dir_path);
                            result.map(|v| Value::Some(Box::new(v)))
                        }
                        _ => {
                            let _ = std::fs::remove_dir_all(&dir_path);
                            Err("io.tempDir expects a function callback".to_string())
                        }
                    }
                },
            }),
        ),
        // === Stream construction (pure, lazy) ===
        (
            "io.streamLines",
            Value::Builtin(BuiltinFn {
                name: "io.streamLines",
                arity: 1,
                func: |args| {
                    let path = match &args[0] {
                        Value::Path(p) => p.clone(),
                        Value::String(s) => Rc::new(std::path::PathBuf::from(s.as_str())),
                        _ => return Err("io.streamLines expects a Path or String".to_string()),
                    };
                    let file_path = path.as_path().to_path_buf();
                    let (tx, rx) = std::sync::mpsc::sync_channel::<Result<String, String>>(16);
                    std::thread::spawn(move || {
                        let file = match std::fs::File::open(&file_path) {
                            Ok(f) => f,
                            Err(e) => {
                                let _ = tx.send(Err(format!("io.streamLines: {e}")));
                                return;
                            }
                        };
                        let reader = std::io::BufReader::new(file);
                        for line in reader.lines() {
                            match line {
                                Ok(l) => {
                                    if tx.send(Ok(l)).is_err() {
                                        break; // consumer dropped
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(format!("io.streamLines: {e}")));
                                    return;
                                }
                            }
                        }
                    });
                    Ok(Value::Stream(Rc::new(StreamValue::from_lines_channel(rx))))
                },
            }),
        ),
        (
            "io.streamCommand",
            Value::Builtin(BuiltinFn {
                name: "io.streamCommand",
                arity: 1,
                func: |args| {
                    let command = match &args[0] {
                        Value::Command(cmd) => cmd.clone(),
                        _ => return Err("io.streamCommand expects a Command".to_string()),
                    };
                    let program = command.program().to_string();
                    let args_list = command.args().to_vec();
                    let cwd = command.cwd().map(|s| s.to_string());
                    let env = command.env().clone();
                    let stdin_data = command.stdin().map(|s| s.to_string());
                    let (tx, rx) = std::sync::mpsc::sync_channel::<Result<String, String>>(16);
                    let max_lines = 100_000usize;
                    std::thread::spawn(move || {
                        let mut c = std::process::Command::new(&program);
                        c.args(&args_list);
                        if let Some(ref wd) = cwd {
                            c.current_dir(wd);
                        }
                        for (k, v) in &env {
                            c.env(k, v);
                        }
                        if stdin_data.is_some() {
                            c.stdin(std::process::Stdio::piped());
                        } else {
                            c.stdin(std::process::Stdio::null());
                        }
                        c.stdout(std::process::Stdio::piped());
                        c.stderr(std::process::Stdio::piped());
                        let mut child = match c.spawn() {
                            Ok(child) => child,
                            Err(e) => {
                                let _ = tx.send(Err(format!("io.streamCommand: {e}")));
                                return;
                            }
                        };
                        if let Some(ref data) = stdin_data {
                            use std::io::Write;
                            let write_err = child
                                .stdin
                                .take()
                                .is_some_and(|mut pipe| pipe.write_all(data.as_bytes()).is_err());
                            if write_err {
                                let _ = tx
                                    .send(Err("io.streamCommand: stdin write failed".to_string()));
                                return;
                            }
                        }
                        let stdout = match child.stdout.take() {
                            Some(s) => s,
                            None => {
                                let _ = tx.send(Err("io.streamCommand: no stdout".to_string()));
                                return;
                            }
                        };
                        let reader = std::io::BufReader::new(stdout);
                        let mut line_count = 0usize;
                        for line in reader.lines() {
                            line_count += 1;
                            if line_count > max_lines {
                                let _ = child.kill();
                                let _ = tx.send(Err(format!(
                                    "io.streamCommand: exceeded max stream lines ({max_lines})"
                                )));
                                return;
                            }
                            match line {
                                Ok(l) => {
                                    if tx.send(Ok(l)).is_err() {
                                        break; // consumer dropped
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(format!("io.streamCommand: {e}")));
                                    return;
                                }
                            }
                        }
                        let _ = child.wait();
                    });
                    Ok(Value::Stream(Rc::new(StreamValue::from_lines_channel(rx))))
                },
            }),
        ),
        (
            "io.streamBytes",
            Value::Builtin(BuiltinFn {
                name: "io.streamBytes",
                arity: 1,
                func: |args| {
                    let path = match &args[0] {
                        Value::Path(p) => p.clone(),
                        Value::String(s) => Rc::new(std::path::PathBuf::from(s.as_str())),
                        _ => return Err("io.streamBytes expects a Path or String".to_string()),
                    };
                    let file_path = path.as_path().to_path_buf();
                    let (tx, rx) = std::sync::mpsc::sync_channel::<Result<Vec<u8>, String>>(16);
                    std::thread::spawn(move || {
                        let mut file = match std::fs::File::open(&file_path) {
                            Ok(f) => f,
                            Err(e) => {
                                let _ = tx.send(Err(format!("io.streamBytes: {e}")));
                                return;
                            }
                        };
                        let mut buf = [0u8; 8192];
                        loop {
                            match std::io::Read::read(&mut file, &mut buf) {
                                Ok(0) => break, // EOF
                                Ok(n) => {
                                    if tx.send(Ok(buf[..n].to_vec())).is_err() {
                                        break; // consumer dropped
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(format!("io.streamBytes: {e}")));
                                    return;
                                }
                            }
                        }
                    });
                    Ok(Value::Stream(Rc::new(StreamValue::from_bytes_channel(rx))))
                },
            }),
        ),
        // === Stream transforms (pure, lazy, wrapped) ===
        (
            "io.streamTake",
            Value::Builtin(BuiltinFn {
                name: "io.streamTake",
                arity: 2,
                func: |args| {
                    let source = match &args[0] {
                        Value::Stream(s) => s.clone(),
                        _ => return Err("io.streamTake expects a Stream".to_string()),
                    };
                    let n: usize = match &args[1] {
                        Value::Int(n) => n.clone().try_into().map_err(|_| {
                            "io.streamTake: n must be non-negative and fit in usize".to_string()
                        })?,
                        _ => return Err("io.streamTake expects an Int".to_string()),
                    };
                    Ok(Value::Stream(Rc::new(StreamValue::from_wrapped(
                        source,
                        StreamTransform::Take { remaining: n },
                    ))))
                },
            }),
        ),
        (
            "io.streamDrop",
            Value::Builtin(BuiltinFn {
                name: "io.streamDrop",
                arity: 2,
                func: |args| {
                    let source = match &args[0] {
                        Value::Stream(s) => s.clone(),
                        _ => return Err("io.streamDrop expects a Stream".to_string()),
                    };
                    let n: usize = match &args[1] {
                        Value::Int(n) => n.clone().try_into().map_err(|_| {
                            "io.streamDrop: n must be non-negative and fit in usize".to_string()
                        })?,
                        _ => return Err("io.streamDrop expects an Int".to_string()),
                    };
                    Ok(Value::Stream(Rc::new(StreamValue::from_wrapped(
                        source,
                        StreamTransform::Drop { remaining: n },
                    ))))
                },
            }),
        ),
        (
            "io.streamMap",
            Value::Builtin(BuiltinFn {
                name: "io.streamMap",
                arity: 2,
                func: |args| {
                    let source = match &args[0] {
                        Value::Stream(s) => s.clone(),
                        _ => return Err("io.streamMap expects a Stream".to_string()),
                    };
                    let func = args[1].clone();
                    Ok(Value::Stream(Rc::new(StreamValue::from_wrapped(
                        source,
                        StreamTransform::Map { func },
                    ))))
                },
            }),
        ),
        (
            "io.streamFilter",
            Value::Builtin(BuiltinFn {
                name: "io.streamFilter",
                arity: 2,
                func: |args| {
                    let source = match &args[0] {
                        Value::Stream(s) => s.clone(),
                        _ => return Err("io.streamFilter expects a Stream".to_string()),
                    };
                    let predicate = args[1].clone();
                    Ok(Value::Stream(Rc::new(StreamValue::from_wrapped(
                        source,
                        StreamTransform::Filter { predicate },
                    ))))
                },
            }),
        ),
        // === Stream consumers (effectful) ===
        (
            "io.streamPipe",
            Value::Builtin(BuiltinFn {
                name: "io.streamPipe",
                arity: 2,
                func: |args| {
                    let source = match &args[0] {
                        Value::Stream(s) => s.clone(),
                        _ => return Err("io.streamPipe expects a Stream".to_string()),
                    };
                    let command = match &args[1] {
                        Value::Command(cmd) => cmd.clone(),
                        _ => return Err("io.streamPipe expects a Command".to_string()),
                    };
                    // Collect all lines from the stream
                    let mut lines: Vec<String> = Vec::new();
                    loop {
                        match source.next() {
                            Ok(Some(v)) => {
                                lines.push(super::format_value_for_output(&v));
                            }
                            Ok(None) => break,
                            Err(e) => return Err(format!("io.streamPipe: {e}")),
                        }
                    }
                    let stdin_text = lines.join("\n");
                    let result = super::execute_command_value_to_process_result_with_input(
                        &command,
                        Some(&stdin_text),
                        "io.streamPipe",
                    )?;
                    Ok(Value::ProcessResult(Rc::new(result)))
                },
            }),
        ),
        (
            "io.streamForEach",
            Value::Builtin(BuiltinFn {
                name: "io.streamForEach",
                arity: 2,
                func: |_args| Err("io.streamForEach is evaluator-owned".to_string()),
            }),
        ),
        (
            "io.streamFold",
            Value::Builtin(BuiltinFn {
                name: "io.streamFold",
                arity: 3,
                func: |_args| Err("io.streamFold is evaluator-owned".to_string()),
            }),
        ),
        (
            "io.streamWithTimeout",
            Value::Builtin(BuiltinFn {
                name: "io.streamWithTimeout",
                arity: 2,
                func: |args| {
                    let source = match &args[0] {
                        Value::Stream(s) => s.clone(),
                        _ => return Err("io.streamWithTimeout expects a Stream".to_string()),
                    };
                    let timeout_ms: u64 = match &args[1] {
                        Value::Int(n) => n.clone().try_into().map_err(|_| {
                            "io.streamWithTimeout: timeout must be non-negative".to_string()
                        })?,
                        _ => return Err("io.streamWithTimeout expects an Int".to_string()),
                    };
                    // Uses Timeout transform: deadline checked on each next() call.
                    // Note: for channel-based streams, a blocking recv() in next()
                    // is NOT interrupted by the timeout. The deadline is checked
                    // between successful element retrievals only.
                    let deadline =
                        std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
                    Ok(Value::Stream(Rc::new(StreamValue::from_wrapped(
                        source,
                        StreamTransform::Timeout { deadline },
                    ))))
                },
            }),
        ),
    ]
}
