//! Integration smoke tests for the real frontend/runtime paths.
//!
//! These tests intentionally avoid placeholder helpers. They cover:
//! - parse + lower + type check via `neve_frontend`
//! - runtime parity between AST compat and HIR evaluators on a supported subset
//! - explicit sentinels for currently known runtime divergence

use neve_common::Int;
use neve_diagnostic::{DiagnosticKind, ErrorCode, Severity};
use neve_eval::{EvalError, Evaluator, Value, compat::AstEvaluator};
use neve_frontend::{AnalysisResult, analyze_source};
use neve_std::{std_module_overrides, stdlib};
use std::fs;
use tempfile::TempDir;

fn int(value: i64) -> Int {
    value.into()
}

fn analyze_without_diagnostics(source: &str) -> AnalysisResult {
    let analysis = analyze_source(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected frontend diagnostics: {:?}",
        analysis.diagnostics
    );
    analysis
}

fn eval_ast(analysis: &AnalysisResult) -> Result<Value, EvalError> {
    let mut evaluator = AstEvaluator::new().with_module_overrides(std_module_overrides());
    evaluator.eval_file(&analysis.ast)
}

fn eval_hir(analysis: &AnalysisResult) -> Result<Value, EvalError> {
    let mut evaluator = Evaluator::new().with_extra_builtins(
        stdlib()
            .into_iter()
            .map(|(name, value)| (name.to_string(), value)),
    );
    evaluator
        .eval_module_with_method_resolutions(&analysis.hir, &analysis.semantics.method_resolutions)
}

fn assert_runtime_parity(source: &str, expected: Value) {
    let analysis = analyze_without_diagnostics(source);

    let ast_value = eval_ast(&analysis).expect("AST evaluator should succeed");
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");

    assert_eq!(ast_value, expected, "unexpected AST result");
    assert_eq!(hir_value, expected, "unexpected HIR result");
    assert_eq!(ast_value, hir_value, "AST/HIR runtime split detected");
}

fn assert_runtime_error_parity(source: &str, expected_fragment: &str) {
    let analysis = analyze_without_diagnostics(source);

    let ast_error = eval_ast(&analysis).expect_err("AST evaluator should fail");
    let hir_error = eval_hir(&analysis).expect_err("HIR evaluator should fail");

    match (ast_error, hir_error) {
        (EvalError::TypeError(ast), EvalError::TypeError(hir)) => {
            assert!(
                ast.contains(expected_fragment),
                "unexpected AST error: {ast}"
            );
            assert!(
                hir.contains(expected_fragment),
                "unexpected HIR error: {hir}"
            );
            assert_eq!(ast, hir, "AST/HIR error split detected");
        }
        other => panic!("expected matching type errors, got {:?}", other),
    }
}

