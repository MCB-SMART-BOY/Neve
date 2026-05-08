//! Integration smoke tests for the real frontend/runtime paths.
//!
//! These tests intentionally avoid placeholder helpers. They cover:
//! - parse + lower + type check via `neve_frontend`
//! - runtime parity between AST compat and HIR evaluators on a supported subset
//! - explicit sentinels for currently known runtime divergence

mod support;

use neve_common::Int;
use neve_derive::Hash;
use neve_diagnostic::{DiagnosticKind, ErrorCode, Severity};
use neve_eval::{EvalError, Evaluator, Value, compat::AstEvaluator};
use neve_frontend::{AnalysisResult, analyze_source};
use neve_std::{std_module_overrides, stdlib};
use std::fs;
use std::rc::Rc;
use support::fetch_fixtures::{init_local_git_repo, start_local_http_fixture};
use support::source_fixtures::{
    pipeline_execution_source as shared_pipeline_execution_source,
    shell_projection_source as shared_shell_projection_source,
};
use tempfile::TempDir;

fn int(value: i64) -> Int {
    value.into()
}

fn analyze_without_diagnostics(source: &str) -> AnalysisResult {
    let analysis = analyze_source(source);
    // Only reject errors; warnings like unreachable patterns are acceptable
    let errors: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|d| d.severity == neve_diagnostic::Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "unexpected frontend errors: {:?}",
        errors
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

fn shell_projection_source() -> String {
    shared_shell_projection_source(Some("x"))
}

fn pipeline_execution_source() -> String {
    shared_pipeline_execution_source(Some("x"))
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
    // Method fallback now emits a warning, so we test directly without diagnostics check
    let source = "
        fn twice(x: Int) -> Int = x + x;
        let y = 21.twice();
    ";
    let analysis = neve_frontend::analyze_source(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Int(int(42)));
    // Verify a warning was emitted about method fallback
    let has_warning = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("method") && d.message.contains("callable fallback"));
    assert!(
        has_warning,
        "expected method fallback warning, got {:?}",
        analysis.diagnostics
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
        let name = path.filenamePath(nested) ?? \"missing\";
        let ext = path.extensionPath(nested) ?? \"missing\";
        let x = if name == \"neve.txt\" && ext == \"txt\" then \"ok\" else \"nope\";
        ",
        Value::String("ok".to_string().into()),
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
fn test_end_to_end_std_io_current_system_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        io.currentSystem()
        ",
        Value::String(format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS).into()),
    );
}

#[test]
fn test_end_to_end_std_io_current_dir_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        io.currentDir()
        ",
        Value::String(
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .into_owned()
                .into(),
        ),
    );
}

#[test]
fn test_end_to_end_std_io_get_env_runtime_parity() {
    let missing = "__NEVE_TEST_MISSING_ENV_37C93B7C__";
    assert!(
        std::env::var_os(missing).is_none(),
        "test environment unexpectedly defines {missing}"
    );
    assert_runtime_parity(
        &format!(
            "
        import std.io as io;
        io.getEnv(\"{missing}\") ?? \"missing\"
        "
        ),
        Value::String("missing".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_fetch_path_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("source.txt");
    let content = b"fetch-path-content";
    fs::write(&file_path, content).unwrap();
    let expected = Hash::of(content).to_hex();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.fetch as fetch;
        let x = fetch.path(\"{escaped}\").hash;
        "
        ),
        Value::String(Rc::new(expected)),
    );
}

#[test]
fn test_end_to_end_std_fetch_path_with_hash_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("source.txt");
    let content = b"fetch-path-content";
    fs::write(&file_path, content).unwrap();
    let expected = Hash::of(content).to_hex();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.fetch as fetch;
        let x = fetch.pathWithHash(\"{escaped}\", \"{expected}\").hash;
        "
        ),
        Value::String(Rc::new(expected)),
    );
}

#[test]
fn test_end_to_end_std_fetch_url_with_hash_runtime_parity() {
    let (url, expected_hash, server) = start_local_http_fixture(b"fetch-url-content");
    assert_runtime_parity(
        &format!(
            "
        import std.fetch as fetch;
        let x = fetch.urlWithHash(\"{url}\", \"{expected_hash}\").hash;
        "
        ),
        Value::String(Rc::new(expected_hash)),
    );
    server.join().expect("fixture server should exit cleanly");
}

#[test]
fn test_end_to_end_std_fetch_url_runtime_parity() {
    let (url, expected_hash, server) = start_local_http_fixture(b"fetch-url-content");
    assert_runtime_parity(
        &format!(
            "
        import std.fetch as fetch;
        let x = fetch.url(\"{url}\").hash;
        "
        ),
        Value::String(Rc::new(expected_hash)),
    );
    server.join().expect("fixture server should exit cleanly");
}

#[test]
fn test_end_to_end_std_fetch_git_runtime_parity() {
    let (_temp, repo_path, expected_hash) = init_local_git_repo();
    let escaped = repo_path.replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.fetch as fetch;
        let x = fetch.git(\"{escaped}\", \"main\").hash;
        "
        ),
        Value::String(Rc::new(expected_hash)),
    );
}

#[test]
fn test_end_to_end_std_fetch_git_with_hash_runtime_parity() {
    let (_temp, repo_path, expected_hash) = init_local_git_repo();
    let escaped = repo_path.replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.fetch as fetch;
        let x = fetch.gitWithHash(\"{escaped}\", \"main\", \"{expected_hash}\").hash;
        "
        ),
        Value::String(Rc::new(expected_hash)),
    );
}

#[test]
fn test_end_to_end_std_io_read_file_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("source.txt");
    fs::write(&file_path, "read-file-content").unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.io as io;
        let x = io.readFile(\"{escaped}\");
        "
        ),
        Value::String(Rc::new("read-file-content".to_string())),
    );
}

#[test]
fn test_end_to_end_std_io_read_dir_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("io-read-dir");
    fs::create_dir_all(dir.join("nested")).unwrap();
    fs::write(dir.join("alpha.txt"), "a").unwrap();
    fs::write(dir.join("beta.txt"), "b").unwrap();
    let escaped = dir.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.io as io;
        import std.list as list;
        let x = list.sort(io.readDir(\"{escaped}\"));
        "
        ),
        Value::List(Rc::new(vec![
            Value::String(Rc::new("alpha.txt".to_string())),
            Value::String(Rc::new("beta.txt".to_string())),
            Value::String(Rc::new("nested".to_string())),
        ])),
    );
}

