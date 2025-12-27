//! The `neve build` command.
//!
//! Builds a package from a Neve file or flake.

use std::fs;
use std::path::Path;
use std::time::Instant;
use neve_parser::parse;
use neve_diagnostic::emit;
use neve_eval::{AstEvaluator, Value};
use neve_derive::Derivation;
use neve_store::Store;
use neve_builder::{Builder, BuilderConfig};
use crate::output;

pub fn run(package: Option<&str>) -> Result<(), String> {
    let start = Instant::now();
    
    // Determine what to build
    let (source_path, target_attr) = match package {
        Some(pkg) => {
            // Check if it's a path or a package name
            if pkg.contains('.') || pkg.starts_with('/') || pkg.starts_with("./") {
                (pkg.to_string(), None)
            } else {
                // Assume it's an attribute in the current flake/file
                ("flake.neve".to_string(), Some(pkg.to_string()))
            }
        }
        None => {
            // Look for default file in current directory
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
    let mut evaluator = if let Some(parent) = path.parent() {
        AstEvaluator::new().with_base_path(parent.to_path_buf())
    } else {
        AstEvaluator::new()
    };
    
    let value = evaluator.eval_file(&ast)
        .map_err(|e| format!("evaluation error: {:?}", e))?;
    
    // Extract derivation(s) from the result
    let derivations = extract_derivations(&value, target_attr.as_deref())?;
    
    if derivations.is_empty() {
        return Err("no derivations found to build".to_string());
    }
    
    output::info(&format!("Found {} derivation(s) to build", derivations.len()));
    
    // Open the store
    let store = Store::open()
        .map_err(|e| format!("cannot open store: {}", e))?;
    
    // Create builder
    let config = BuilderConfig::default();
    let mut builder = Builder::with_config(store, config);
    
    // Build each derivation
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
    if failed_count == 0 {
        output::success(&format!(
            "Successfully built {} derivation(s) in {:.2}s",
            built_count, elapsed.as_secs_f64()
        ));
        Ok(())
    } else {
        output::error(&format!(
            "{} of {} build(s) failed",
            failed_count, derivations.len()
        ));
        Err("build failed".to_string())
    }
}

/// Extract derivations from an evaluated value.
fn extract_derivations(value: &Value, target: Option<&str>) -> Result<Vec<Derivation>, String> {
    let mut derivations = Vec::new();
    
    // Handle different value structures
    match value {
        Value::Record(fields) => {
            // If a target is specified, look for that attribute
            if let Some(target_name) = target {
                if let Some(target_value) = fields.get(target_name) {
                    return extract_derivations(target_value, None);
                } else {
                    return Err(format!("attribute '{}' not found", target_name));
                }
            }
            
            // Look for standard output attributes
            let current_system = current_system();
            
            // Check for flake-style outputs
            if let Some(Value::Record(packages)) = fields.get("packages") {
                if let Some(Value::Record(system_pkgs)) = packages.get(&current_system) {
                    // Get the default package
                    if let Some(pkg) = system_pkgs.get("default") {
                        if let Some(drv) = value_to_derivation(pkg)? {
                            derivations.push(drv);
                        }
                    }
                    // Or get all packages
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
            }
            
            // Check for derivation-like structure directly
            if derivations.is_empty() {
                if let Some(drv) = value_to_derivation(value)? {
                    derivations.push(drv);
                }
            }
            
            // Look for 'output' or 'package' attribute
            if derivations.is_empty() {
                for attr in &["output", "package", "default"] {
                    if let Some(val) = fields.get(*attr) {
                        if let Some(drv) = value_to_derivation(val)? {
                            derivations.push(drv);
                            break;
                        }
                    }
                }
            }
        }
        Value::List(items) => {
            // Build all derivations in the list
            for item in items.iter() {
                if let Some(drv) = value_to_derivation(item)? {
                    derivations.push(drv);
                }
            }
        }
        _ => {
            // Try to convert the value directly
            if let Some(drv) = value_to_derivation(value)? {
                derivations.push(drv);
            }
        }
    }
    
    Ok(derivations)
}

/// Try to convert a Value to a Derivation.
fn value_to_derivation(value: &Value) -> Result<Option<Derivation>, String> {
    match value {
        Value::Record(fields) => {
            // Check if this looks like a derivation record
            let name = fields.get("name")
                .and_then(|v| if let Value::String(s) = v { Some(s.to_string()) } else { None });
            
            if name.is_none() {
                return Ok(None);
            }
            let name = name.unwrap();
            
            let version = fields.get("version")
                .and_then(|v| if let Value::String(s) = v { Some(s.to_string()) } else { None })
                .unwrap_or_else(|| "0.0.0".to_string());
            
            let system = fields.get("system")
                .and_then(|v| if let Value::String(s) = v { Some(s.to_string()) } else { None })
                .unwrap_or_else(current_system);
            
            let builder = fields.get("builder")
                .and_then(|v| if let Value::String(s) = v { Some(s.to_string()) } else { None })
                .unwrap_or_else(|| "/bin/sh".to_string());
            
            let mut drv = Derivation::builder(&name, &version)
                .system(&system)
                .builder_path(&builder);
            
            // Add build args
            if let Some(Value::List(args)) = fields.get("args") {
                for arg in args.iter() {
                    if let Value::String(s) = arg {
                        drv = drv.arg(s.to_string());
                    }
                }
            }
            
            // Add environment variables
            if let Some(Value::Record(env)) = fields.get("env") {
                for (key, val) in env.iter() {
                    if let Value::String(s) = val {
                        drv = drv.env(key.clone(), s.to_string());
                    }
                }
            }
            
            // Handle build script
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
fn current_system() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    format!("{}-{}", arch, os)
}
