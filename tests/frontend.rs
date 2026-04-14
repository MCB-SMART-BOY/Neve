//! Integration tests for the frontend analysis pipeline.
//! 前端分析管线的集成测试。

use neve_diagnostic::{DiagnosticKind, ErrorCode};
use neve_frontend::{analyze_snippet_ast, analyze_source};
use neve_hir::ItemKind;
use neve_parser::parse;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_frontend_reports_parse_errors() {
    let result = analyze_source("let x =");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Parser),
        "expected parser diagnostics"
    );
}

#[test]
fn test_frontend_reports_type_errors() {
    let result = analyze_source("let x = 1 + true;");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected type diagnostics"
    );
}

#[test]
fn test_frontend_formats_named_types_readably_in_diagnostics() {
    let result = analyze_source(
        r#"
            struct User {};
            fn broken(x: User) -> Int = x.name;
        "#,
    );

    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected type diagnostics, got {:?}",
        result.diagnostics
    );

    for diagnostic in &result.diagnostics {
        assert!(
            !diagnostic.message.contains("Type#"),
            "unexpected raw type placeholder in message: {:?}",
            diagnostic
        );
        for label in &diagnostic.labels {
            assert!(
                !label.message.contains("Type#"),
                "unexpected raw type placeholder in label: {:?}",
                diagnostic
            );
        }
        for note in &diagnostic.notes {
            assert!(
                !note.contains("Type#"),
                "unexpected raw type placeholder in note: {:?}",
                diagnostic
            );
        }
        if let Some(help) = &diagnostic.help {
            assert!(
                !help.contains("Type#"),
                "unexpected raw type placeholder in help: {:?}",
                diagnostic
            );
        }
    }
}

