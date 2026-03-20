//! Source fetching helpers for the standard library.
//! 标准库源码获取辅助函数。
//!
//! These helpers expose `neve-fetch` to Neve programs so package expressions
//! can materialize sources with optional hash verification.
//! 这些辅助函数把 `neve-fetch` 暴露给 Neve 程序，使包表达式可按需获取源码并可选校验哈希。

use neve_derive::Hash;
use neve_eval::value::{BuiltinFn, Value};
use neve_fetch::{FetchResult, Fetcher, Source};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

/// Returns all fetch builtins.
/// 返回所有 fetch 内置函数。
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        (
            "fetch.url",
            Value::Builtin(BuiltinFn {
                name: "fetch.url",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(url) => fetch_source(Source::url(url.as_str())),
                    _ => Err("fetch.url expects a URL string".to_string()),
                },
            }),
        ),
        (
            "fetch.urlWithHash",
            Value::Builtin(BuiltinFn {
                name: "fetch.urlWithHash",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(url), hash_value) => {
                        let hash = parse_hash_arg(hash_value, "fetch.urlWithHash")?;
                        fetch_source(Source::url_with_hash(url.as_str(), hash))
                    }
                    _ => Err("fetch.urlWithHash expects (URL string, hash string)".to_string()),
                },
            }),
        ),
        (
            "fetch.path",
            Value::Builtin(BuiltinFn {
                name: "fetch.path",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => fetch_source(Source::path(PathBuf::from(path.as_str()))),
                    _ => Err("fetch.path expects a file path string".to_string()),
                },
            }),
        ),
        (
            "fetch.pathWithHash",
            Value::Builtin(BuiltinFn {
                name: "fetch.pathWithHash",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(path), hash_value) => {
                        let hash = parse_hash_arg(hash_value, "fetch.pathWithHash")?;
                        fetch_source(Source::path(PathBuf::from(path.as_str())).with_hash(hash))
                    }
                    _ => Err("fetch.pathWithHash expects (path string, hash string)".to_string()),
                },
            }),
        ),
        (
            "fetch.git",
            Value::Builtin(BuiltinFn {
                name: "fetch.git",
                arity: 2,
                func: |args| match (&args[0], &args[1]) {
                    (Value::String(url), Value::String(rev)) => {
                        fetch_source(Source::git(url.as_str(), rev.as_str()))
                    }
                    _ => Err("fetch.git expects (repo URL string, revision string)".to_string()),
                },
            }),
        ),
        (
            "fetch.gitWithHash",
            Value::Builtin(BuiltinFn {
                name: "fetch.gitWithHash",
                arity: 3,
                func: |args| match (&args[0], &args[1], &args[2]) {
                    (Value::String(url), Value::String(rev), hash_value) => {
                        let hash = parse_hash_arg(hash_value, "fetch.gitWithHash")?;
                        fetch_source(Source::git(url.as_str(), rev.as_str()).with_hash(hash))
                    }
                    _ => Err(
                        "fetch.gitWithHash expects (repo URL string, revision string, hash string)"
                            .to_string(),
                    ),
                },
            }),
        ),
    ]
}

fn parse_hash_arg(value: &Value, builtin: &str) -> Result<Hash, String> {
    match value {
        Value::String(hash) => {
            Hash::from_hex(hash).map_err(|_| format!("{builtin} expects a 64-character hex hash"))
        }
        _ => Err(format!("{builtin} expects a hash string")),
    }
}

fn fetch_source(source: Source) -> Result<Value, String> {
    let cache_dir = default_fetch_cache_dir();
    let fetcher = Fetcher::new(cache_dir).map_err(|e| format!("fetch: {e}"))?;
    let result = fetcher.fetch(&source).map_err(|e| format!("fetch: {e}"))?;
    Ok(fetch_result_to_value(result))
}

fn fetch_result_to_value(result: FetchResult) -> Value {
    let mut fields = HashMap::new();
    fields.insert(
        "path".to_string(),
        Value::String(Rc::new(result.path.to_string_lossy().to_string())),
    );
    fields.insert(
        "hash".to_string(),
        Value::String(Rc::new(result.hash.to_hex())),
    );
    fields.insert("cached".to_string(), Value::Bool(result.cached));
    Value::Record(Rc::new(fields))
}

fn default_fetch_cache_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("NEVE_FETCH_CACHE") {
        return PathBuf::from(dir);
    }

    if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(xdg_cache).join("neve").join("fetch");
    }

    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".cache")
            .join("neve")
            .join("fetch");
    }

    std::env::temp_dir()
        .join("neve")
        .join("fetch")
        .join(std::process::id().to_string())
}
