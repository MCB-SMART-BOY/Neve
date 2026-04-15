//! IO operations for the standard library.
//! 标准库的 IO 操作。
//!
//! These are impure operations that interact with the file system.
//! They are primarily used during package builds and configuration generation.
//! 这些是与文件系统交互的非纯操作。
//! 主要用于包构建和配置生成期间。

use neve_eval::value::{
    BuiltinFn, CommandValue, PipelineValue, ProcessResultValue, RedirectValue, TaskTargetValue,
    TaskValue, Value,
};
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

/// Returns all IO builtins.
/// 返回所有 IO 内置函数。
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
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
        // Process execution / 进程执行
    ]
}

/// Compute SHA-256 hash and return as hex string.
/// 计算 SHA-256 哈希并返回十六进制字符串。
fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(data);
    format!("{:x}", digest)
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

fn execute_command_value(command: &CommandValue) -> Result<Value, String> {
    let result = execute_command_value_to_process_result(command, "io.execCommand")?;
    Ok(Value::ProcessResult(Rc::new(result)))
}

fn execute_pipeline_value(pipeline: &PipelineValue) -> Result<Value, String> {
    let result = execute_pipeline_value_to_process_result(pipeline, "io.execPipeline")?;
    Ok(Value::ProcessResult(Rc::new(result)))
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