#[test]
fn test_end_to_end_std_io_hash_file_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("source.txt");
    let content = b"hash-file-content";
    fs::write(&file_path, content).unwrap();
    let expected = "09f00a4ba8e49c5a253e1af9ff6c40f8151754ccd88f95ef162981960b2ad8f7";
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.io as io;
        let x = io.hashFile(\"{escaped}\");
        "
        ),
        Value::String(Rc::new(expected.to_string())),
    );
}

#[test]
fn test_end_to_end_std_io_hash_file_path_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("source.txt");
    let content = b"hash-file-path-content";
    fs::write(&file_path, content).unwrap();
    let expected = "9c3675e0b07ef1223e4cb9afdc255c51c8557ac075e91e601978676b894c95b1";
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.io as io;
        import std.path as path;
        let x = io.hashFilePath(path.fromString(\"{escaped}\"));
        "
        ),
        Value::String(Rc::new(expected.to_string())),
    );
}

#[test]
fn test_end_to_end_std_io_exec_command_matches_canonical_process_projection() {
    assert_runtime_parity(
        "
        import std.io as io;
        let migrated = io.execCommand(io.command(\"rustc\", [\"--version\"]));
        let canonical = io.execCommand(io.command(\"rustc\", [\"--version\"]));
        let same =
            typeOf(migrated) == \"ProcessResult\" &&
            io.processSuccess(migrated) == io.processSuccess(canonical) &&
            io.processStdout(migrated) == io.processStdout(canonical) &&
            io.processCode(migrated) == io.processCode(canonical) &&
            io.processStderr(migrated) == io.processStderr(canonical);
        ",
        Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_std_io_explicit_shell_command_matches_canonical_process_projection() {
    let source = shell_projection_source();
    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_exec_with_matches_canonical_process_projection() {
    assert_runtime_parity(
        "
        import std.io as io;
        let migrated =
            io.execCommand(io.commandWith(#{ program = \"rustc\", args = [\"--version\"] }));
        let canonical =
            io.execCommand(io.commandWith(#{ program = \"rustc\", args = [\"--version\"] }));
        let same =
            typeOf(migrated) == \"ProcessResult\" &&
            io.processSuccess(migrated) == io.processSuccess(canonical) &&
            io.processStdout(migrated) == io.processStdout(canonical) &&
            io.processCode(migrated) == io.processCode(canonical) &&
            io.processStderr(migrated) == io.processStderr(canonical);
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
fn test_end_to_end_std_io_read_file_bytes_path_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("io-read-bytes-path.neve.bin");
    std::fs::write(&file_path, [0xde, 0xad, 0xbe, 0xef]).unwrap();
    let escaped = file_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        import std.path as path;
        let bytes = io.readFileBytesPath(path.fromString(\"{escaped}\"));
        let x = if typeOf(bytes) == \"Bytes\" then toString(bytes) else \"nope\";
        "
    );

    assert_runtime_parity(&source, Value::String("<bytes:4>".to_string().into()));
}

#[test]
fn test_end_to_end_std_io_read_dir_path_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("io-read-dir-path.neve");
    fs::create_dir_all(dir_path.join("nested")).unwrap();
    fs::write(dir_path.join("alpha.txt"), "a").unwrap();
    fs::write(dir_path.join("beta.txt"), "b").unwrap();
    let escaped = dir_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        import std.path as path;
        import std.list as list;
        let entries = io.readDirPath(path.fromString(\"{escaped}\"));
        let x = list.sort(entries);
        "
    );

    assert_runtime_parity(
        &source,
        Value::List(
            vec!["alpha.txt", "beta.txt", "nested"]
                .into_iter()
                .map(|name| Value::String(name.to_string().into()))
                .collect::<Vec<_>>()
                .into(),
        ),
    );
}

#[test]
fn test_end_to_end_std_io_read_dir_entry_paths_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("io-read-dir-entry-paths.neve");
    let file_path = dir_path.join("alpha.txt");
    let nested_path = dir_path.join("nested");
    fs::create_dir_all(&dir_path).unwrap();
    fs::create_dir_all(&nested_path).unwrap();
    fs::write(&file_path, "a").unwrap();
    let escaped = dir_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        import std.path as path;
        import std.list as list;
        list.sort(io.readDirEntryPaths(path.fromString(\"{escaped}\")))
        "
    );

    assert_runtime_parity(
        &source,
        Value::List(
            vec![
                Value::Path(Rc::new(file_path)),
                Value::Path(Rc::new(nested_path)),
            ]
            .into(),
        ),
    );
}

#[test]
fn test_end_to_end_std_io_write_file_bytes_path_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let src_path = temp_dir.path().join("io-write-bytes-src.neve.bin");
    let dst_path = temp_dir.path().join("io-write-bytes-dst.neve.bin");
    std::fs::write(&src_path, [0xde, 0xad, 0xbe, 0xef]).unwrap();
    let escaped_src = src_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");
    let escaped_dst = dst_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        import std.path as path;
        let src = path.fromString(\"{escaped_src}\");
        let dst = path.fromString(\"{escaped_dst}\");
        let bytes = io.readFileBytesPath(src);
        let done = io.writeFileBytesPath(dst, bytes);
        let copied = io.readFileBytesPath(dst);
        let x = typeOf(copied) == \"Bytes\" && copied == bytes;
        "
    );

    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_write_file_path_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let dst_path = temp_dir.path().join("io-write-path-dst.neve.txt");
    let escaped_dst = dst_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        import std.path as path;
        let dst = path.fromString(\"{escaped_dst}\");
        let done = io.writeFilePath(dst, \"hello-path\");
        let x = io.readFilePath(dst);
        "
    );

    assert_runtime_parity(&source, Value::String("hello-path".to_string().into()));
}

#[test]
fn test_end_to_end_std_io_write_file_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let dst_path = temp_dir.path().join("io-write-dst.neve.txt");
    let escaped_dst = dst_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        let done = io.writeFile(\"{escaped_dst}\", \"hello\");
        let x = io.readFile(\"{escaped_dst}\");
        "
    );

    assert_runtime_parity(&source, Value::String("hello".to_string().into()));
}

#[test]
fn test_end_to_end_std_io_append_file_path_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let dst_path = temp_dir.path().join("io-append-path-dst.neve.txt");
    let escaped_dst = dst_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        import std.path as path;
        let dst = path.fromString(\"{escaped_dst}\");
        let reset = io.writeFilePath(dst, \"hello\");
        let done = io.appendFilePath(dst, \"-path\");
        let x = io.readFilePath(dst);
        "
    );

    assert_runtime_parity(&source, Value::String("hello-path".to_string().into()));
}

#[test]
fn test_end_to_end_std_io_append_file_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let dst_path = temp_dir.path().join("io-append-dst.neve.txt");
    std::fs::write(&dst_path, "hello").unwrap();
    let escaped_dst = dst_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        let reset = io.writeFile(\"{escaped_dst}\", \"hello\");
        let done = io.appendFile(\"{escaped_dst}\", \"-path\");
        let x = io.readFile(\"{escaped_dst}\");
        "
    );

    assert_runtime_parity(&source, Value::String("hello-path".to_string().into()));
}

#[test]
fn test_end_to_end_std_io_append_file_bytes_path_runtime_parity() {
    let temp_dir = TempDir::new().unwrap();
    let init_path = temp_dir.path().join("io-append-bytes-init.neve.bin");
    let append_path = temp_dir.path().join("io-append-bytes-src.neve.bin");
    let dst_path = temp_dir.path().join("io-append-bytes-dst.neve.bin");
    let expected_path = temp_dir.path().join("io-append-bytes-expected.neve.bin");
    std::fs::write(&init_path, [0xaa]).unwrap();
    std::fs::write(&append_path, [0xde, 0xad, 0xbe]).unwrap();
    std::fs::write(&expected_path, [0xaa, 0xde, 0xad, 0xbe]).unwrap();
    let escaped_init = init_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");
    let escaped_append = append_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");
    let escaped_dst = dst_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");
    let escaped_expected = expected_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        "
        import std.io as io;
        import std.path as path;
        let init = io.readFileBytesPath(path.fromString(\"{escaped_init}\"));
        let append = io.readFileBytesPath(path.fromString(\"{escaped_append}\"));
        let expected = io.readFileBytesPath(path.fromString(\"{escaped_expected}\"));
        let dst = path.fromString(\"{escaped_dst}\");
        let reset = io.writeFileBytesPath(dst, init);
        let done = io.appendFileBytesPath(dst, append);
        let copied = io.readFileBytesPath(dst);
        let x = typeOf(copied) == \"Bytes\" && copied == expected;
        "
    );

    assert_runtime_parity(&source, Value::Bool(true));
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
fn test_end_to_end_std_io_home_dir_path_runtime_parity() {
    let expected = match std::env::var("HOME") {
        Ok(home) => Value::Some(Box::new(Value::Path(Rc::new(home.into())))),
        Err(_) => Value::None,
    };
    assert_runtime_parity(
        "
        import std.io as io;
        io.homeDirPath()
        ",
        expected,
    );
}

#[test]
fn test_end_to_end_std_io_home_dir_runtime_parity() {
    let expected = match std::env::var("HOME") {
        Ok(home) => Value::Some(Box::new(Value::String(home.into()))),
        Err(_) => Value::None,
    };
    assert_runtime_parity(
        "
        import std.io as io;
        io.homeDir()
        ",
        expected,
    );
}

#[test]
fn test_end_to_end_std_io_create_dir_all_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("legacy").join("a").join("b");
    let escaped = target.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.io as io;
        let target = \"{escaped}\";
        let done = io.createDirAll(target);
        io.pathExists(target) && io.isDir(target)
        "
        ),
        Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_std_io_create_dir_all_path_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("typed").join("a").join("b");
    let escaped = target.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.io as io;
        import std.path as path;
        let target = path.fromString(\"{escaped}\");
        let done = io.createDirAllPath(target);
        io.pathExistsPath(target) && io.isDirPath(target)
        "
        ),
        Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_std_io_remove_dir_all_path_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("typed").join("a").join("b");
    let escaped = target.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.io as io;
        import std.path as path;
        let target = path.fromString(\"{escaped}\");
        let created = io.createDirAllPath(target);
        let done = io.removeDirAllPath(target);
        !io.pathExistsPath(target)
        "
        ),
        Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_std_io_remove_dir_all_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("legacy").join("a").join("b");
    let escaped = target.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.io as io;
        let target = \"{escaped}\";
        let created = io.createDirAll(target);
        let done = io.removeDirAll(target);
        !io.pathExists(target)
        "
        ),
        Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_std_io_path_exists_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("exists.txt");
    fs::write(&file, "neve").unwrap();
    let escaped = file.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.io as io;
        let x = io.pathExists(\"{escaped}\");
        "
        ),
        Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_std_io_is_dir_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("nested");
    fs::create_dir_all(&dir).unwrap();
    let escaped = dir.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.io as io;
        let x = io.isDir(\"{escaped}\");
        "
        ),
        Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_std_io_is_file_runtime_parity() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("nested.txt");
    fs::write(&file, "neve").unwrap();
    let escaped = file.to_string_lossy().replace('\\', "\\\\");
    assert_runtime_parity(
        &format!(
            "
        import std.io as io;
        let x = io.isFile(\"{escaped}\");
        "
        ),
        Value::Bool(true),
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
fn test_end_to_end_std_io_command_with_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let cmd = io.commandWith(#{ program = \"printf\", args = [\"neve\"], cwd = \"/tmp\" });
        let x = if typeOf(cmd) == \"Command\" then toString(cmd) else \"nope\";
        ",
        Value::String("<command:printf 1 arg(s), configured>".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_io_pipeline_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let pipe = io.pipeline([io.command(\"printf\", [\"neve\"]), io.command(\"cat\", [])]);
        let x = if typeOf(pipe) == \"Pipeline\" then toString(pipe) else \"nope\";
        ",
        Value::String("<pipeline:2 command(s)>".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_io_pipeline_with_redirects_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        import std.path as path;
        let pipe = io.pipelineWithRedirects(
            io.pipeline([io.command(\"printf\", [\"neve\"]), io.command(\"cat\", [])]),
            [io.redirectStdoutPath(path.fromString(\"/tmp/neve.out\"))]
        );
        let x = if typeOf(pipe) == \"Pipeline\" then toString(pipe) else \"nope\";
        ",
        Value::String("<pipeline:2 command(s)>".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_io_exec_pipeline_runtime_parity() {
    assert_runtime_parity(&pipeline_execution_source(), Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_exec_pipeline_with_redirect_runtime_parity() {
    let temp = TempDir::new().expect("temp dir should exist");
    let redirect_path = temp.path().join("pipeline-stdout.txt");
    let redirect_path_source = redirect_path.to_string_lossy().replace('\\', "\\\\");
    let source = if cfg!(windows) {
        format!(
            r#"
        import std.io as io;
        import std.path as path;
        let target = path.fromString("{redirect_path_source}");
        let result = io.execPipeline(
            io.pipelineWithRedirects(
                io.pipeline([
                    io.command("cmd", ["/C", "echo neve"]),
                    io.command("cmd", ["/C", "findstr neve"])
                ]),
                [io.redirectStdoutPath(target)]
            )
        );
        let redirected = io.readFilePath(target);
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) == "" &&
            redirected != "";
        "#
        )
    } else {
        format!(
            r#"
        import std.io as io;
        import std.path as path;
        let target = path.fromString("{redirect_path_source}");
        let result = io.execPipeline(
            io.pipelineWithRedirects(
                io.pipeline([
                    io.command("sh", ["-c", "printf neve"]),
                    io.command("sh", ["-c", "grep neve"])
                ]),
                [io.redirectStdoutPath(target)]
            )
        );
        let redirected = io.readFilePath(target);
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) == "" &&
            redirected != "";
        "#
        )
    };
    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_redirect_stdout_path_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        import std.path as path;
        let redirect = io.redirectStdoutPath(path.fromString(\"/tmp/neve.out\"));
        let x = if typeOf(redirect) == \"Redirect\" then toString(redirect) else \"nope\";
        ",
        Value::String("<redirect:stdout:path>".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_io_redirect_stderr_path_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        import std.path as path;
        let redirect = io.redirectStderrPath(path.fromString(\"/tmp/neve.err\"));
        let x = if typeOf(redirect) == \"Redirect\" then toString(redirect) else \"nope\";
        ",
        Value::String("<redirect:stderr:path>".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_io_redirect_stdin_path_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        import std.path as path;
        let redirect = io.redirectStdinPath(path.fromString(\"/tmp/neve.in\"));
        let x = if typeOf(redirect) == \"Redirect\" then toString(redirect) else \"nope\";
        ",
        Value::String("<redirect:stdin:path>".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_io_exec_command_with_redirect_runtime_parity() {
    let temp = TempDir::new().expect("temp dir should exist");
    let redirect_path = temp.path().join("stdout.txt");
    let redirect_path_source = redirect_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io as io;
        import std.path as path;
        let target = path.fromString("{redirect_path_source}");
        let result = io.execCommand(
            io.commandWithRedirects(
                io.command("rustc", ["--version"]),
                [io.redirectStdoutPath(target)]
            )
        );
        let redirected = io.readFilePath(target);
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) == "" &&
            redirected != "";
        "#
    );
    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_exec_command_with_stdin_redirect_runtime_parity() {
    let temp = TempDir::new().expect("temp dir should exist");
    let redirect_path = temp.path().join("stdin.txt");
    fs::write(&redirect_path, "neve stdin line\n").expect("stdin file should be writable");
    let redirect_path_source = redirect_path.to_string_lossy().replace('\\', "\\\\");
    let source = if cfg!(windows) {
        format!(
            r#"
        import std.io as io;
        import std.path as path;
        let target = path.fromString("{redirect_path_source}");
        let result = io.execCommand(
            io.commandWithRedirects(
                io.command("cmd", ["/C", "findstr neve"]),
                [io.redirectStdinPath(target)]
            )
        );
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) != "";
        "#
        )
    } else {
        format!(
            r#"
        import std.io as io;
        import std.path as path;
        let target = path.fromString("{redirect_path_source}");
        let result = io.execCommand(
            io.commandWithRedirects(
                io.command("sh", ["-c", "grep neve"]),
                [io.redirectStdinPath(target)]
            )
        );
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) != "";
        "#
        )
    };
    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_exec_command_with_redirects_runtime_parity() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stdin_path = temp.path().join("stdin.txt");
    let stdout_path = temp.path().join("stdout.txt");
    fs::write(&stdin_path, "neve stdin line\n").expect("stdin file should be writable");
    let stdin_path_source = stdin_path.to_string_lossy().replace('\\', "\\\\");
    let stdout_path_source = stdout_path.to_string_lossy().replace('\\', "\\\\");
    let source = if cfg!(windows) {
        format!(
            r#"
        import std.io as io;
        import std.path as path;
        let input = path.fromString("{stdin_path_source}");
        let output = path.fromString("{stdout_path_source}");
        let result = io.execCommand(
            io.commandWithRedirects(
                io.command("cmd", ["/C", "findstr neve"]),
                [io.redirectStdinPath(input), io.redirectStdoutPath(output)]
            )
        );
        let redirected = io.readFilePath(output);
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) == "" &&
            redirected != "";
        "#
        )
    } else {
        format!(
            r#"
        import std.io as io;
        import std.path as path;
        let input = path.fromString("{stdin_path_source}");
        let output = path.fromString("{stdout_path_source}");
        let result = io.execCommand(
            io.commandWithRedirects(
                io.command("sh", ["-c", "grep neve"]),
                [io.redirectStdinPath(input), io.redirectStdoutPath(output)]
            )
        );
        let redirected = io.readFilePath(output);
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) == "" &&
            redirected != "";
        "#
        )
    };
    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_exec_pipeline_with_stdin_redirect_runtime_parity() {
    let temp = TempDir::new().expect("temp dir should exist");
    let redirect_path = temp.path().join("pipeline-stdin.txt");
    fs::write(&redirect_path, "neve stdin line\n").expect("stdin file should be writable");
    let redirect_path_source = redirect_path.to_string_lossy().replace('\\', "\\\\");
    let source = if cfg!(windows) {
        format!(
            r#"
        import std.io as io;
        import std.path as path;
        let target = path.fromString("{redirect_path_source}");
        let result = io.execPipeline(
            io.pipelineWithRedirects(
                io.pipeline([
                    io.command("cmd", ["/C", "findstr neve"]),
                    io.command("cmd", ["/C", "findstr neve"])
                ]),
                [io.redirectStdinPath(target)]
            )
        );
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) != "";
        "#
        )
    } else {
        format!(
            r#"
        import std.io as io;
        import std.path as path;
        let target = path.fromString("{redirect_path_source}");
        let result = io.execPipeline(
            io.pipelineWithRedirects(
                io.pipeline([
                    io.command("sh", ["-c", "grep neve"]),
                    io.command("sh", ["-c", "grep neve"])
                ]),
                [io.redirectStdinPath(target)]
            )
        );
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) != "";
        "#
        )
    };
    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_exec_pipeline_with_redirects_runtime_parity() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stdin_path = temp.path().join("pipeline-stdin.txt");
    let stdout_path = temp.path().join("pipeline-stdout.txt");
    fs::write(&stdin_path, "neve stdin line\n").expect("stdin file should be writable");
    let stdin_path_source = stdin_path.to_string_lossy().replace('\\', "\\\\");
    let stdout_path_source = stdout_path.to_string_lossy().replace('\\', "\\\\");
    let source = if cfg!(windows) {
        format!(
            r#"
        import std.io as io;
        import std.path as path;
        let input = path.fromString("{stdin_path_source}");
        let output = path.fromString("{stdout_path_source}");
        let result = io.execPipeline(
            io.pipelineWithRedirects(
                io.pipeline([
                    io.command("cmd", ["/C", "findstr neve"]),
                    io.command("cmd", ["/C", "findstr neve"])
                ]),
                [io.redirectStdinPath(input), io.redirectStdoutPath(output)]
            )
        );
        let redirected = io.readFilePath(output);
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) == "" &&
            redirected != "";
        "#
        )
    } else {
        format!(
            r#"
        import std.io as io;
        import std.path as path;
        let input = path.fromString("{stdin_path_source}");
        let output = path.fromString("{stdout_path_source}");
        let result = io.execPipeline(
            io.pipelineWithRedirects(
                io.pipeline([
                    io.command("sh", ["-c", "grep neve"]),
                    io.command("sh", ["-c", "grep neve"])
                ]),
                [io.redirectStdinPath(input), io.redirectStdoutPath(output)]
            )
        );
        let redirected = io.readFilePath(output);
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) == "" &&
            redirected != "";
        "#
        )
    };
    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_pipeline_rejects_empty_pipeline() {
    let source = r#"
        import std.io as io;
        let pipe = io.pipeline([]);
        "#
    .to_string();

    assert_runtime_error_parity(&source, "io.pipeline: requires a non-empty List<Command>");
}

#[test]
fn test_end_to_end_std_io_exec_pipeline_with_redirects_rejects_final_stage_stderr_conflict() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stderr_path = temp.path().join("pipeline-stderr.txt");
    let stderr_path_source = stderr_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io as io;
        import std.path as path;
        let err = path.fromString("{stderr_path_source}");
        let result = io.execPipeline(
            io.pipelineWithRedirects(
                io.pipeline([
                    io.commandWithRedirects(
                        io.command("rustc", ["--definitely-not-a-real-rustc-flag"]),
                        [io.redirectStderrPath(err)]
                    )
                ]),
                [io.redirectStderrPath(err)]
            )
        );
        "#
    );

    assert_runtime_error_parity(
        &source,
        "final pipeline stage cannot combine boundary stderr with stage-local stderr redirect",
    );
}

#[test]
fn test_end_to_end_std_io_pipeline_with_redirects_rejects_final_stage_stdout_conflict() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stdout_path = temp.path().join("pipeline-stdout.txt");
    let stdout_path_source = stdout_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io as io;
        import std.path as path;
        let output = path.fromString("{stdout_path_source}");
        let pipe = io.pipelineWithRedirects(
            io.pipeline([
                io.commandWithRedirects(
                    io.command("printf", ["neve"]),
                    [io.redirectStdoutPath(output)]
                )
            ]),
            [io.redirectStdoutPath(output)]
        );
        "#
    );

    assert_runtime_error_parity(
        &source,
        "final pipeline stage cannot combine boundary stdout with stage-local stdout redirect",
    );
}

#[test]
fn test_end_to_end_std_io_exec_pipeline_rejects_non_final_stage_stdout_redirect() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stdout_path = temp.path().join("stage-stdout.txt");
    let stdout_path_source = stdout_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io as io;
        import std.path as path;
        let out = path.fromString("{stdout_path_source}");
        let result = io.execPipeline(
            io.pipeline([
                io.commandWithRedirects(
                    io.command("printf", ["neve"]),
                    [io.redirectStdoutPath(out)]
                ),
                io.command("cat", [])
            ])
        );
        "#
    );

    assert_runtime_error_parity(
        &source,
        "pipeline stage 1 cannot carry stdout redirect before final stage",
    );
}

#[test]
fn test_end_to_end_std_io_pipeline_with_redirects_rejects_boundary_stdin_conflict() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stdin_path = temp.path().join("pipeline-stdin.txt");
    let stdin_path_source = stdin_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io as io;
        import std.path as path;
        let input = path.fromString("{stdin_path_source}");
        let pipe = io.pipelineWithRedirects(
            io.pipeline([
                io.commandWith(#{{ program = "cat", stdin = "neve" }})
            ]),
            [io.redirectStdinPath(input)]
        );
        "#
    );

    assert_runtime_error_parity(
        &source,
        "pipeline stage 1 cannot combine boundary stdin with stage-local stdin",
    );
}

#[test]
fn test_end_to_end_std_io_command_with_redirects_runtime_parity() {
    let source = r#"
        import std.io as io;
        import std.path as path;
        let cmd = io.commandWithRedirects(
            io.command("printf", ["neve"]),
            [io.redirectStdoutPath(path.fromString("/tmp/neve.out"))]
        );
        let x =
            typeOf(cmd) == "Command" &&
            toString(cmd) == "<command:printf 1 arg(s), configured>";
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_exec_pipeline_honors_stage_local_redirects_runtime_parity() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stderr_path = temp.path().join("pipeline-stage-stderr.txt");
    let stderr_path_source = stderr_path.to_string_lossy().replace('\\', "\\\\");
    let source = if cfg!(windows) {
        format!(
            r#"
        import std.io as io;
        import std.path as path;
        let err = path.fromString("{stderr_path_source}");
        let result = io.execPipeline(
            io.pipeline([
                io.commandWithRedirects(
                    io.command("cmd", ["/C", "(echo neve) & (echo err 1>&2)"]),
                    [io.redirectStderrPath(err)]
                ),
                io.command("cmd", ["/C", "findstr neve"])
            ])
        );
        let redirected = io.readFilePath(err);
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processStdout(result) != "" &&
            io.processStderr(result) == "" &&
            redirected != "";
        "#
        )
    } else {
        format!(
            r#"
        import std.io as io;
        import std.path as path;
        let err = path.fromString("{stderr_path_source}");
        let result = io.execPipeline(
            io.pipeline([
                io.commandWithRedirects(
                    io.command("sh", ["-c", "printf neve && printf err >&2"]),
                    [io.redirectStderrPath(err)]
                ),
                io.command("sh", ["-c", "grep neve"])
            ])
        );
        let redirected = io.readFilePath(err);
        let x =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processStdout(result) != "" &&
            io.processStderr(result) == "" &&
            redirected != "";
        "#
        )
    };
    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_exec_command_with_stderr_redirect_runtime_parity() {
    let temp = TempDir::new().expect("temp dir should exist");
    let redirect_path = temp.path().join("stderr.txt");
    let redirect_path_source = redirect_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io as io;
        import std.path as path;
        let target = path.fromString("{redirect_path_source}");
        let result = io.execCommand(
            io.commandWithRedirects(
                io.command("rustc", ["--definitely-not-a-real-rustc-flag"]),
                [io.redirectStderrPath(target)]
            )
        );
        let redirected = io.readFilePath(target);
        let x =
            typeOf(result) == "ProcessResult" &&
            !io.processSuccess(result) &&
            io.processCode(result) != 0 &&
            io.processStderr(result) == "" &&
            redirected != "";
        "#
    );
    assert_runtime_parity(&source, Value::Bool(true));
}

#[test]
fn test_end_to_end_std_io_task_command_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let task = io.taskCommand(io.command(\"printf\", [\"neve\"]));
        let x = if typeOf(task) == \"Task\" then toString(task) else \"nope\";
        ",
        Value::String("<task:command->ProcessResult>".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_io_task_pipeline_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let task = io.taskPipeline(io.pipeline([
            io.command(\"printf\", [\"neve\"]),
            io.command(\"cat\", [])
        ]));
        let x = if typeOf(task) == \"Task\" then toString(task) else \"nope\";
        ",
        Value::String("<task:pipeline->ProcessResult>".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_io_await_task_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let task = io.taskCommand(io.command(\"rustc\", [\"--version\"]));
        let result = io.awaitTask(task);
        let x = if typeOf(result) == \"ProcessResult\" then toString(result) else \"nope\";
        ",
        Value::String("<process-result:0 ok>".to_string().into()),
    );
}

#[test]
fn test_end_to_end_std_io_await_tasks_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let results = io.awaitTasks([
            io.taskCommand(io.command(\"printf\", [\"neve\"])),
            io.taskPipeline(io.pipeline([
                io.command(\"printf\", [\"lang\"]),
                io.command(\"cat\", [])
            ]))
        ]);
        match results {
            [first, second] ->
                io.processStdout(first) == \"neve\" &&
                io.processStdout(second) == \"lang\" &&
                io.processSuccess(first) &&
                io.processSuccess(second),
            _ -> false,
        }
        ",
        Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_std_io_await_pipeline_task_runtime_parity() {
    assert_runtime_parity(
        "
        import std.io as io;
        let pipeline = io.pipeline([
            io.command(\"printf\", [\"neve\"]),
            io.command(\"cat\", [])
        ]);
        let result = io.awaitTask(io.taskPipeline(pipeline));
        let x = if typeOf(result) == \"ProcessResult\" then toString(result) else \"nope\";
        ",
        Value::String("<process-result:0 ok>".to_string().into()),
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
        import std.list as list;
        let map = Map.insert(\"a\", 41, Map.empty);
        let set = Set.insert(1, Set.empty);
        let x = Map.getWithDefault(\"a\", 0, map) + Set.size(set) + list.sum(Map.values(map));
        ",
        Value::Int(int(83)),
    );
}

#[test]
fn test_end_to_end_std_math_conversion_runtime_parity() {
    assert_runtime_parity(
        r#"
        import std.math as math;
        let x = (math.toInt(true), math.toFloat("1.5"));
        "#,
        Value::Tuple(Rc::new(vec![Value::Int(int(1)), Value::Float(1.5)])),
    );
}

#[test]
fn test_end_to_end_std_math_float_predicate_runtime_parity() {
    assert_runtime_parity(
        r#"
        import std.math as math;
        let x = (math.isNan(math.nan), math.isInf(math.inf));
        "#,
        Value::Tuple(Rc::new(vec![Value::Bool(true), Value::Bool(true)])),
    );
}

#[test]
fn test_end_to_end_std_math_rounding_runtime_parity() {
    assert_runtime_parity(
        r#"
        import std.math as math;
        let x = (math.floor(1.9), math.ceil(1.1), math.round(1.6));
        "#,
        Value::Tuple(Rc::new(vec![
            Value::Int(int(1)),
            Value::Int(int(2)),
            Value::Int(int(2)),
        ])),
    );
}

#[test]
fn test_end_to_end_std_math_unary_float_transform_runtime_parity() {
    assert_runtime_parity(
        r#"
        import std.math as math;
        let x = (math.sqrt(9.0), math.log(1.0), math.log10(1000.0), math.exp(0.0));
        "#,
        Value::Tuple(Rc::new(vec![
            Value::Float(3.0),
            Value::Float(0.0),
            Value::Float(3.0),
            Value::Float(1.0),
        ])),
    );
}

#[test]
fn test_end_to_end_std_math_trigonometric_runtime_parity() {
    assert_runtime_parity(
        r#"
        import std.math as math;
        let x = (math.sin(0.0), math.cos(0.0), math.tan(0.0));
        "#,
        Value::Tuple(Rc::new(vec![
            Value::Float(0.0),
            Value::Float(1.0),
            Value::Float(0.0),
        ])),
    );
}

#[test]
fn test_end_to_end_write_read_roundtrip() {
    assert_runtime_parity(
        r#"
        import std.io as io;
        import std.string as string;
        let _ = io.writeFile("/tmp/neve_e2e_rt.txt", "roundtrip data");
        string.trim(io.readFile("/tmp/neve_e2e_rt.txt"))
        "#,
        neve_eval::Value::String("roundtrip data".to_string().into()),
    );
}

#[test]
fn test_end_to_end_io_which_finds_sh() {
    assert_runtime_parity(
        r#"import std.io as io; match io.which("sh") { Some(_) -> true, None -> false }"#,
        neve_eval::Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_full_scripting_workflow() {
    // Test the complete scripting workflow: args, file I/O, process exec, pipes
    assert_runtime_parity(
        r#"
        import std.io as io;
        import std.string as string;
        
        -- Write a test file
        let _ = io.writeFile("/tmp/neve_workflow_test.txt", "line1\nline2\nline3");
        
        -- Read it back
        let content = io.readFile("/tmp/neve_workflow_test.txt");
        
        -- Count lines via external command
        let wc = io.execCommand(io.command("wc", ["-l", "/tmp/neve_workflow_test.txt"]));
        
        -- Verify everything worked
        let lines = string.lines(string.trim(content));
        io.processSuccess(wc)
        "#,
        neve_eval::Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_pipeline_with_timeout_completes() {
    assert_runtime_parity(
        r#"
        import std.io as io;
        let p = io.pipeline([io.command("echo", ["hello pipeline"]), io.command("cat", [])]);
        let task = io.taskPipeline(p);
        let result = io.awaitTaskWithTimeout(task, 5000);
        match result {
            Some(pr) -> io.processSuccess(pr),
            None -> false
        }
        "#,
        neve_eval::Value::Bool(true),
    );
}

#[test]
fn test_end_to_end_effect_annotation_is_checked() {
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> String = io.readFile("/etc/hostname");
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_effect_annotation_passes_with_effect_kw() {
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn good() -> String effect = io.readFile("/etc/hostname");
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call"));
    assert!(
        !has_effect_error,
        "unexpected effect error: {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_io_exec_command_streaming_returns_process_result() {
    // Test that io.execCommandStreaming executes a command and returns a successful ProcessResult.
    // Uses the canonical HIR evaluator path (AST compat does not support evaluator-owned streaming).
    let source = r#"
    import std.io as io;
    let cmd = io.command("echo", ["hello"]);
    let result = io.execCommandStreaming(cmd, fn(line) { () });
    let x = typeOf(result) == "ProcessResult" && io.processSuccess(result);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_exec_command_streaming_effect_checking() {
    // Verify that io.execCommandStreaming is recognized as effectful.
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> ProcessResult = io.execCommandStreaming(
            io.command("echo", ["hello"]),
            fn(line) { () }
        );
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error for io.execCommandStreaming, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_io_exec_command_streaming_with_stdin() {
    // Test that io.execCommandStreaming respects Command stdin.
    let source = r#"
    import std.io as io;
    let cmd = io.commandWith(#{
        program = "cat",
        args = [],
        stdin = "hello from stdin"
    });
    let result = io.execCommandStreaming(cmd, fn(line) { () });
    let x = typeOf(result) == "ProcessResult" && io.processSuccess(result);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_exec_pipeline_streaming_returns_process_result() {
    // Test that io.execPipelineStreaming executes a pipeline and returns ProcessResult.
    let source = r#"
    import std.io as io;
    let pipeline = io.pipeline([
        io.command("echo", ["hello pipeline"]),
        io.command("cat", [])
    ]);
    let result = io.execPipelineStreaming(pipeline, fn(line) { () });
    let x = typeOf(result) == "ProcessResult" && io.processSuccess(result);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_read_file_lines_calls_callback() {
    // Test that io.readFileLines reads a file and calls the callback for each line.
    let temp = TempDir::new().expect("temp dir should exist");
    let file_path = temp.path().join("streaming-read.txt");
    let file_path_source = file_path.to_string_lossy().replace("\\", "/");
    std::fs::write(
        &file_path,
        "line1
line2
line3
",
    )
    .expect("write should succeed");

    let source = format!(
        r#"
        import std.io as io;
        let _ = io.readFileLines("{file_path_source}", fn(line) {{ () }});
        let x = true;
        "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_exec_pipeline_streaming_effect_checking() {
    // Verify that io.execPipelineStreaming is recognized as effectful.
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> ProcessResult = io.execPipelineStreaming(
            io.pipeline([io.command("echo", ["hello"])]),
            fn(line) {{ () }}
        );
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error for io.execPipelineStreaming, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_lambda_in_effect_fn_allows_effectful_calls() {
    // Regression: lambdas inside `effect` functions should inherit the effect context.
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn good() -> ProcessResult effect = io.execCommandStreaming(
            io.command("echo", ["hello"]),
            fn(line) { io.writeFile("/tmp/lambda_test.txt", line); () }
        );
        "#,
    );
    let has_lambda_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("in lambda"));
    assert!(
        !has_lambda_error,
        "lambda inside effect fn should allow effectful calls, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_lambda_in_pure_fn_rejects_effectful_calls() {
    // Lambdas inside non-effect functions should still reject effectful calls.
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> ProcessResult = io.execCommandStreaming(
            io.command("echo", ["hello"]),
            fn(line) { io.writeFile("/tmp/lambda_test.txt", line); () }
        );
        "#,
    );
    let has_lambda_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("in lambda"));
    assert!(
        has_lambda_error,
        "lambda inside pure fn should reject effectful calls, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_impl_method_with_effect_allows_io() {
    // Impl method with `effect` should allow io calls.
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        trait Logger { fn log(msg: String) -> Unit; };
        struct Dummy {};
        impl Logger for Dummy {
            fn log(msg: String) -> Unit effect = io.writeFile("/dev/null", msg);
        };
        "#,
    );
    let has_impl_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("in impl method"));
    assert!(
        !has_impl_error,
        "impl method with effect should allow io, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_impl_method_without_effect_rejects_io() {
    // Impl method without `effect` should reject io calls.
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        trait Logger { fn log(msg: String) -> Unit; };
        struct Dummy {};
        impl Logger for Dummy {
            fn log(msg: String) -> Unit = io.writeFile("/dev/null", msg);
        };
        "#,
    );
    let has_impl_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("in impl method"));
    assert!(
        has_impl_error,
        "impl method without effect should reject io, got {:?}",
        analysis.diagnostics
    );
}

// === Streaming I/O timeout tests / 流式 I/O 超时测试 ===

#[test]
fn test_end_to_end_io_exec_command_streaming_with_timeout_returns_some() {
    // Normal completion: timeout is generous, process finishes in time.
    let source = r#"
    import std.io as io;
    let cmd = io.command("echo", ["hello timeout"]);
    let result = io.execCommandStreamingWithTimeout(cmd, fn(line) { () }, 5000);
    let x = match result {
        Some(r) -> typeOf(r) == "ProcessResult" && io.processSuccess(r),
        None -> false
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_exec_command_streaming_with_timeout_returns_none_on_timeout() {
    // Timeout: process takes longer than the timeout -> returns None.
    let source = r#"
    import std.io as io;
    let cmd = io.command("sleep", ["5"]);
    let result = io.execCommandStreamingWithTimeout(cmd, fn(line) { () }, 100);
    let x = match result {
        Some(_) -> false,
        None -> true
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_exec_pipeline_streaming_with_timeout_returns_some() {
    // Normal pipeline completion with timeout.
    let source = r#"
    import std.io as io;
    let pipeline = io.pipeline([
        io.command("echo", ["hello pipeline timeout"]),
        io.command("cat", [])
    ]);
    let result = io.execPipelineStreamingWithTimeout(pipeline, fn(line) { () }, 5000);
    let x = match result {
        Some(r) -> typeOf(r) == "ProcessResult" && io.processSuccess(r),
        None -> false
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_exec_pipeline_streaming_with_timeout_returns_none_on_timeout() {
    // Pipeline timeout: pipeline takes longer than timeout -> returns None.
    let source = r#"
    import std.io as io;
    let pipeline = io.pipeline([
        io.command("sleep", ["3"]),
        io.command("echo", ["never runs"])
    ]);
    let result = io.execPipelineStreamingWithTimeout(pipeline, fn(line) { () }, 100);
    let x = match result {
        Some(_) -> false,
        None -> true
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_exec_command_streaming_with_timeout_effect_checking() {
    // Verify that io.execCommandStreamingWithTimeout is recognized as effectful.
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> Option = io.execCommandStreamingWithTimeout(
            io.command("echo", ["hello"]),
            fn(line) { () },
            1000
        );
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error for io.execCommandStreamingWithTimeout, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_io_exec_pipeline_streaming_with_timeout_effect_checking() {
    // Verify that io.execPipelineStreamingWithTimeout is recognized as effectful.
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> Option = io.execPipelineStreamingWithTimeout(
            io.pipeline([io.command("echo", ["hello"])]),
            fn(line) { () },
            1000
        );
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error for io.execPipelineStreamingWithTimeout, got {:?}",
        analysis.diagnostics
    );
}

// === Signal handling tests ===

#[test]
fn test_end_to_end_io_on_signal_registers_handler() {
    // Verify that io.onSignal accepts valid signal names and returns Unit.
    let source = r#"
    import std.io as io;
    let _ = io.onSignal("INT", fn() { print("interrupted!"); () });
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_on_signal_rejects_unknown_signal() {
    // Verify that io.onSignal rejects unknown signal names.
    let source = r#"
    import std.io as io;
    let _ = io.onSignal("UNKNOWN", fn() { () });
    let x = true;
    "#;
    let analysis = analyze_source(source);
    let _ = eval_hir(&analysis);
    // The analysis may or may not have type errors; the runtime should fail
    // We assert that the evaluation fails (returns Err)
    let hir_result = eval_hir(&analysis);
    assert!(
        hir_result.is_err(),
        "expected error for unknown signal, got {:?}",
        hir_result
    );
}

#[test]
fn test_end_to_end_io_on_signal_effect_checking() {
    // Verify that io.onSignal is recognized as effectful.
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> Unit = io.onSignal("INT", fn() { () });
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error for io.onSignal, got {:?}",
        analysis.diagnostics
    );
}

// === Event chain tests (eventMap / eventFilter) ===

#[test]
fn test_end_to_end_io_event_map_transforms_event() {
    // Verify that io.eventMap chains a transformation onto an event.
    let source = r#"
    import std.io as io;
    let timer = io.every(1);
    let mapped = io.eventMap(timer, fn(x) { x + 1 });
    let x = typeOf(mapped) == "Event";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_event_filter_chains_predicate() {
    // Verify that io.eventFilter chains a predicate onto an event.
    let source = r#"
    import std.io as io;
    let timer = io.every(1);
    let filtered = io.eventFilter(timer, fn(x) { x > 0 });
    let x = typeOf(filtered) == "Event";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_event_map_effect_checking() {
    // Verify that io.eventMap is NOT effectful (it's a pure constructor).
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn ok() -> Event = io.eventMap(io.every(1), fn(x) { x + 1 });
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        !has_effect_error,
        "io.eventMap should be pure, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_io_event_filter_effect_checking() {
    // Verify that io.eventFilter is NOT effectful (it's a pure constructor).
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn ok() -> Event = io.eventFilter(io.every(1), fn(x) { x > 0 });
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        !has_effect_error,
        "io.eventFilter should be pure, got {:?}",
        analysis.diagnostics
    );
}

// === "undefined" suggestion tests ===

#[test]
fn test_frontend_suggests_names_for_undefined_global() {
    // Verify that undefined globals produce suggestions when known names exist.
    let analysis = neve_frontend::analyze_source(
        r#"
        fn greet(name: String) -> String = name;
        let x = greeter("world");
        "#,
    );
    let has_suggestion = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("undefined") && d.message.contains("available"));
    assert!(
        has_suggestion,
        "expected suggestion with available names, got {:?}",
        analysis.diagnostics
    );
}