#[test]
fn test_frontend_reports_parse_errors() {
    let analysis = analyze_source("let x =");
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Parser),
        "expected parser diagnostics, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_reports_type_errors() {
    let analysis = analyze_source("let x = 1 + true;");
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected type diagnostics, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_reports_dedicated_missing_method_for_unresolved_method_call() {
    let analysis = analyze_source("let value = 21.missing();");
    assert!(
        analysis.diagnostics.iter().any(|diag| {
            diag.kind == DiagnosticKind::Type
                && diag.code == Some(ErrorCode::UnknownMethod)
                && diag.message.contains("no method `missing` found for `Int`")
        }),
        "expected unresolved method-call target diagnostic, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_reports_user_enum_missing_patterns_in_declaration_order() {
    let analysis = analyze_source(
        "
        enum Status { Pending(Int), Running, Done(String), Failed };
        let value = match Running() {
            Running -> 1
        };
        ",
    );
    assert!(
        analysis.diagnostics.iter().any(|diag| {
            diag.kind == DiagnosticKind::Type
                && diag
                    .notes
                    .iter()
                    .any(|note| note.contains("missing patterns: Pending(_), Done(_), Failed"))
        }),
        "expected ordered missing-pattern note, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_arithmetic_runtime_parity() {
    assert_runtime_parity("let x = 1 + 2 * 3;", Value::Int(int(7)));
}

#[test]
fn test_end_to_end_pipe_runtime_parity() {
    assert_runtime_parity(
        "
        fn double(x) = x * 2;
        let x = 40 |> double |> double;
        ",
        Value::Int(int(160)),
    );
}

#[test]
fn test_end_to_end_record_field_runtime_parity() {
    assert_runtime_parity(
        "
        let config = #{ port = 40, host = \"localhost\" };
        let x = config.port;
        ",
        Value::Int(int(40)),
    );
}

#[test]
fn test_end_to_end_list_match_runtime_parity() {
    assert_runtime_parity(
        "
        fn sum_pair(xs) = match xs {
            [a, b] -> a + b,
            _ -> 0,
        };
        let x = sum_pair([1, 2]);
        ",
        Value::Int(int(3)),
    );
}

#[test]
fn test_end_to_end_enum_match_runtime_parity() {
    assert_runtime_parity(
        "
        enum Option { Some(Int), None };
        let x = Some(1);
        let y = match x {
            Some(v) -> v + 1,
            None -> 0,
        };
        ",
        Value::Int(int(2)),
    );
}

#[test]
fn test_end_to_end_recursive_fibonacci_runtime_parity() {
    assert_runtime_parity(
        "
        fn fib(n) = if n <= 1 then n else fib(n - 1) + fib(n - 2);
        let x = fib(10);
        ",
        Value::Int(int(55)),
    );
}

#[test]
fn test_end_to_end_lazy_force_runtime_parity() {
    assert_runtime_parity(
        "
        let thunk = lazy 42;
        let x = force(thunk);
        ",
        Value::Int(int(42)),
    );
}

#[test]
fn test_end_to_end_higher_order_list_runtime_parity() {
    assert_runtime_parity(
        "
        import std.list as list;
        fn inc(x) = x + 1;
        fn isEven(x) = x % 2 == 0;
        let mapped = list.map(inc, [1, 2, 3]);
        let x = list.filter(isEven, mapped);
        ",
        Value::List(std::rc::Rc::new(vec![
            Value::Int(int(2)),
            Value::Int(int(4)),
        ])),
    );
}

#[test]
fn test_end_to_end_block_let_pattern_runtime_parity() {
    assert_runtime_parity(
        "
        let result = {
            let (x, y) = (1, 2);
            x + y
        };
        ",
        Value::Int(int(3)),
    );
}

#[test]
fn test_end_to_end_or_pattern_runtime_parity() {
    assert_runtime_parity(
        "
        let x = match (1, 2) {
            (0, v) | (1, v) -> v,
            _ -> 0,
        };
        ",
        Value::Int(int(2)),
    );
}

#[test]
fn test_end_to_end_binding_pattern_runtime_parity() {
    assert_runtime_parity(
        "
        let x = match 42 {
            n @ 42 -> n + 1,
            _ -> 0,
        };
        ",
        Value::Int(int(43)),
    );
}

#[test]
fn test_end_to_end_list_rest_pattern_runtime_parity() {
    assert_runtime_parity(
        "
        let x = match [1, 2, 3, 4] {
            [first, ..middle, last] -> match middle {
                [a, b] -> first + a + b + last,
                _ -> 0,
            },
            _ -> 0,
        };
        ",
        Value::Int(int(10)),
    );
}

#[test]
fn test_end_to_end_try_runtime_parity() {
    assert_runtime_parity(
        "
        enum Option { Some(Int), None };
        let x = Some(41)? + 1;
        ",
        Value::Int(int(42)),
    );
}

#[test]
fn test_end_to_end_coalesce_runtime_parity() {
    assert_runtime_parity(
        "
        enum Option { Some(Int), None };
        let x = Some(41) ?? 0;
        ",
        Value::Int(int(41)),
    );
}

#[test]
fn test_end_to_end_safe_field_coalesce_runtime_parity() {
    assert_runtime_parity(
        "
        let r = #{ name = \"test\" };
        let x = r?.missing ?? \"default\";
        ",
        Value::String("default".to_string().into()),
    );
}

#[test]
fn test_end_to_end_safe_field_option_record_runtime_parity() {
    assert_runtime_parity(
        "
        import std.option as option;
        let x = option.some(#{ name = \"test\" })?.name ?? \"default\";
        ",
        Value::String("test".to_string().into()),
    );
}

#[test]
fn test_frontend_reports_safe_field_non_record_callsite_error() {
    let analysis = analyze_source(
        "
        let readName = fn(config) config?.name ?? \"default\";
        let value = readName(42);
        ",
    );
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected type diagnostics, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_reports_try_on_non_optional_error() {
    let analysis = analyze_source("let value = 41?;");
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected type diagnostics, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_reports_coalesce_on_non_optional_error() {
    let analysis = analyze_source("let value = 41 ?? 0;");
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diag| diag.kind == DiagnosticKind::Type),
        "expected type diagnostics, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_reports_subset_shadowing_unreachable_pattern_warning() {
    let analysis = analyze_source(
        "
        import std.option as option;
        let value = match option.some(1) {
            Some(_) -> 1,
            Some(inner) -> inner,
            None -> 0
        };
        ",
    );
    assert!(
        analysis.diagnostics.iter().any(|diag| {
            diag.kind == DiagnosticKind::Type
                && diag.severity == Severity::Warning
                && diag.message.contains("unreachable pattern")
        }),
        "expected unreachable-pattern warning, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_try_error_runtime_parity() {
    assert_runtime_error_parity(
        "
        enum Result { Ok(Int), Err(String) };
        let x = Err(\"boom\")?;
        ",
        "boom",
    );
}

#[test]
fn test_end_to_end_method_call_fallback_runtime_parity() {
    assert_runtime_parity(
        "
        fn twice(x: Int) -> Int = x + x;
        let y = 21.twice();
        ",
        Value::Int(int(42)),
    );
}

#[test]
fn test_end_to_end_trait_dispatch_precedence_over_callable_target_fallback_runtime_parity() {
    assert_runtime_parity(
        "
        fn twice(x: Int) -> Int = x + 1000;
        trait Twice { fn twice(self) -> Int; };
        impl Twice for Int {
            fn twice(self) -> Int = self + self;
        };
        let x = 21.twice();
        ",
        Value::Int(int(42)),
    );
}

#[test]
fn test_end_to_end_trait_method_runtime_parity() {
    assert_runtime_parity(
        "
        trait Twice { fn twice(self) -> Int; };
        impl Twice for Int {
            fn twice(self) -> Int = self + self;
        };
        let x = 21.twice();
        ",
        Value::Int(int(42)),
    );
}

#[test]
fn test_end_to_end_trait_method_assoc_return_runtime_parity() {
    assert_runtime_parity(
        "
        trait Iterator { type Item; fn first(self) -> Self.Item; };
        impl Iterator for Int {
            type Item = String;
            fn first(self) -> Self.Item = toString(self);
        };
        let x = 1.first();
        ",
        Value::String("1".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_item_import_runtime_parity() {
    assert_runtime_parity(
        "
        import std.list (len);
        let x = len([1, 2, 3]);
        ",
        Value::Int(int(3)),
    );
}

#[test]
fn test_end_to_end_std_module_import_runtime_parity() {
    assert_runtime_parity(
        r#"
        import std.string as string;
        let x = string.len("abcd");
        "#,
        Value::Int(int(4)),
    );
}

#[test]
fn test_end_to_end_std_glob_import_runtime_parity() {
    assert_runtime_parity(
        "
        import std.list (*);
        let x = len([1, 2, 3, 4]);
        ",
        Value::Int(int(4)),
    );
}

#[test]
fn test_end_to_end_std_option_builtin_runtime_parity() {
    assert_runtime_parity(
        "
        import std.option as option;
        let a = option.some(41)? + 1;
        let b = option.none ?? 5;
        let x = a + b;
        ",
        Value::Int(int(47)),
    );
}

#[test]
fn test_end_to_end_builtin_option_match_runtime_parity() {
    assert_runtime_parity(
        "
        import std.option as option;
        let x = match option.some(41) {
            Some(value) -> value,
            None -> 0
        };
        ",
        Value::Int(int(41)),
    );
}

#[test]
fn test_end_to_end_std_result_builtin_runtime_parity() {
    assert_runtime_parity(
        "
        import std.result as result;
        let a = result.ok(41)? + 1;
        let err = result.unwrap_err(result.err(\"boom\"));
        let x = if err == \"boom\" then a else 0;
        ",
        Value::Int(int(42)),
    );
}

#[test]
fn test_end_to_end_builtin_result_match_runtime_parity() {
    assert_runtime_parity(
        "
        import std.result as result;
        let x = match result.err(\"boom\") {
            Ok(value) -> value,
            Err(message) -> if message == \"boom\" then 1 else 0
        };
        ",
        Value::Int(int(1)),
    );
}

#[test]
fn test_end_to_end_std_path_builtin_runtime_parity() {
    assert_runtime_parity(
        "
        import std.path as path;
        let parent = path.parent(\"/tmp/file.txt\") ?? \"/\";
        let x = if path.is_absolute(\"/tmp/file.txt\") then parent else \"nope\";
        ",
        Value::String("/tmp".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_path_from_string_runtime_parity() {
    assert_runtime_parity(
        "
        import std.path as path;
        let p = path.fromString(\"/tmp/file.txt\");
        let x = if typeOf(p) == \"Path\" then toString(p) else \"nope\";
        ",
        Value::String("/tmp/file.txt".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_typed_path_adapter_runtime_parity() {
    assert_runtime_parity(
        "
        import std.path as path;
        let nested = path.joinPath(path.fromString(\"/tmp\"), \"neve.txt\");
        let parent = path.parentPath(nested) ?? path.fromString(\"/\");
        let x = if path.isAbsolutePath(parent) then toString(parent) else \"nope\";
        ",
        Value::String("/tmp".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_io_builtin_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let x = io.hashString(\"abc\");
        ",
        Value::String(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                .to_string()
                .into(),
        ),
    );
}

#[test]
fn test_end_to_end_std_io_exec_matches_canonical_process_projection() {
    assert_runtime_parity(
        "
        import std.io as io;
        let legacy = io.exec(\"rustc\", [\"--version\"]);
        let canonical = io.execCommand(io.command(\"rustc\", [\"--version\"]));
        let same =
            legacy.success == io.processSuccess(canonical) &&
            legacy.stdout == io.processStdout(canonical) &&
            legacy.code == io.processCode(canonical) &&
            legacy.stderr == io.processStderr(canonical);
        ",
        Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_std_io_read_file_path_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("io-read-path.neve.txt");
    fs::write(&file_path, "hello-path").unwrap();
    let escaped = file_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        import std.path as path;
        let x = io.readFilePath(path.fromString(\"{escaped}\"));
        "
    );

    assert_runtime_parity(&source, Value::String("hello-path".to_string().into()));
}

#[test]
fn test_end_to_end_std_io_current_dir_path_runtime_parity() {
    let expected = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert_runtime_parity(
        "
        import std.io as io;
        let cwd = io.currentDirPath();
        let x = if typeOf(cwd) == \"Path\" then toString(cwd) else \"nope\";
        ",
        Value::String(expected.into()),
    );
}

#[test]
fn test_end_to_end_std_io_command_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let cmd = io.command(\"printf\", [\"neve\"]);
        let x = if typeOf(cmd) == \"Command\" then toString(cmd) else \"nope\";
        ",
        Value::String("<command:printf 1 arg(s)>".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_io_exec_command_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let cmd = io.command(\"rustc\", [\"--version\"]);
        let result = io.execCommand(cmd);
        let x = if typeOf(result) == \"ProcessResult\" then toString(result) else \"nope\";
        ",
        Value::String("<process-result:0 ok>".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_io_process_success_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let result = io.execCommand(io.command(\"rustc\", [\"--version\"]));
        let x = io.processSuccess(result);
        ",
        Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_std_io_process_stdout_runtime_parity() {
    let analysis = analyze_without_diagnostics(
        "
        import std.io as io;
        let result = io.execCommand(io.command(\"rustc\", [\"--version\"]));
        let x = io.processStdout(result);
        ",
    );

    let ast_value = eval_ast(&analysis).expect("AST evaluator should succeed");
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");

    match (&ast_value, &hir_value) {
        (Value::String(ast), Value::String(hir)) => {
            assert!(ast.contains("rustc"), "AST stdout should contain rustc");
            assert!(hir.contains("rustc"), "HIR stdout should contain rustc");
            assert_eq!(ast, hir, "AST/HIR stdout split detected");
        }
        other => panic!("expected matching stdout strings, got {:?}", other),
    }
}

#[test]
fn test_end_to_end_std_io_process_code_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let result = io.execCommand(io.command(\"rustc\", [\"--version\"]));
        let x = io.processCode(result);
        ",
        Value::Int(0.into()),
    );
}

#[test]
fn test_end_to_end_std_io_process_stderr_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let result = io.execCommand(io.command(\"rustc\", [\"--version\"]));
        let x = io.processStderr(result);
        ",
        Value::String(String::new().into()),
    );
}

#[test]
fn test_end_to_end_std_io_path_exists_path_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("exists-path.neve.txt");
    std::fs::write(&file_path, "exists").unwrap();
    let escaped = file_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        import std.path as path;
        let x = io.pathExistsPath(path.fromString(\"{escaped}\"));
        "
    );

    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_is_dir_path_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("dir-path");
    std::fs::create_dir_all(&dir_path).unwrap();
    let escaped = dir_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        import std.path as path;
        let x = io.isDirPath(path.fromString(\"{escaped}\"));
        "
    );

    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_is_file_path_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("file-path.neve.txt");
    std::fs::write(&file_path, "file").unwrap();
    let escaped = file_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        import std.path as path;
        let x = io.isFilePath(path.fromString(\"{escaped}\"));
        "
    );

    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_map_and_set_builtin_runtime_parity() {
    assert_runtime_parity(
        "
        import std.Map;
        import std.Set;
        let map = Map.insert(\"a\", 41, Map.empty);
        let set = Set.insert(1, Set.empty);
        let x = Map.getWithDefault(\"a\", 0, map) + Set.size(set);
        ",
        Value::Int(int(42)),
    );
}
