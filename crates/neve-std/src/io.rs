//! IO operations for the standard library.
//!
//! These are impure operations that interact with the file system.
//! They are primarily used during package builds and configuration generation.

use neve_eval::value::{Value, BuiltinFn};
use std::rc::Rc;

pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // File reading
        ("io.readFile", Value::Builtin(BuiltinFn {
            name: "io.readFile",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(path) => {
                        std::fs::read_to_string(path.as_str())
                            .map(|s| Value::String(Rc::new(s)))
                            .map_err(|e| format!("io.readFile: {e}"))
                    }
                    _ => Err("io.readFile expects a string path".to_string()),
                }
            },
        })),
        ("io.readDir", Value::Builtin(BuiltinFn {
            name: "io.readDir",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(path) => {
                        let entries: Result<Vec<_>, _> = std::fs::read_dir(path.as_str())
                            .map_err(|e| format!("io.readDir: {e}"))?
                            .map(|entry| {
                                entry
                                    .map(|e| Value::String(Rc::new(e.file_name().to_string_lossy().to_string())))
                                    .map_err(|e| format!("io.readDir: {e}"))
                            })
                            .collect();
                        entries.map(|v| Value::List(Rc::new(v)))
                    }
                    _ => Err("io.readDir expects a string path".to_string()),
                }
            },
        })),
        
        // File checks
        ("io.pathExists", Value::Builtin(BuiltinFn {
            name: "io.pathExists",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(path) => {
                        Ok(Value::Bool(std::path::Path::new(path.as_str()).exists()))
                    }
                    _ => Err("io.pathExists expects a string path".to_string()),
                }
            },
        })),
        ("io.isDir", Value::Builtin(BuiltinFn {
            name: "io.isDir",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(path) => {
                        Ok(Value::Bool(std::path::Path::new(path.as_str()).is_dir()))
                    }
                    _ => Err("io.isDir expects a string path".to_string()),
                }
            },
        })),
        ("io.isFile", Value::Builtin(BuiltinFn {
            name: "io.isFile",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(path) => {
                        Ok(Value::Bool(std::path::Path::new(path.as_str()).is_file()))
                    }
                    _ => Err("io.isFile expects a string path".to_string()),
                }
            },
        })),
        
        // Environment
        ("io.getEnv", Value::Builtin(BuiltinFn {
            name: "io.getEnv",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(name) => {
                        match std::env::var(name.as_str()) {
                            Ok(val) => Ok(Value::Some(Box::new(Value::String(Rc::new(val))))),
                            Err(_) => Ok(Value::None),
                        }
                    }
                    _ => Err("io.getEnv expects a string".to_string()),
                }
            },
        })),
        ("io.currentDir", Value::Builtin(BuiltinFn {
            name: "io.currentDir",
            arity: 0,
            func: |_args| {
                std::env::current_dir()
                    .map(|p| Value::String(Rc::new(p.to_string_lossy().to_string())))
                    .map_err(|e| format!("io.currentDir: {e}"))
            },
        })),
        ("io.homeDir", Value::Builtin(BuiltinFn {
            name: "io.homeDir",
            arity: 0,
            func: |_args| {
                Ok(std::env::var("HOME")
                    .map(|p| Value::Some(Box::new(Value::String(Rc::new(p)))))
                    .unwrap_or(Value::None))
            },
        })),
        
        // Hashing (useful for content-addressed store)
        ("io.hashFile", Value::Builtin(BuiltinFn {
            name: "io.hashFile",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(path) => {
                        let content = std::fs::read(path.as_str())
                            .map_err(|e| format!("io.hashFile: {e}"))?;
                        let hash = sha256_hex(&content);
                        Ok(Value::String(Rc::new(hash)))
                    }
                    _ => Err("io.hashFile expects a string path".to_string()),
                }
            },
        })),
        ("io.hashString", Value::Builtin(BuiltinFn {
            name: "io.hashString",
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => {
                        let hash = sha256_hex(s.as_bytes());
                        Ok(Value::String(Rc::new(hash)))
                    }
                    _ => Err("io.hashString expects a string".to_string()),
                }
            },
        })),
        
        // System info
        ("io.currentSystem", Value::Builtin(BuiltinFn {
            name: "io.currentSystem",
            arity: 0,
            func: |_args| {
                let arch = std::env::consts::ARCH;
                let os = std::env::consts::OS;
                Ok(Value::String(Rc::new(format!("{}-{}", arch, os))))
            },
        })),
    ]
}

/// Compute SHA-256 hash and return as hex string.
fn sha256_hex(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    // Simple hash for now - in production, use a proper SHA-256 implementation
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:016x}", hash)
}
