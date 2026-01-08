//! Integration tests for type checking across modules.

use neve_hir::ModuleLoader;
use neve_typeck::TypeChecker;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write_module(dir: &Path, path: &[&str], content: &str) {
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
fn test_typeck_imported_function() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    write_module(
        root,
        &["math"],
        r#"
            pub fn add(x: Int, y: Int) -> Int = x + y;
        "#,
    );

    write_module(
        root,
        &["main"],
        r#"
            import math (add);
            let result = add(1, 2);
        "#,
    );

    let mut loader = ModuleLoader::new(root);
    loader.load_module(&["main".into()]).unwrap();

    let mut global_types = std::collections::HashMap::new();
    let mut global_spans = std::collections::HashMap::new();
    for module_id in loader.load_order() {
        let module = loader.hir_module(*module_id).unwrap();
        let (types, spans) = TypeChecker::collect_signatures(module);
        global_types.extend(types);
        global_spans.extend(spans);
    }

    for module_id in loader.load_order() {
        let module = loader.hir_module(*module_id).unwrap();
        let mut checker = TypeChecker::with_global_env(global_types.clone(), global_spans.clone());
        checker.check(module);
        let diagnostics = checker.diagnostics();
        assert!(
            diagnostics.is_empty(),
            "unexpected type errors: {:?}",
            diagnostics
        );
    }
}
