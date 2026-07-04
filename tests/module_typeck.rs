//! Integration tests for type checking across modules.

use neve_frontend::rewrite_diagnostics_with_module_set;
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
            fn add(x: Int, y: Int) -> Int = x + y;
        "#,
    );

    write_module(
        root,
        &["main"],
        r#"
            use math (add);
            let result = add(1, 2);
        "#,
    );

    let mut loader = ModuleLoader::new(root);
    loader.load_module(&["main".into()]).unwrap();

    let mut global_types = std::collections::HashMap::new();
    let mut global_spans = std::collections::HashMap::new();
    let mut global_fn_bounds = std::collections::HashMap::new();
    for module_id in loader.load_order() {
        let module = loader.hir_module(*module_id).unwrap();
        let (types, spans, bounds) = TypeChecker::collect_signatures(module);
        global_types.extend(types);
        global_spans.extend(spans);
        global_fn_bounds.extend(bounds);
    }

    for module_id in loader.load_order() {
        let module = loader.hir_module(*module_id).unwrap();
        let mut checker = TypeChecker::with_global_env(global_types.clone(), global_spans.clone(), global_fn_bounds.clone());
        checker.check(module);
        let diagnostics = checker.diagnostics();
        assert!(
            diagnostics.is_empty(),
            "unexpected type errors: {:?}",
            diagnostics
        );
    }
}

#[test]
fn test_module_typeck_formats_imported_named_types_readably() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    write_module(
        root,
        &["types"],
        r#"
            type User = { name: String };
        "#,
    );

    write_module(
        root,
        &["main"],
        r#"
            use types (User);
            fn broken(x: User) -> Int = x;
        "#,
    );

    let mut loader = ModuleLoader::new(root);
    loader.load_module(&["main".into()]).unwrap();

    let mut global_types = std::collections::HashMap::new();
    let mut global_spans = std::collections::HashMap::new();
    let mut global_fn_bounds = std::collections::HashMap::new();
    for module_id in loader.load_order() {
        let module = loader.hir_module(*module_id).unwrap();
        let (types, spans, bounds) = TypeChecker::collect_signatures(module);
        global_types.extend(types);
        global_spans.extend(spans);
        global_fn_bounds.extend(bounds);
    }

    let modules: Vec<_> = loader
        .load_order()
        .iter()
        .filter_map(|module_id| loader.hir_module(*module_id))
        .collect();
    let main_module = loader
        .hir_module(*loader.load_order().last().unwrap())
        .unwrap();

    let mut checker = TypeChecker::with_global_env(global_types, global_spans, global_fn_bounds);
    checker.check(main_module);
    let diagnostics =
        rewrite_diagnostics_with_module_set(checker.diagnostics(), modules.iter().copied());

    assert!(
        diagnostics
            .iter()
            .any(|diag| !diag.message.contains("Type#")),
        "expected readable type names in diagnostics, got {diagnostics:?}"
    );
    for diagnostic in &diagnostics {
        assert!(
            !diagnostic.message.contains("Type#"),
            "unexpected raw type placeholder in message: {diagnostic:?}"
        );
        for label in &diagnostic.labels {
            assert!(
                !label.message.contains("Type#"),
                "unexpected raw type placeholder in label: {diagnostic:?}"
            );
        }
    }
}
