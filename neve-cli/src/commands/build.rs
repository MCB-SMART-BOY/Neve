//! The `neve build` command.
//! `neve build` 命令。
//!
//! Builds a package from a Neve file or flake.
//! 从 Neve 文件或 flake 构建软件包。

use crate::output;
use crate::platform::{BuildBackend, PlatformCapabilities, warn_limited_sandbox};
use neve_builder::{Builder, BuilderConfig};
use neve_derive::Derivation;
use neve_diagnostic::emit;
use neve_eval::{AstEvaluator, Value};
use neve_parser::parse;
use neve_store::Store;
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Run the build command.
/// 运行构建命令。
pub fn run(package: Option<&str>, backend_arg: &str) -> Result<(), String> {
    let start = Instant::now();

    // Detect platform and determine build backend
    // 检测平台并确定构建后端
    let caps = PlatformCapabilities::detect();
    let backend = match backend_arg {
        "auto" => caps.recommended_backend(),
        "native" => BuildBackend::Native,
        "docker" => BuildBackend::Docker,
        "simple" => BuildBackend::Simple,
        _ => {
            return Err(format!(
                "unknown backend: {}. Use 'native', 'docker', 'simple', or 'auto'",
                backend_arg
            ));
        }
    };

    // Warn about limited sandbox on non-Linux
    // 在非 Linux 上警告有限的沙箱支持
    if backend == BuildBackend::Simple && !caps.can_sandbox_build() {
        warn_limited_sandbox();
    }

    // Show backend info
    // 显示后端信息
    output::info(&format!("Build backend: {}", backend));

    // Determine what to build
    // 确定要构建的内容
    let (source_path, target_attr) = match package {
        Some(pkg) => {
            // Check if it's a path or a package name
            // 检查是路径还是软件包名称
            if pkg.contains('.') || pkg.starts_with('/') || pkg.starts_with("./") {
                (pkg.to_string(), None)
            } else {
                // Assume it's an attribute in the current flake/file
                // 假设它是当前 flake/文件中的属性
                ("flake.neve".to_string(), Some(pkg.to_string()))
            }
        }
        None => {
            // Look for default file in current directory
            // 在当前目录中查找默认文件
            if Path::new("flake.neve").exists() {
                ("flake.neve".to_string(), None)
            } else if Path::new("default.neve").exists() {
                ("default.neve".to_string(), None)
            } else {
                return Err("no flake.neve or default.neve found in current directory".to_string());
            }
        }
    };

    let path = Path::new(&source_path);
    if !path.exists() {
        return Err(format!("file not found: {}", source_path));
    }

    output::info(&format!("Evaluating {}", source_path));

    // Parse and evaluate the file
    // 解析并求值文件
    let source = fs::read_to_string(path)
        .map_err(|e| format!("cannot read file '{}': {}", source_path, e))?;

    let (ast, diagnostics) = parse(&source);

    for diag in &diagnostics {
        emit(&source, &source_path, diag);
    }

    if !diagnostics.is_empty() {
        return Err("parse error".to_string());
    }

    // Evaluate the file
    // 求值文件
    let mut evaluator = if let Some(parent) = path.parent() {
        AstEvaluator::new().with_base_path(parent.to_path_buf())
    } else {
        AstEvaluator::new()
    };

    let value = evaluator
        .eval_file(&ast)
        .map_err(|e| format!("evaluation error: {:?}", e))?;

    // Extract derivation(s) from the result
    // 从结果中提取派生
    let derivations = extract_derivations(&value, target_attr.as_deref())?;

    if derivations.is_empty() {
        return Err("no derivations found to build".to_string());
    }

    output::info(&format!(
        "Found {} derivation(s) to build",
        derivations.len()
    ));

    // Open the store
    // 打开存储
    let store = Store::open().map_err(|e| format!("cannot open store: {}", e))?;

    // Create builder
    // 创建构建器
    let config = BuilderConfig::default();
    let mut builder = Builder::with_config(store, config);

    // Build each derivation
    // 构建每个派生
    let mut built_count = 0;
    let mut failed_count = 0;

    for drv in &derivations {
        output::info(&format!("Building {}-{}", drv.name, drv.version));

        match builder.build(drv) {
            Ok(result) => {
                built_count += 1;

                for (output_name, store_path) in &result.outputs {
                    let path_display = store_path.display_name();
                    if output_name == "out" {
                        output::success(&format!("Built: {}", path_display));
                    } else {
                        output::success(&format!("Built {}: {}", output_name, path_display));
                    }
                }

                if result.duration_secs > 0.1 {
                    output::info(&format!("Build time: {:.2}s", result.duration_secs));
                }
            }
            Err(e) => {
                failed_count += 1;
                output::error(&format!("Failed to build {}: {}", drv.name, e));
            }
        }
    }

    let elapsed = start.elapsed();

    // Summary
    // 总结
    if failed_count == 0 {
        output::success(&format!(
            "Successfully built {} derivation(s) in {:.2}s",
            built_count,
            elapsed.as_secs_f64()
        ));
        Ok(())
    } else {
        output::error(&format!(
            "{} of {} build(s) failed",
            failed_count,
            derivations.len()
        ));
        Err("build failed".to_string())
    }
}