#[test]
fn test_frontend_accepts_record_field_access_after_record_binding() {
    let result = analyze_source(
        r#"
            let config = #{ port = 40, host = "localhost" };
            let x = config.port;
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_lazy_force_pipeline() {
    let result = analyze_source(
        r#"
            let thunk = lazy 42;
            let x = force(thunk);
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_or_and_binding_patterns() {
    let result = analyze_source(
        r#"
            let a = match (1, 2) { (0, v) | (1, v) -> v, _ -> 0 };
            let b = match 42 { n @ 42 -> n, _ -> 0 };
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_list_rest_patterns() {
    let result = analyze_source(
        r#"
            let x = match [1, 2, 3, 4] {
                [first, ..middle, last] -> match middle {
                    [a, b] -> first + a + b + last,
                    _ -> 0,
                },
                _ -> 0,
            };
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_reports_impl_method_type_errors() {
    let result = analyze_source(
        r#"
            struct Counter {};
            impl Counter {
                fn value(self) -> Int = true;
            };
        "#,
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected impl method type diagnostics, got {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_reports_trait_impl_signature_mismatch() {
    let result = analyze_source(
        r#"
            trait Show { fn show(self) -> Int; };
            struct Counter {};
            impl Show for Counter {
                fn show(self) -> String = "counter";
            };
        "#,
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected trait impl signature diagnostics, got {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_self_and_assoc_type_use_sites() {
    let result = analyze_source(
        r#"
            trait Iterator {
                type Item;
                fn first(self) -> Self.Item;
            };
            struct Counter {};
            impl Iterator for Counter {
                type Item = Int;
                fn first(self) -> Self.Item = 1;
            };
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_assoc_bound_through_canonical_self_assoc_binding() {
    let result = analyze_source(
        r#"
            trait Show { };
            trait Iterator { type Item: Show; type Alias; };
            struct Foo {};
            impl Show for Int { };
            impl Iterator for Foo {
                type Alias = Int;
                type Item = Self.Alias;
            };
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_assoc_bound_failure_uses_canonical_type_rendering() {
    let result = analyze_source(
        r#"
            trait Show { };
            trait Iterator { type Item: Show; type Alias; };
            struct Foo {};
            impl Iterator for Foo {
                type Alias = Int;
                type Item = Self.Alias;
            };
        "#,
    );
    let diag = result
        .diagnostics
        .iter()
        .find(|diag| {
            diag.message.contains(
                "associated type 'Item' in impl of trait 'Iterator' must satisfy bound 'Show'",
            )
        })
        .expect("expected assoc-type bound diagnostic");

    assert!(
        diag.labels.iter().any(|label| label
            .message
            .contains("associated type resolves to `Int` here")),
        "expected canonical assoc-bound label, got {:?}",
        diag
    );
    assert!(
        diag.notes
            .iter()
            .any(|note| note.contains("`Int` does not implement `Show`")),
        "expected canonical assoc-bound note, got {:?}",
        diag
    );
}

#[test]
fn test_frontend_exposes_assoc_projection_resolutions_for_explicit_self_item_use_sites() {
    let result = analyze_source(
        r#"
            trait Iterator {
                type Item;
                fn first(self, fallback: Self.Item) -> Self.Item;
            };
            impl Iterator for Int {
                type Item = String;
                fn first(self, fallback: Self.Item) -> Self.Item = fallback;
            };
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    let impl_def = result
        .hir
        .items
        .iter()
        .find_map(|item| match &item.kind {
            ItemKind::Impl(impl_def) => Some(impl_def),
            _ => None,
        })
        .expect("impl definition should exist");
    let method = impl_def.items.first().expect("impl method should exist");
    let fallback_span = method.params[1].ty.span;
    let return_span = method.return_ty.span;

    let fallback_ty = result
        .semantics
        .assoc_projection_resolution(fallback_span)
        .expect("fallback Self.Item projection should be recorded");
    let return_ty = result
        .semantics
        .assoc_projection_resolution(return_span)
        .expect("return Self.Item projection should be recorded");

    assert_eq!(
        neve_frontend::format_type_in_module(fallback_ty, &result.hir),
        "String"
    );
    assert_eq!(
        neve_frontend::format_type_in_module(return_ty, &result.hir),
        "String"
    );
}

#[test]
fn test_frontend_keeps_trait_self_assoc_spans_source_level_when_impl_is_present() {
    let result = analyze_source(
        r#"
            trait Iterator {
                type Item;
                fn first(self, fallback: Self.Item) -> Self.Item;
            };
            impl Iterator for Int {
                type Item = String;
                fn first(self, fallback: Self.Item) -> Self.Item = fallback;
            };
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    let trait_def = result
        .hir
        .items
        .iter()
        .find_map(|item| match &item.kind {
            ItemKind::Trait(trait_def) => Some(trait_def),
            _ => None,
        })
        .expect("trait definition should exist");
    let impl_def = result
        .hir
        .items
        .iter()
        .find_map(|item| match &item.kind {
            ItemKind::Impl(impl_def) => Some(impl_def),
            _ => None,
        })
        .expect("impl definition should exist");

    let trait_method = trait_def.items.first().expect("trait method should exist");
    let impl_method = impl_def.items.first().expect("impl method should exist");

    assert!(
        result
            .semantics
            .assoc_projection_resolution(trait_method.params[1].span)
            .is_none(),
        "trait param Self.Item span should stay source-level"
    );
    assert!(
        result
            .semantics
            .assoc_projection_resolution(trait_method.return_ty.span)
            .is_none(),
        "trait return Self.Item span should stay source-level"
    );

    let impl_param_ty = result
        .semantics
        .assoc_projection_resolution(impl_method.params[1].ty.span)
        .expect("impl param Self.Item projection should be recorded");
    let impl_return_ty = result
        .semantics
        .assoc_projection_resolution(impl_method.return_ty.span)
        .expect("impl return Self.Item projection should be recorded");

    assert_eq!(
        neve_frontend::format_type_in_module(impl_param_ty, &result.hir),
        "String"
    );
    assert_eq!(
        neve_frontend::format_type_in_module(impl_return_ty, &result.hir),
        "String"
    );
}

#[test]
fn test_frontend_trait_signature_mismatch_uses_projection_labels() {
    let result = analyze_source(
        r#"
            trait Iterator {
                type Item;
                fn first(self, fallback: Self.Item) -> Self.Item;
            };
            impl Iterator for Int {
                type Item = String;
                fn first(self, fallback: Int) -> Int = fallback;
            };
        "#,
    );
    let diag = result
        .diagnostics
        .iter()
        .find(|diag| {
            diag.message
                .contains("does not match trait `Iterator` signature")
        })
        .expect("expected impl signature mismatch diagnostic");

    assert!(
        diag.labels.iter().any(|label| label
            .message
            .contains("`Self.Item` resolves to `String` here")),
        "expected canonical assoc-projection label, got {:?}",
        diag
    );
}

#[test]
fn test_frontend_impl_method_body_mismatch_uses_projection_labels() {
    let result = analyze_source(
        r#"
            trait Iterator {
                type Item;
                fn first(self) -> Self.Item;
            };
            impl Iterator for Int {
                type Item = String;
                fn first(self) -> Self.Item = true;
            };
        "#,
    );
    let diag = result
        .diagnostics
        .iter()
        .find(|diag| diag.message.contains("impl method `first` return type"))
        .expect("expected impl body mismatch diagnostic");

    assert!(
        diag.labels.iter().any(|label| label
            .message
            .contains("`Self.Item` resolves to `String` here")),
        "expected canonical assoc-projection label, got {:?}",
        diag
    );
}

#[test]
fn test_frontend_accepts_try_on_option_and_result_like_enums() {
    let result = analyze_source(
        r#"
            enum Option { Some(Int), None };
            enum Result { Ok(Int), Err(String) };
            let a = Some(41)? + 1;
            let b = Ok(1)? + 1;
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_coalesce_on_safe_field_and_option_enum() {
    let result = analyze_source(
        r#"
            enum Option { Some(Int), None };
            let a = Some(41) ?? 0;
            let r = #{ name = "test" };
            let b = r?.missing ?? "default";
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_reports_try_invalid_optional_flow_message_and_code() {
    let result = analyze_source("let value = 41?;");
    let diag = result
        .diagnostics
        .iter()
        .find(|diag| {
            diag.message
                .contains("`?` expects Option-like or Result-like value")
        })
        .unwrap_or_else(|| {
            panic!(
                "expected canonical invalid-try diagnostic, got {:?}",
                result.diagnostics
            )
        });
    assert_eq!(diag.kind, DiagnosticKind::Type);
    assert_eq!(diag.code, Some(ErrorCode::TypeMismatch));
}

#[test]
fn test_frontend_reports_coalesce_invalid_optional_flow_message_and_code() {
    let result = analyze_source("let value = 41 ?? 0;");
    let diag = result
        .diagnostics
        .iter()
        .find(|diag| diag.message.contains("`??` expects Option-like value"))
        .unwrap_or_else(|| {
            panic!(
                "expected canonical invalid-coalesce diagnostic, got {:?}",
                result.diagnostics
            )
        });
    assert_eq!(diag.kind, DiagnosticKind::Type);
    assert_eq!(diag.code, Some(ErrorCode::TypeMismatch));
}

#[test]
fn test_frontend_reports_safe_field_boundary_message_and_code() {
    let result = analyze_source(r#"let value = 42?.name ?? "default";"#);
    let diag = result
        .diagnostics
        .iter()
        .find(|diag| {
            diag.message
                .contains("safe field access requires a record or Option[Record]")
        })
        .unwrap_or_else(|| {
            panic!(
                "expected canonical invalid safe-field diagnostic, got {:?}",
                result.diagnostics
            )
        });
    assert_eq!(diag.kind, DiagnosticKind::Type);
    assert_eq!(diag.code, Some(ErrorCode::TypeMismatch));
}

#[test]
fn test_frontend_reports_invalid_io_read_file_path_message_and_code() {
    let result = analyze_source(
        r#"
            import std.io as io;
            let value = io.readFilePath("/tmp/file.txt");
        "#,
    );
    let diag = result
        .diagnostics
        .iter()
        .find(|diag| diag.message.contains("type mismatch"))
        .unwrap_or_else(|| {
            panic!(
                "expected canonical invalid io.readFilePath diagnostic, got {:?}",
                result.diagnostics
            )
        });
    assert_eq!(diag.kind, DiagnosticKind::Type);
    assert_eq!(diag.code, Some(ErrorCode::TypeMismatch));
}

#[test]
fn test_frontend_snippet_accepts_local_imports_against_root_dir() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(
        temp_dir.path().join("math.neve"),
        "pub fn add(x, y) = x + y;",
    )
    .unwrap();

    let source = "import math (add); let result = add(1, 2);";
    let (ast, diagnostics) = parse(source);
    assert!(
        diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        diagnostics
    );

    let analysis =
        analyze_snippet_ast(&ast, temp_dir.path()).expect("snippet analysis should succeed");
    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected snippet diagnostics: {:?}",
        analysis.diagnostics
    );
    assert!(
        analysis
            .loaded_modules
            .iter()
            .any(|entry| entry.file_path.ends_with("math.neve") && entry.diagnostics.is_empty()),
        "expected successfully loaded dependency module, got {:?}",
        analysis.loaded_modules
    );
}

#[test]
fn test_frontend_snippet_reports_loaded_module_diagnostics() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("math.neve"), "pub fn add(x, y) = ;").unwrap();

    let source = "import math (add); let result = add(1, 2);";
    let (ast, diagnostics) = parse(source);
    assert!(
        diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        diagnostics
    );

    let analysis =
        analyze_snippet_ast(&ast, temp_dir.path()).expect("snippet analysis should succeed");
    assert!(
        analysis
            .loaded_modules
            .iter()
            .flat_map(|entry| entry.diagnostics.iter())
            .any(|diag| diag.kind == DiagnosticKind::Parser),
        "expected loaded parser diagnostics, got {:?}",
        analysis.loaded_modules
    );
}

#[test]
fn test_frontend_accepts_trait_method_call_analysis() {
    let result = analyze_source(
        r#"
            trait Show { fn show(self) -> String; };
            impl Show for Int {
                fn show(self) -> String = toString(self);
            };
            let x = 1.show();
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_method_dispatch_precedence_records_method_resolution() {
    let result = analyze_source(
        r#"
            fn twice(x: Int) -> String = "fallback";
            trait Twice { fn twice(self) -> Int; };
            impl Twice for Int {
                fn twice(self) -> Int = self + self;
            };
            let value: Int = 21.twice();
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(
        result.semantics.method_resolutions.len(),
        1,
        "expected trait dispatch to record one canonical method resolution"
    );
}

#[test]
fn test_frontend_callable_target_fallback_does_not_record_method_resolution() {
    let result = analyze_source(
        r#"
            fn twice(x: Int) -> Int = x + x;
            let value = 21.twice();
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    assert!(
        result.semantics.method_resolutions.is_empty(),
        "callable-target fallback should not record a method resolution"
    );
}

#[test]
fn test_frontend_reports_dedicated_missing_method_diagnostic_when_no_fallback_exists() {
    let result = analyze_source("let value = 21.missing();");
    let diag = result
        .diagnostics
        .iter()
        .find(|diag| diag.message.contains("no method `missing` found for `Int`"))
        .unwrap_or_else(|| {
            panic!(
                "expected dedicated missing-method diagnostic, got {:?}",
                result.diagnostics
            )
        });
    assert_eq!(diag.kind, DiagnosticKind::Type);
    assert_eq!(diag.code, Some(ErrorCode::UnknownMethod));
}

#[test]
fn test_frontend_accepts_std_item_and_module_imports() {
    let result = analyze_source(
        r#"
            import std.list (len);
            import std.string as string;
            let a = len([1, 2, 3]);
            let b = string.len("abc");
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_glob_imports() {
    let result = analyze_source(
        r#"
            import std.list (*);
            let x = len([1, 2, 3]);
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_option_and_result_builtins() {
    let result = analyze_source(
        r#"
            import std.option as option;
            import std.result as result;
            let a = option.some(41)? + 1;
            let b = option.none ?? 5;
            let c = result.ok(1)? + 1;
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_path_builtins() {
    let result = analyze_source(
        r#"
            import std.path as path;
            let p = path.fromString("/tmp/file.txt");
            let d = toString(p);
            let a = path.join("a", "b");
            let b = path.parent("/tmp/file.txt") ?? "/";
            let c = path.is_absolute("/tmp/file.txt");
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_typed_path_adapters() {
    let result = analyze_source(
        r#"
            import std.path as path;
            let nested = path.joinPath(path.fromString("/tmp"), "neve.txt");
            let parent = path.parentPath(nested) ?? path.fromString("/");
            let abs = path.isAbsolutePath(parent);
            let shown = toString(parent);
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_and_fetch_builtins() {
    let result = analyze_source(
        r#"
            import std.io as io;
            import std.fetch as fetch;
            let a = io.hashString("abc");
            let b = io.exec("printf", ["neve"]).stdout;
            let c = fetch.path("Cargo.toml").hash;
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_read_file_path_bridge() {
    let result = analyze_source(
        r#"
            import std.io as io;
            import std.path as path;
            let content = io.readFilePath(path.fromString("/tmp/file.txt"));
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_current_dir_path_bridge() {
    let result = analyze_source(
        r#"
            import std.io as io;
            let cwd = io.currentDirPath();
            let shown = toString(cwd);
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_command_bridge() {
    let result = analyze_source(
        r#"
            import std.io as io;
            let cmd = io.command("printf", ["neve"]);
            let shown = toString(cmd);
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_exec_command_bridge() {
    let result = analyze_source(
        r#"
            import std.io as io;
            let result = io.execCommand(io.command("rustc", ["--version"]));
            let shown = toString(result);
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_process_success_bridge() {
    let result = analyze_source(
        r#"
            import std.io as io;
            let success = io.processSuccess(io.execCommand(io.command("rustc", ["--version"])));
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_process_stdout_bridge() {
    let result = analyze_source(
        r#"
            import std.io as io;
            let stdout = io.processStdout(io.execCommand(io.command("rustc", ["--version"])));
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_process_code_bridge() {
    let result = analyze_source(
        r#"
            import std.io as io;
            let code = io.processCode(io.execCommand(io.command("rustc", ["--version"])));
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_process_stderr_bridge() {
    let result = analyze_source(
        r#"
            import std.io as io;
            let stderr = io.processStderr(io.execCommand(io.command("rustc", ["--version"])));
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_path_exists_path_bridge() {
    let result = analyze_source(
        r#"
            import std.io as io;
            import std.path as path;
            let exists = io.pathExistsPath(path.fromString("/tmp/file.txt"));
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_is_dir_path_bridge() {
    let result = analyze_source(
        r#"
            import std.io as io;
            import std.path as path;
            let dir = io.isDirPath(path.fromString("/tmp"));
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_io_is_file_path_bridge() {
    let result = analyze_source(
        r#"
            import std.io as io;
            import std.path as path;
            let file = io.isFilePath(path.fromString("/tmp/file.txt"));
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_frontend_accepts_std_map_and_set_builtins() {
    let result = analyze_source(
        r#"
            import std.Map;
            import std.Set;
            let map = Map.insert("a", 1, Map.empty);
            let set = Set.insert(1, Set.empty);
            let value = Map.getWithDefault("a", 0, map) + Set.size(set);
        "#,
    );
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}
