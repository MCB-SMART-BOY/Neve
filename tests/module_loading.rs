// Integration tests for module loading and imports
//
// Tests the module system including circular dependency detection,
// path resolution, and import chains.

use neve_hir::{ModuleLoadError, ModuleLoader};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper to create a test module file
fn create_test_module(dir: &Path, path: &[&str], content: &str) {
    let mut full_path = dir.to_path_buf();
    for (i, segment) in path.iter().enumerate() {
        full_path.push(segment);
        if i < path.len() - 1 {
            fs::create_dir_all(&full_path).unwrap();
        }
    }
    full_path.set_extension("neve");
    fs::write(full_path, content).unwrap();
}

#[test]
fn test_simple_module_loading() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a simple module
    create_test_module(
        root,
        &["math"],
        r#"

            pub fn add(x, y) = x + y;
            pub fn multiply(x, y) = x * y;
        "#,
    );

    // Create main module that imports it
    create_test_module(
        root,
        &["main"],
        r#"

            import math (add, multiply);

            fn compute(a, b) = multiply(add(a, b), 2);
        "#,
    );

    let mut loader = ModuleLoader::new(root);
    let result = loader.load_module(&["main".into()]);

    assert!(result.is_ok());
}

#[test]
fn test_nested_module_loading() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create nested module structure: utils/string.neve
    fs::create_dir_all(root.join("utils")).unwrap();
    create_test_module(
        root,
        &["utils", "string"],
        r#"

            pub fn toUpper(s) = s;  // Simplified
        "#,
    );

    // Create utils/mod.neve to re-export
    create_test_module(
        root,
        &["utils", "mod"],
        r#"

            pub import self.string (toUpper);
        "#,
    );

    // Create main that uses nested import
    create_test_module(
        root,
        &["main"],
        r#"

            import utils (toUpper);

            fn process(text) = toUpper(text);
        "#,
    );

    let mut loader = ModuleLoader::new(root);
    let result = loader.load_module(&["main".into()]);

    assert!(result.is_ok());
}

#[test]
fn test_circular_dependency_detection() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create circular dependency: a -> b -> c -> a
    create_test_module(
        root,
        &["a"],
        r#"

            import b (funcB);
            pub fn funcA() = funcB();
        "#,
    );

    create_test_module(
        root,
        &["b"],
        r#"

            import c (funcC);
            pub fn funcB() = funcC();
        "#,
    );

    create_test_module(
        root,
        &["c"],
        r#"

            import a (funcA);
            pub fn funcC() = funcA();
        "#,
    );

    let mut loader = ModuleLoader::new(root);
    let result = loader.load_module(&["a".into()]);

    // Should detect circular dependency
    assert!(result.is_err());
    match result {
        Err(ModuleLoadError::CircularDependency { module, chain }) => {
            // Verify the error contains the chain
            assert!(!chain.is_empty());
            assert_eq!(module, vec!["a".to_string()]);
        }
        _ => panic!("Expected CircularDependency error"),
    }
}

#[test]
fn test_circular_dependency_error_message() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create simple circular dependency: a -> b -> a
    create_test_module(
        root,
        &["a"],
        r#"

            import b (funcB);
            pub fn funcA() = funcB();
        "#,
    );

    create_test_module(
        root,
        &["b"],
        r#"

            import a (funcA);
            pub fn funcB() = funcA();
        "#,
    );

    let mut loader = ModuleLoader::new(root);
    let result = loader.load_module(&["a".into()]);

    assert!(result.is_err());
    match result {
        Err(ModuleLoadError::CircularDependency { module, chain }) => {
            let error_msg = format!("{}", ModuleLoadError::CircularDependency { module, chain });
            // Should contain "(cycle!)" or "circular" marker
            assert!(error_msg.contains("cycle") || error_msg.contains("circular"));
        }
        _ => panic!("Expected CircularDependency error"),
    }
}

#[test]
fn test_self_import() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create module structure
    fs::create_dir_all(root.join("mylib")).unwrap();
    create_test_module(
        root,
        &["mylib", "utils"],
        r#"

            pub fn helper(x) = x + 1;
        "#,
    );

    create_test_module(
        root,
        &["mylib", "mod"],
        r#"

            import self.utils (helper);

            pub fn process(x) = helper(x) * 2;
        "#,
    );

    let mut loader = ModuleLoader::new(root);
    let result = loader.load_module(&["mylib".into()]);

    assert!(result.is_ok());
}

#[test]
fn test_super_import() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create module structure
    fs::create_dir_all(root.join("mylib").join("submod")).unwrap();

    create_test_module(
        root,
        &["mylib", "config"],
        r#"

            pub let DEBUG = true;
        "#,
    );

    create_test_module(
        root,
        &["mylib", "submod", "worker"],
        r#"

            import super.config (DEBUG);

            pub fn run() = if DEBUG then "debug" else "release";
        "#,
    );

    let mut loader = ModuleLoader::new(root);
    let result = loader.load_module(&["mylib".into(), "submod".into(), "worker".into()]);

    assert!(result.is_ok());
}

#[test]
fn test_crate_import() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a top-level module with the function
    create_test_module(
        root,
        &["lib"],
        r#"

            pub fn rootFunc() = 42;
        "#,
    );

    // Create nested module that imports from crate root
    // Using crate.lib since crate root helpers aren't parsed as `crate (item)` syntax
    fs::create_dir_all(root.join("deep").join("nested")).unwrap();
    create_test_module(
        root,
        &["deep", "nested", "worker"],
        r#"

            import crate.lib (rootFunc);

            pub fn work() = rootFunc() + 1;
        "#,
    );

    let mut loader = ModuleLoader::new(root);
    let result = loader.load_module(&["deep".into(), "nested".into(), "worker".into()]);

    assert!(result.is_ok());
}

#[test]
fn test_module_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_module(
        root,
        &["main"],
        r#"

            import nonexistent (func);

            fn test() = func();
        "#,
    );

    let mut loader = ModuleLoader::new(root);
    let result = loader.load_module(&["main".into()]);

    // Should fail with module not found error
    assert!(result.is_err());
    match result {
        Err(ModuleLoadError::NotFound(_)) => {} // Expected
        Err(e) => panic!("Expected NotFound error, got {:?}", e),
        Ok(_) => panic!("Expected error but got success"),
    }
}

#[test]
fn test_diamond_dependency() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Diamond pattern: main -> a, b -> c
    // Both a and b depend on c, main depends on both a and b
    create_test_module(
        root,
        &["c"],
        r#"

            pub fn funcC() = 42;
        "#,
    );

    create_test_module(
        root,
        &["a"],
        r#"

            import c (funcC);
            pub fn funcA() = funcC() + 1;
        "#,
    );

    create_test_module(
        root,
        &["b"],
        r#"

            import c (funcC);
            pub fn funcB() = funcC() * 2;
        "#,
    );

    create_test_module(
        root,
        &["main"],
        r#"

            import a (funcA);
            import b (funcB);

            fn compute() = funcA() + funcB();
        "#,
    );

    let mut loader = ModuleLoader::new(root);
    let result = loader.load_module(&["main".into()]);

    // Diamond dependencies are fine, not circular
    assert!(result.is_ok());
}
