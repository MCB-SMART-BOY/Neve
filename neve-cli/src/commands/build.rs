//! The `neve build` command.
//! `neve build` 命令。
//!
//! Builds a package from a Neve file or flake.
//! 从 Neve 文件或 flake 构建软件包。

use crate::output;
use crate::platform::{BuildBackend, PlatformCapabilities, warn_limited_sandbox};
use neve_builder::{BuildBackend as BuilderBackend, Builder, BuilderConfig};
use neve_derive::{Derivation, StorePath};
use neve_diagnostic::emit;
use neve_eval::{Value, compat::AstEvaluator};
use neve_parser::parse;
use neve_std::std_module_overrides;
use neve_store::{BinaryCache, CacheConfig, Store};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Run the build command.
/// 运行构建命令。
pub struct BuildRunArgs<'a> {
    pub package: Option<&'a str>,
    pub backend_arg: &'a str,
    pub cli_cache_urls: &'a [String],
    pub cli_cache_dirs: &'a [String],
    pub cli_substitute: bool,
    pub cli_cache_upload: bool,
    pub cli_cache_public_keys: &'a [String],
    pub cli_cache_private_keys: &'a [String],
}

pub fn run(args: BuildRunArgs<'_>) -> Result<(), String> {
    let BuildRunArgs {
        package,
        backend_arg,
        cli_cache_urls,
        cli_cache_dirs,
        cli_substitute,
        cli_cache_upload,
        cli_cache_public_keys,
        cli_cache_private_keys,
    } = args;

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

    // Validate backend availability
    // 验证后端可用性
    match backend {
        BuildBackend::Native if !caps.native_sandbox => {
            return Err(
                "native backend is not available on this platform. Use --backend docker or --backend simple"
                    .to_string(),
            );
        }
        BuildBackend::Docker if !caps.docker_available => {
            return Err(
                "docker backend requested but Docker is not available. Install Docker or use --backend simple"
                    .to_string(),
            );
        }
        _ => {}
    }

    // Warn about limited sandbox on non-Linux
    // 在非 Linux 上警告有限的沙箱支持
    if backend == BuildBackend::Simple && !caps.can_sandbox_build() {
        warn_limited_sandbox();
    }

    // Show backend info using debug for less verbose output
    // 使用 debug 显示后端信息以减少冗余输出
    output::debug(&format!("Build backend: {}", backend));
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
    let mut evaluator = AstEvaluator::new().with_module_overrides(std_module_overrides());
    if let Some(parent) = path.parent() {
        evaluator = evaluator.with_base_path(parent.to_path_buf());
    }

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

    // Configure binary caches (substituter + optional upload).
    // 配置二进制缓存（substituter + 可选上传）。
    let cache_settings = BuildCacheSettings::from_inputs(
        cli_cache_urls,
        cli_cache_dirs,
        cli_substitute,
        cli_cache_upload,
        cli_cache_public_keys,
        cli_cache_private_keys,
    );
    let mut binary_cache = if cache_settings.has_any_cache() {
        Some(configure_binary_cache(&cache_settings)?)
    } else {
        None
    };
    let cache_store = if binary_cache.is_some() {
        Some(Store::open().map_err(|e| format!("cannot open store: {}", e))?)
    } else {
        None
    };

    if let Some(cache) = &binary_cache {
        output::info(&format!(
            "Binary cache configured: {} source(s), substitute={}, upload={}, signature_verify={}, signature_sign={}",
            cache.stats().total_caches,
            cache_settings.substitute_enabled,
            cache_settings.upload_enabled,
            !cache_settings.public_keys.is_empty(),
            !cache_settings.private_keys.is_empty()
        ));
    }

    // Open the store
    // 打开存储
    let store = Store::open().map_err(|e| format!("cannot open store: {}", e))?;

    // Create builder
    // 创建构建器
    let mut config = BuilderConfig::default();
    config.backend = match backend {
        BuildBackend::Native => BuilderBackend::Native,
        BuildBackend::Docker => BuilderBackend::Docker,
        BuildBackend::Simple => BuilderBackend::Simple,
    };
    config.sandbox = config.backend != BuilderBackend::Simple;
    let mut builder = Builder::with_config(store, config);

    // Build each derivation
    // 构建每个派生
    let mut built_count = 0;
    let mut substituted_count = 0;
    let mut failed_count = 0;
    let total = derivations.len();

    let mut progress = output::ProgressBar::new(total, "Building");

    for drv in &derivations {
        output::highlight(&format!("▶ Building {}-{}", drv.name, drv.version));

        if cache_settings.substitute_enabled
            && let (Some(cache), Some(store_for_cache)) =
                (binary_cache.as_mut(), cache_store.as_ref())
        {
            match try_substitute_derivation(cache, store_for_cache, drv) {
                Ok(Some(outputs)) => {
                    built_count += 1;
                    substituted_count += 1;
                    for (output_name, store_path) in &outputs {
                        let path_display = store_path.display_name();
                        if output_name == "out" {
                            output::success(&format!("Substituted: {}", path_display));
                        } else {
                            output::success(&format!(
                                "Substituted {}: {}",
                                output_name, path_display
                            ));
                        }
                    }
                    progress.inc();
                    continue;
                }
                Ok(None) => {}
                Err(err) => {
                    output::warning(&format!(
                        "Substitute attempt failed for {}-{}: {}. Falling back to local build.",
                        drv.name, drv.version, err
                    ));
                }
            }
        }

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
                    output::info(&format!(
                        "Build time: {}",
                        output::format_duration(result.duration_secs as u64)
                    ));
                }

                if cache_settings.upload_enabled
                    && let Some(cache) = binary_cache.as_mut()
                {
                    for (output_name, store_path) in &result.outputs {
                        if let Err(err) = cache.push(store_path) {
                            output::warning(&format!(
                                "Failed to upload {} output '{}' to cache: {}",
                                drv.name, output_name, err
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                failed_count += 1;
                output::error(&format!("Failed to build {}: {}", drv.name, e));
            }
        }
        progress.inc();
    }

    progress.finish();

    let elapsed = start.elapsed();

    // Summary
    // 总结
    if failed_count == 0 {
        output::success(&format!(
            "Successfully built {} derivation(s) in {:.2}s",
            built_count,
            elapsed.as_secs_f64()
        ));
        if substituted_count > 0 {
            output::info(&format!(
                "Substituted {} derivation(s) from binary cache",
                substituted_count
            ));
        }
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

#[derive(Debug, Clone)]
struct BuildCacheSettings {
    cache_urls: Vec<String>,
    cache_dirs: Vec<PathBuf>,
    substitute_enabled: bool,
    upload_enabled: bool,
    public_keys: Vec<String>,
    private_keys: Vec<String>,
}

impl BuildCacheSettings {
    fn from_inputs(
        cli_cache_urls: &[String],
        cli_cache_dirs: &[String],
        cli_substitute: bool,
        cli_cache_upload: bool,
        cli_cache_public_keys: &[String],
        cli_cache_private_keys: &[String],
    ) -> Self {
        let mut cache_urls = cli_cache_urls
            .iter()
            .map(|u| u.trim().to_string())
            .filter(|u| !u.is_empty())
            .collect::<Vec<_>>();
        if cache_urls.is_empty() {
            cache_urls = parse_env_list("NEVE_BINARY_CACHE_URLS");
        }

        let mut cache_dirs = cli_cache_dirs
            .iter()
            .map(|d| PathBuf::from(d.trim()))
            .filter(|d| !d.as_os_str().is_empty())
            .collect::<Vec<_>>();
        if cache_dirs.is_empty() {
            cache_dirs = parse_env_list("NEVE_BINARY_CACHE_LOCAL_DIRS")
                .into_iter()
                .map(PathBuf::from)
                .collect();
        }

        Self {
            cache_urls,
            cache_dirs,
            substitute_enabled: cli_substitute && parse_env_bool("NEVE_SUBSTITUTE", true),
            upload_enabled: cli_cache_upload || parse_env_bool("NEVE_BINARY_CACHE_UPLOAD", false),
            public_keys: if cli_cache_public_keys.is_empty() {
                parse_env_keys(
                    "NEVE_BINARY_CACHE_PUBLIC_KEYS",
                    "NEVE_BINARY_CACHE_PUBLIC_KEY",
                )
            } else {
                cli_cache_public_keys
                    .iter()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .collect()
            },
            private_keys: if cli_cache_private_keys.is_empty() {
                parse_env_keys(
                    "NEVE_BINARY_CACHE_PRIVATE_KEYS",
                    "NEVE_BINARY_CACHE_PRIVATE_KEY",
                )
            } else {
                cli_cache_private_keys
                    .iter()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .collect()
            },
        }
    }

    fn has_any_cache(&self) -> bool {
        !self.cache_urls.is_empty() || !self.cache_dirs.is_empty()
    }
}

fn parse_env_list(name: &str) -> Vec<String> {
    std::env::var(name)
        .ok()
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_env_keys(list_var: &str, fallback_single_var: &str) -> Vec<String> {
    let list_values = parse_env_list(list_var);
    if !list_values.is_empty() {
        return list_values;
    }

    std::env::var(fallback_single_var)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| vec![value])
        .unwrap_or_default()
}

fn parse_env_bool(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(v) => match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        },
        Err(_) => default,
    }
}

fn validate_key_count(kind: &str, keys: &[String], total_caches: usize) -> Result<(), String> {
    if keys.len() > 1 && keys.len() != total_caches {
        return Err(format!(
            "{} count mismatch: got {}, but cache sources are {} (expected 0, 1, or {})",
            kind,
            keys.len(),
            total_caches,
            total_caches
        ));
    }
    Ok(())
}

fn key_for_cache(keys: &[String], cache_index: usize) -> Option<String> {
    if keys.is_empty() {
        None
    } else if keys.len() == 1 {
        Some(keys[0].clone())
    } else {
        keys.get(cache_index).cloned()
    }
}

fn configure_binary_cache(settings: &BuildCacheSettings) -> Result<BinaryCache, String> {
    let store = Store::open().map_err(|e| format!("cannot open store: {}", e))?;
    let mut cache =
        BinaryCache::new(store).map_err(|e| format!("cannot init binary cache: {}", e))?;

    let total_caches = settings.cache_dirs.len() + settings.cache_urls.len();
    validate_key_count("cache public key", &settings.public_keys, total_caches)?;
    validate_key_count("cache private key", &settings.private_keys, total_caches)?;

    let mut priority: i32 = 100;
    let mut cache_index: usize = 0;
    for (idx, dir) in settings.cache_dirs.iter().enumerate() {
        cache.add_cache(CacheConfig {
            name: format!("local-{}", idx),
            local_dir: Some(dir.clone()),
            private_key: key_for_cache(&settings.private_keys, cache_index),
            upload: settings.upload_enabled,
            priority,
            ..Default::default()
        });
        priority -= 1;
        cache_index += 1;
    }

    for (idx, url) in settings.cache_urls.iter().enumerate() {
        cache.add_cache(CacheConfig {
            name: format!("remote-{}", idx),
            url: Some(url.clone()),
            public_key: key_for_cache(&settings.public_keys, cache_index),
            private_key: key_for_cache(&settings.private_keys, cache_index),
            upload: settings.upload_enabled,
            priority,
            ..Default::default()
        });
        priority -= 1;
        cache_index += 1;
    }

    Ok(cache)
}

fn output_store_name(drv: &Derivation, output_name: &str) -> String {
    if output_name == "out" {
        format!("{}-{}", drv.name, drv.version)
    } else {
        format!("{}-{}-{}", drv.name, drv.version, output_name)
    }
}

fn candidate_output_paths(drv: &Derivation) -> Option<Vec<(String, StorePath)>> {
    let mut paths = Vec::with_capacity(drv.outputs.len());
    for (name, output) in &drv.outputs {
        if let Some(path) = &output.path {
            paths.push((name.clone(), path.clone()));
            continue;
        }

        if let Some(expected_hash) = output.expected_hash {
            let store_name = output_store_name(drv, name);
            let path = StorePath::new(expected_hash, store_name);
            paths.push((name.clone(), path));
            continue;
        }

        // At least one output path is unknown before build, so substitution can't
        // safely satisfy this derivation up front.
        // 至少有一个输出路径在构建前未知，因此无法提前安全替换该派生。
        return None;
    }

    Some(paths)
}

fn try_substitute_derivation(
    cache: &mut BinaryCache,
    store: &Store,
    drv: &Derivation,
) -> Result<Option<Vec<(String, StorePath)>>, String> {
    let Some(candidates) = candidate_output_paths(drv) else {
        return Ok(None);
    };

    let mut resolved = Vec::with_capacity(candidates.len());

    for (output_name, path) in candidates {
        if store.path_exists(&path) {
            resolved.push((output_name, path));
            continue;
        }

        let cached = cache
            .query(&path)
            .map_err(|e| format!("cache query failed: {}", e))?;
        let Some(cached) = cached else {
            return Ok(None);
        };

        cache
            .fetch(&cached)
            .map_err(|e| format!("cache fetch failed for {}: {}", path.display_name(), e))?;

        if !store.path_exists(&path) {
            return Err(format!(
                "cache fetch reported success but store path is missing: {}",
                path.display_name()
            ));
        }

        resolved.push((output_name, path));
    }

    Ok(Some(resolved))
}

#[cfg(test)]
mod tests {
    use super::*;
    use neve_derive::{Hash, Output};

    #[test]
    fn test_candidate_output_paths_for_fixed_output() {
        let hash = Hash::of(b"fixed-output");
        let drv = Derivation::builder("demo", "1.0.0")
            .output(Output::fixed("out", hash, neve_derive::HashMode::Recursive))
            .build();

        let candidates = candidate_output_paths(&drv).expect("fixed output should be predictable");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0, "out");
        assert_eq!(candidates[0].1.hash(), &hash);
        assert_eq!(candidates[0].1.name(), "demo-1.0.0");
    }

    #[test]
    fn test_candidate_output_paths_returns_none_for_unknown_output() {
        let drv = Derivation::builder("demo", "1.0.0").build();
        assert!(candidate_output_paths(&drv).is_none());
    }

    #[test]
    fn test_output_store_name_non_out_suffixes_output_name() {
        let drv = Derivation::builder("demo", "1.0.0").build();
        assert_eq!(output_store_name(&drv, "doc"), "demo-1.0.0-doc");
    }

    #[test]
    fn test_validate_key_count_accepts_zero_one_or_total() {
        assert!(validate_key_count("cache public key", &[], 3).is_ok());
        assert!(validate_key_count("cache public key", &["k0".to_string()], 3).is_ok());
        assert!(
            validate_key_count(
                "cache public key",
                &["k0".to_string(), "k1".to_string(), "k2".to_string()],
                3
            )
            .is_ok()
        );
    }

    #[test]
    fn test_validate_key_count_rejects_partial_mapping() {
        let err = validate_key_count(
            "cache private key",
            &["k0".to_string(), "k1".to_string()],
            3,
        )
        .unwrap_err();
        assert!(err.contains("count mismatch"));
    }

    #[test]
    fn test_key_for_cache_mapping() {
        let none = key_for_cache(&[], 0);
        assert!(none.is_none());

        let single = vec!["k0".to_string()];
        assert_eq!(key_for_cache(&single, 0).as_deref(), Some("k0"));
        assert_eq!(key_for_cache(&single, 5).as_deref(), Some("k0"));

        let many = vec!["k0".to_string(), "k1".to_string(), "k2".to_string()];
        assert_eq!(key_for_cache(&many, 0).as_deref(), Some("k0"));
        assert_eq!(key_for_cache(&many, 1).as_deref(), Some("k1"));
        assert_eq!(key_for_cache(&many, 2).as_deref(), Some("k2"));
        assert!(key_for_cache(&many, 3).is_none());
    }
}