/// Extract derivations from an evaluated value.
/// 从求值结果中提取派生。
fn extract_derivations(value: &Value, target: Option<&str>) -> Result<Vec<Derivation>, String> {
    let mut derivations = Vec::new();

    // Handle different value structures
    // 处理不同的值结构
    match value {
        Value::Record(fields) => {
            // If a target is specified, look for that attribute
            // 如果指定了目标，查找该属性
            if let Some(target_name) = target {
                if let Some(target_value) = fields.get(target_name) {
                    return extract_derivations(target_value, None);
                } else {
                    return Err(format!("attribute '{}' not found", target_name));
                }
            }

            // Look for standard output attributes
            // 查找标准输出属性
            let current_system = current_system();

            // Check for flake-style outputs
            // 检查 flake 风格的输出
            if let Some(Value::Record(packages)) = fields.get("packages")
                && let Some(Value::Record(system_pkgs)) = packages.get(&current_system)
            {
                // Get the default package
                // 获取默认软件包
                if let Some(pkg) = system_pkgs.get("default")
                    && let Some(drv) = value_to_derivation(pkg)?
                {
                    derivations.push(drv);
                }
                // Or get all packages
                // 或获取所有软件包
                if derivations.is_empty() {
                    for (name, pkg) in system_pkgs.iter() {
                        if let Some(drv) = value_to_derivation(pkg)? {
                            derivations.push(drv);
                        } else {
                            output::warning(&format!("skipping non-derivation: {}", name));
                        }
                    }
                }
            }

            // Check for derivation-like structure directly
            // 直接检查类似派生的结构
            if derivations.is_empty()
                && let Some(drv) = value_to_derivation(value)?
            {
                derivations.push(drv);
            }

            // Look for 'output' or 'package' attribute
            // 查找 'output' 或 'package' 属性
            if derivations.is_empty() {
                for attr in &["output", "package", "default"] {
                    if let Some(val) = fields.get(*attr)
                        && let Some(drv) = value_to_derivation(val)?
                    {
                        derivations.push(drv);
                        break;
                    }
                }
            }
        }
        Value::List(items) => {
            // Build all derivations in the list
            // 构建列表中的所有派生
            for item in items.iter() {
                if let Some(drv) = value_to_derivation(item)? {
                    derivations.push(drv);
                }
            }
        }
        _ => {
            // Try to convert the value directly
            // 尝试直接转换值
            if let Some(drv) = value_to_derivation(value)? {
                derivations.push(drv);
            }
        }
    }

    Ok(derivations)
}

/// Try to convert a Value to a Derivation.
/// 尝试将 Value 转换为 Derivation。
fn value_to_derivation(value: &Value) -> Result<Option<Derivation>, String> {
    match value {
        Value::Record(fields) => {
            // Check if this looks like a derivation record
            // 检查是否看起来像派生记录
            let name = fields.get("name").and_then(|v| {
                if let Value::String(s) = v {
                    Some(s.to_string())
                } else {
                    None
                }
            });

            if name.is_none() {
                return Ok(None);
            }
            let name = name.unwrap();

            let version = fields
                .get("version")
                .and_then(|v| {
                    if let Value::String(s) = v {
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "0.0.0".to_string());

            let system = fields
                .get("system")
                .and_then(|v| {
                    if let Value::String(s) = v {
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(current_system);

            let builder = fields
                .get("builder")
                .and_then(|v| {
                    if let Value::String(s) = v {
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "/bin/sh".to_string());

            let mut drv = Derivation::builder(&name, &version)
                .system(&system)
                .builder_path(&builder);

            // Add build args
            // 添加构建参数
            if let Some(Value::List(args)) = fields.get("args") {
                for arg in args.iter() {
                    if let Value::String(s) = arg {
                        drv = drv.arg(s.to_string());
                    }
                }
            }

            // Add environment variables
            // 添加环境变量
            if let Some(Value::Record(env)) = fields.get("env") {
                for (key, val) in env.iter() {
                    if let Value::String(s) = val {
                        drv = drv.env(key.clone(), s.to_string());
                    }
                }
            }

            // Handle build script
            // 处理构建脚本
            if let Some(Value::String(build_script)) = fields.get("build") {
                drv = drv.arg("-c".to_string());
                drv = drv.arg(build_script.to_string());
            }

            Ok(Some(drv.build()))
        }
        _ => Ok(None),
    }
}

/// Get the current system identifier.
/// 获取当前系统标识符。
fn current_system() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    format!("{}-{}", arch, os)
}
