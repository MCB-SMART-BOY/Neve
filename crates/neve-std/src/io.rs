//! IO operations for the standard library.
//! 标准库的 IO 操作。
//!
//! These are impure operations that interact with the file system.
//! They are primarily used during package builds and configuration generation.
//! 这些是与文件系统交互的非纯操作。
//! 主要用于包构建和配置生成期间。

use neve_eval::value::{BuiltinFn, Value};
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
            "io.appendFile",
            Value::Builtin(BuiltinFn {
                name: "io.appendFile",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => {
                        std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path.as_str())
                            .and_then(|mut f| f.write_all(content.as_bytes()))
                            .map(|_| Value::Unit)
                            .map_err(|e| format!("io.appendFile: {e}"))
                    }
                    _ => Err("io.appendFile expects (String, String)".to_string()),
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
        (
            "io.exec",
            Value::Builtin(BuiltinFn {
                name: "io.exec",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(program), Value::List(argv)) => {
                        let argv = list_to_string_vec(argv, "io.exec args")?;
                        execute_command(program, &argv)
                    }
                    _ => Err("io.exec expects (String, List<String>)".to_string()),
                },
            }),
        ),
        (
            "io.execShell",
            Value::Builtin(BuiltinFn {
                name: "io.execShell",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(command) => execute_shell_command(command),
                    _ => Err("io.execShell expects a string command".to_string()),
                },
            }),
        ),
        (
            "io.execWith",
            Value::Builtin(BuiltinFn {
                name: "io.execWith",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Record(options) => execute_with_options(options),
                    _ => Err("io.execWith expects a record options object".to_string()),
                },
            }),
        ),
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

fn output_to_record(output: std::process::Output) -> Value {
    let mut fields = HashMap::with_capacity(4);
    let code = output.status.code().unwrap_or(-1);
    fields.insert("code".to_string(), Value::Int(code.into()));
    fields.insert("success".to_string(), Value::Bool(output.status.success()));
    fields.insert(
        "stdout".to_string(),
        Value::String(Rc::new(String::from_utf8_lossy(&output.stdout).to_string())),
    );
    fields.insert(
        "stderr".to_string(),
        Value::String(Rc::new(String::from_utf8_lossy(&output.stderr).to_string())),
    );
    Value::Record(Rc::new(fields))
}

fn execute_command(program: &str, argv: &[String]) -> Result<Value, String> {
    std::process::Command::new(program)
        .args(argv)
        .output()
        .map(output_to_record)
        .map_err(|e| format!("io.exec: {e}"))
}

fn execute_with_options(options: &HashMap<String, Value>) -> Result<Value, String> {
    let program = record_string_required(options, "program", "io.execWith")?;
    let args = record_string_list_optional(options, "args", "io.execWith")?.unwrap_or_default();
    let cwd = record_string_optional(options, "cwd", "io.execWith")?;
    let stdin = record_string_optional(options, "stdin", "io.execWith")?;
    let env = record_env_optional(options, "env", "io.execWith")?;

    let mut cmd = std::process::Command::new(program.as_str());
    cmd.args(&args);

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }

    if let Some(stdin_text) = stdin {
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("io.execWith: {e}"))?;
        if let Some(mut pipe) = child.stdin.take() {
            pipe.write_all(stdin_text.as_bytes())
                .map_err(|e| format!("io.execWith: failed writing stdin: {e}"))?;
        }

        child
            .wait_with_output()
            .map(output_to_record)
            .map_err(|e| format!("io.execWith: {e}"))
    } else {
        cmd.output()
            .map(output_to_record)
            .map_err(|e| format!("io.execWith: {e}"))
    }
}

#[cfg(not(windows))]
fn execute_shell_command(command: &str) -> Result<Value, String> {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map(output_to_record)
        .map_err(|e| format!("io.execShell: {e}"))
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
        Some(Value::List(items)) => list_to_string_vec(items, &format!("{fn_name}.{key}")).map(Some),
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

#[cfg(windows)]
fn execute_shell_command(command: &str) -> Result<Value, String> {
    std::process::Command::new("cmd")
        .arg("/C")
        .arg(command)
        .output()
        .map(output_to_record)
        .map_err(|e| format!("io.execShell: {e}"))
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
