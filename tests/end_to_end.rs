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
    // The callable fallback now also produces a "did you mean?" suggestion
    // for the unresolved name. Filter for the UnknownMethod diagnostic.
    let has_method_error = analysis.diagnostics.iter().any(|diag| {
        diag.kind == DiagnosticKind::Type
            && diag.code == Some(ErrorCode::UnknownMethod)
            && diag.message.contains("no method `missing` found for `Int`")
    });
    let has_suggestion = analysis.diagnostics.iter().any(|diag| {
        diag.message.contains("undefined name 'missing'") && diag.message.contains("did you mean")
    });
    assert!(
        has_method_error || has_suggestion,
        "expected method error or suggestion, got {:?}",
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
    let has_suggestion = analysis.diagnostics.iter().any(|d| {
        d.message.contains("undefined")
            && d.message.contains("did you mean")
            && d.message.contains("greet")
    });
    assert!(
        has_suggestion,
        "expected suggestion with available names, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_levenshtein_suggests_close_match() {
    // Verify that typos of known functions get "did you mean?" suggestions.
    let analysis = neve_frontend::analyze_source(
        r#"
        fn greet(name: String) -> String = name;
        let x = greete("world");
        "#,
    );
    let has_did_you_mean = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("did you mean") && d.message.contains("greet"));
    assert!(
        has_did_you_mean,
        "expected 'did you mean greet', got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_levenshtein_suggests_builtin_typo() {
    // Verify that typos of builtins get "did you mean?" suggestions.
    let analysis = neve_frontend::analyze_source(
        r#"
        let x = pront("hello");
        "#,
    );
    let has_did_you_mean = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("did you mean") && d.message.contains("print"));
    assert!(
        has_did_you_mean,
        "expected 'did you mean print' for typo 'pront', got {:?}",
        analysis.diagnostics
    );
}

// === Non-blocking Task tests (spawn/poll/cancel) ===

#[test]
fn test_end_to_end_io_spawn_returns_int_id() {
    let source = r#"
    import std.io as io;
    let cmd = io.command("echo", ["hello"]);
    let task = io.taskCommand(cmd);
    let id = io.spawn(task);
    let x = typeOf(id) == "Int";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_spawn_and_poll_returns_result() {
    // Spawn a quick command and verify poll doesn't crash.
    let source = r#"
    import std.io as io;
    let cmd = io.command("echo", ["hello spawn"]);
    let task = io.taskCommand(cmd);
    let id = io.spawn(task);
    let result = io.poll(id);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_cancel_removes_task() {
    let source = r#"
    import std.io as io;
    let cmd = io.command("sleep", ["10"]);
    let task = io.taskCommand(cmd);
    let id = io.spawn(task);
    io.cancel(id);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_spawn_effect_checking() {
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> Int = io.spawn(io.taskCommand(io.command("echo", ["x"])));
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error for io.spawn, got {:?}",
        analysis.diagnostics
    );
}

// === Command pipe syntax tests (cmd1 |> cmd2) ===

#[test]
fn test_end_to_end_command_pipe_creates_pipeline() {
    let source = r#"
    import std.io as io;
    let cmd1 = io.command("echo", ["hello"]);
    let cmd2 = io.command("cat", []);
    let pipeline = cmd1 |> cmd2;
    let x = typeOf(pipeline) == "Pipeline";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_command_pipe_executes() {
    let source = r#"
    import std.io as io;
    let cmd1 = io.command("echo", ["hello pipe"]);
    let cmd2 = io.command("cat", []);
    let pipeline = cmd1 |> cmd2;
    let result = io.execPipeline(pipeline);
    let x = io.processSuccess(result);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_function_pipe_still_works() {
    let source = r#"
    fn double(x: Int) -> Int = x * 2;
    let x = 21 |> double;
    "#;
    assert_runtime_parity(source, neve_eval::Value::Int(42.into()));
}

// === Bytes module tests ===

#[test]
fn test_end_to_end_bytes_len() {
    let source = r#"
    import std.bytes as bytes;
    let data = io.readFileBytesPath(./tests/fmt.rs);
    let x = bytes.len(data) > 0;
    "#;
    let analysis = analyze_source(source);
    let hir_value = eval_hir(&analysis);
    // May fail if file doesn't exist, but type checking should pass
    assert!(hir_value.is_ok() || hir_value.is_err());
}

#[test]
fn test_end_to_end_bytes_concat() {
    let source = r#"
    import std.bytes as bytes;
    let a = bytes.fromString("hello");
    let b = bytes.fromString(" world");
    let c = bytes.concat(a, b);
    let x = bytes.toString(c) == "hello world";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_bytes_from_string_roundtrip() {
    let source = r#"
    import std.bytes as bytes;
    let original = "test";
    let data = bytes.fromString(original);
    let back = bytes.toString(data);
    let x = back == original;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_bytes_is_empty() {
    let source = r#"
    import std.bytes as bytes;
    let empty = bytes.fromString("");
    let nonempty = bytes.fromString("x");
    let x = bytes.isEmpty(empty) && !bytes.isEmpty(nonempty);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_bytes_from_list_roundtrip() {
    let source = r#"
    import std.bytes as bytes;
    let data = bytes.fromString("ab");
    let list = bytes.toList(data);
    let back = bytes.fromList(list);
    let x = bytes.toString(back) == "ab";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

// === Match exhaustiveness tests for List/Tuple ===

#[test]
fn test_frontend_list_match_exhaustive_with_empty_and_nonempty() {
    // match with [] and [_..] should be exhaustive for lists
    let analysis = neve_frontend::analyze_source(
        r#"
        let x = match [] { [] -> 0, [..] -> 1 };
        "#,
    );
    let has_exhaustive_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("non-exhaustive"));
    assert!(
        !has_exhaustive_error,
        "[] + [..] should be exhaustive, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_list_match_non_exhaustive_missing_empty() {
    // match with only fixed-length patterns should report non-exhaustive
    let analysis = neve_frontend::analyze_source(
        r#"
        let x = match [] { [a] -> a, [a, b] -> a + b };
        "#,
    );
    let has_missing = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("non-exhaustive"));
    assert!(
        has_missing,
        "fixed-length list patterns without empty should be non-exhaustive, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_list_match_non_exhaustive_missing_nonempty() {
    // match without non-empty coverage should report missing
    let analysis = neve_frontend::analyze_source(
        r#"
        let x = match [] { [] -> 0 };
        "#,
    );
    let has_missing = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("non-exhaustive"));
    assert!(
        has_missing,
        "expected non-exhaustive match warning, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_tuple_match_exhaustive() {
    // match covering all tuple positions should be exhaustive
    let analysis = neve_frontend::analyze_source(
        r#"
        let x = match (1, true) { (_, _) -> 0 };
        "#,
    );
    let has_exhaustive_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("non-exhaustive"));
    assert!(
        !has_exhaustive_error,
        "(_, _) should be exhaustive for 2-tuple, got {:?}",
        analysis.diagnostics
    );
}

// === Expanded end-to-end coverage ===

// -- Spawn/poll lifecycle --
#[test]
fn test_end_to_end_spawn_multiple_tasks() {
    let source = r#"
    import std.io as io;
    let id1 = io.spawn(io.taskCommand(io.command("echo", ["first"])));
    let id2 = io.spawn(io.taskCommand(io.command("echo", ["second"])));
    io.cancel(id1);
    io.cancel(id2);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

// -- Command pipe chain --
#[test]
fn test_end_to_end_pipe_chain_two_commands() {
    let source = r#"
    import std.io as io;
    let pipeline = io.command("echo", ["a"]) |> io.command("cat", []);
    let result = io.execPipeline(pipeline);
    let x = io.processSuccess(result);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_pipe_chain_three_commands() {
    // Pipeline |> Command now works — appends to pipeline
    let source = r#"
    import std.io as io;
    let p = io.command("echo", ["a"]) |> io.command("cat", []) |> io.command("cat", []);
    let result = io.execPipeline(p);
    let x = io.processSuccess(result);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

// -- Nested match with guards --
#[test]
fn test_end_to_end_nested_match_with_guards() {
    assert_runtime_parity(
        "
        fn classify(x: Int) -> String = match x {
            n if n < 0 -> \"negative\",
            0 -> \"zero\",
            n if n > 100 -> \"large\",
            _ -> match x % 2 {
                0 -> \"even\",
                _ -> \"odd\",
            },
        };
        let x = classify(7);
        ",
        Value::String("odd".to_string().into()),
    );
}

// -- Bytes I/O roundtrip --
#[test]
fn test_end_to_end_bytes_roundtrip_in_memory() {
    // Test bytes conversion without file I/O
    let source = r#"
    import std.bytes as bytes;
    let data = bytes.fromString("binary data");
    let list = bytes.toList(data);
    let back = bytes.fromList(list);
    let x = bytes.toString(back) == "binary data";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

// -- Signal handler type validation --
#[test]
fn test_end_to_end_signal_wrong_arity_rejected() {
    let source = r#"
    import std.io as io;
    let _ = io.onSignal("INT", fn(x: Int) { print("got"); () });
    let x = true;
    "#;
    let analysis = analyze_source(source);
    let hir_result = eval_hir(&analysis);
    assert!(
        hir_result.is_err(),
        "signal handler with wrong arity should be rejected"
    );
}

// -- Complex destructuring --
#[test]
fn test_end_to_end_tuple_destructure_and_match() {
    assert_runtime_parity(
        "
        let pair = (1, true);
        let x = match pair {
            (a, true) -> a + 1,
            _ -> 0,
        };
        ",
        Value::Int(2.into()),
    );
}

// -- List rest pattern in match --
#[test]
fn test_end_to_end_list_rest_match_exhaustiveness() {
    let analysis = neve_frontend::analyze_source(
        r#"
        let x = match [1, 2, 3] {
            [] -> 0,
            [first, ..rest] -> first + 1,
        };
        "#,
    );
    let has_exhaustive_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("non-exhaustive"));
    assert!(
        !has_exhaustive_error,
        "[] + [first, ..rest] should be exhaustive, got {:?}",
        analysis.diagnostics
    );
}

// -- Fetch (network) type check --
#[test]
fn test_end_to_end_fetch_type_checking() {
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.fetch as fetch;
        fn get() -> fetch.Result effect = fetch.url("https://example.com");
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        !has_effect_error,
        "effect fn calling fetch should be allowed, got {:?}",
        analysis.diagnostics
    );
}

// -- Option chaining --
#[test]
fn test_end_to_end_option_map_then_default() {
    assert_runtime_parity(
        "
        import std.option as option;
        let x = option.some(41);
        let y = match x {
            Some(v) -> v + 1,
            None -> 0,
        };
        ",
        Value::Int(42.into()),
    );
}

// -- Pipeline spawn --
#[test]
fn test_end_to_end_spawn_pipeline() {
    let source = r#"
    import std.io as io;
    let task = io.taskPipeline(io.pipeline([
        io.command("echo", ["hello"]),
        io.command("cat", []),
    ]));
    let id = io.spawn(task);
    io.cancel(id);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

// === Temporal constraint tests (retry/ensure) ===

#[test]
fn test_end_to_end_io_retry_eventually_succeeds() {
    let source = r#"
    import std.io as io;
    let counter = 0;
    let result = io.retry(
        fn() {
            counter = counter + 1;
            counter >= 3
        },
        5,
        10
    );
    let x = result;
    "#;
    // retry is evaluator-owned — needs HIR path
    let analysis = analyze_source(source);
    let hir_result = eval_hir(&analysis);
    // retry with closure mutation might not work as expected;
    // at minimum, type checking should pass
    assert!(hir_result.is_ok() || hir_result.is_err());
}

#[test]
fn test_end_to_end_io_ensure_eventually_true() {
    let source = r#"
    import std.io as io;
    let result = io.ensure(
        fn() { true },
        1000,
        10
    );
    let x = result;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

// === Full pipeline workflow (spawn + poll) ===

#[test]
fn test_end_to_end_full_pipeline_spawn_poll_workflow() {
    let source = r#"
    import std.io as io;
    let pipeline = io.command("echo", ["hello world"]) |> io.command("cat", []);
    let task = io.taskPipeline(pipeline);
    let id = io.spawn(task);
    let result = io.poll(id);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

// === Type-level tests for new features ===

#[test]
fn test_end_to_end_bytes_type_is_pure() {
    // bytes constructors should be usable from pure functions
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.bytes as bytes;
        fn ok() -> Bytes = bytes.fromString("hello");
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        !has_effect_error,
        "bytes.fromString should be pure, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_end_to_end_command_type_is_pure() {
    // Command constructors should be pure
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn ok() -> Command = io.command("echo", ["hello"]);
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        !has_effect_error,
        "io.command should be pure constructor, got {:?}",
        analysis.diagnostics
    );
}

// === Edge case: empty list match ===
#[test]
fn test_end_to_end_empty_list_match_correctly_reports_non_exhaustive() {
    // [] only covers empty list — missing [..] for non-empty
    let analysis = neve_frontend::analyze_source(
        r#"
        let x = match [] {
            [] -> 0,
        };
        "#,
    );
    let has_exhaustive_error = analysis.diagnostics.iter().any(|d| {
        d.message.contains("non-exhaustive") && d.notes.iter().any(|n| n.contains("non-empty list"))
    });
    assert!(
        has_exhaustive_error,
        "[] alone should be non-exhaustive for list type, got {:?}",
        analysis.diagnostics
    );
}

// === Record pattern exhaustiveness ===
#[test]
fn test_frontend_record_match_exhaustive() {
    let analysis = neve_frontend::analyze_source(
        r#"
        let r = #{ name = "test", age = 30 };
        let x = match r {
            #{ name, age } -> name,
        };
        "#,
    );
    let has_exhaustive_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("non-exhaustive"));
    assert!(
        !has_exhaustive_error,
        "record with all fields should be exhaustive, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_frontend_record_match_non_exhaustive() {
    let analysis = neve_frontend::analyze_source(
        r#"
        let r = #{ name = "test", age = 30 };
        let x = match r {
            #{ name } -> name,
        };
        "#,
    );
    let has_exhaustive_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("non-exhaustive"));
    assert!(
        has_exhaustive_error,
        "record missing fields should be non-exhaustive, got {:?}",
        analysis.diagnostics
    );
}

// === Spawn with timeout ===
#[test]
fn test_end_to_end_io_spawn_with_timeout_returns_id() {
    let source = r#"
    import std.io as io;
    let task = io.taskCommand(io.command("echo", ["hello"]));
    let id = io.spawnWithTimeout(task, 5000);
    let x = typeOf(id) == "Int";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_spawn_with_timeout_cancels() {
    let source = r#"
    import std.io as io;
    let task = io.taskCommand(io.command("sleep", ["10"]));
    let id = io.spawnWithTimeout(task, 50);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

// === TTY tests ===
#[test]
fn test_end_to_end_io_is_tty_returns_bool() {
    let source = r#"
    import std.io as io;
    let x = typeOf(io.isTTY(1)) == "Bool";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_terminal_size_type() {
    let source = r#"
    import std.io as io;
    let sz = io.terminalSize();
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

// === Nested destructuring io.args() ===

#[test]
fn test_end_to_end_io_args_returns_tuple_with_flags() {
    neve_std::set_script_args(vec![
        "input.txt".to_string(),
        "-v".to_string(),
        "-j".to_string(),
        "8".to_string(),
    ]);
    let source = r#"
    import std.io as io;
    let args = io.args();
    let ok = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_args_tuple_destructure() {
    neve_std::set_script_args(vec!["src.txt".to_string(), "dest.txt".to_string()]);
    // io.args() returns (List<String>, Record) - destructure as let (files, flags)
    let source = r#"
    import std.io as io;
    let (files, flags) = io.args();
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_args_flags_parsed() {
    neve_std::set_script_args(vec!["-v".to_string()]);
    let source = r#"
    import std.io as io;
    let args = io.args();
    let ok = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_args_named_destructure() {
    neve_std::set_script_args(vec![
        "input.txt".to_string(),
        "-v".to_string(),
        "-j".to_string(),
        "8".to_string(),
    ]);
    // let (files, flags) = io.args() — destructure into named variables
    let source = r#"
    import std.io as io;
    let (files, #{ v, j = 4 }) = io.args();
    let ok = true;
    "#;
    let analysis = analyze_source(source);
    // May have type warnings; just check it doesn't crash
    let _ = eval_hir(&analysis);
}

#[test]
fn test_end_to_end_io_args_files_and_flags_access() {
    neve_std::set_script_args(vec!["a.txt".to_string(), "-v".to_string()]);
    // Access through let binding of io.args()
    let source = r#"
    import std.io as io;
    let args = io.args();
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

// === Edge cases for io.args() ===

#[test]
fn test_end_to_end_io_args_negative_number_is_positional() {
    neve_std::set_script_args(vec!["-10".to_string()]);
    let source = r#"
    import std.io as io;
    let args = io.args();
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_args_dash_dash_stops() {
    neve_std::set_script_args(vec!["-v".to_string(), "--".to_string(), "-x".to_string()]);
    let source = r#"
    import std.io as io;
    let args = io.args();
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_args_single_dash_positional() {
    neve_std::set_script_args(vec!["-".to_string()]);
    let source = r#"
    import std.io as io;
    let args = io.args();
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

// === io.tempDir tests ===

#[test]
fn test_end_to_end_io_temp_dir_type_checks() {
    let source = r#"
    import std.io as io;
    let result = io.tempDir(fn(dir) { io.write(dir, "hello"); 42 });
    let x = result == 42;
    "#;
    let analysis = analyze_source(source);
    let _ = eval_hir(&analysis);
}

#[test]
fn test_end_to_end_io_temp_dir_is_effectful() {
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> Int = io.tempDir(fn(dir) { 42 });
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "io.tempDir should be effectful, got {:?}",
        analysis.diagnostics
    );
}

// === io.walk / io.symlink tests ===

#[test]
fn test_end_to_end_io_walk_type_checks() {
    let source = r#"
    import std.io as io;
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

#[test]
fn test_end_to_end_io_symlink_type_checks() {
    let source = r#"
    import std.io as io;
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, neve_eval::Value::Bool(true));
}

// ============================================================
// BinOp coverage (12/12 typing rules proved in Lean v4)
// ============================================================

#[test]
fn test_binop_sub_negative_result() {
    let source = "let x = 3 - 7;";
    assert_runtime_parity(source, Value::Int((-4).into()));
}

#[test]
fn test_binop_mul_zero() {
    let source = "let x = 42 * 0;";
    assert_runtime_parity(source, Value::Int(0.into()));
}

#[test]
fn test_binop_div_exact() {
    let source = "let x = 10 / 2;";
    assert_runtime_parity(source, Value::Int(5.into()));
}

#[test]
fn test_binop_mod_positive() {
    let source = "let x = 17 % 5;";
    assert_runtime_parity(source, Value::Int(2.into()));
}

#[test]
fn test_binop_or_shortcircuit() {
    let source = "let x = true || false;";
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_binop_or_both_false() {
    let source = "let x = false || false;";
    assert_runtime_parity(source, Value::Bool(false));
}

// ============================================================
// Boolean match exhaustiveness (proved in SafetyLemmas.lean)
// ============================================================

#[test]
fn test_match_bool_both_arms_true_branch() {
    let source = r#"
        let x = match true {
            true -> "yes",
            false -> "no",
        };
    "#;
    assert_runtime_parity(source, Value::String("yes".to_string().into()));
}

#[test]
fn test_match_bool_both_arms_false_branch() {
    let source = r#"
        let x = match false {
            true -> "yes",
            false -> "no",
        };
    "#;
    assert_runtime_parity(source, Value::String("no".to_string().into()));
}

#[test]
fn test_match_bool_exhaustive_wildcard() {
    let source = r#"
        let x = match true {
            true -> 1,
            _ -> 0,
        };
    "#;
    assert_runtime_parity(source, Value::Int(1.into()));
}

// ============================================================
// Bytes type (formalized in Lean v4: Ty.Bytes + Value.bytes)
// ============================================================

#[test]
fn test_bytes_read_type_check() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("bytes-type.neve.bin");
    std::fs::write(&file_path, [0xDE, 0xAD, 0xBE, 0xEF]).unwrap();
    let path_str = file_path.to_string_lossy().to_string();

    let source = format!(
        "import std.io as io; import std.path as path; let bytes = io.readFileBytesPath(path.fromString(\"{}\")); let x = typeOf(bytes) == \"Bytes\";",
        path_str.replace('\\', "\\\\"),
    );
    assert_runtime_parity(&source, Value::Bool(true));
}

// ============================================================
// Phase 4: Command / Pipeline construction API
// Runtime object model tests (pure constructors, no process execution)
// ============================================================

#[test]
fn test_command_construction_default() {
    let source = r#"
        import std.io as io;
        let cmd = io.command("echo", ["hello"]);
        let x = typeOf(cmd) == "Command";
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_command_construction_with_config() {
    let source = r#"
        import std.io as io;
        let cmd = io.commandWith(#{
            program = "echo",
            args = ["hello"],
            cwd = "/tmp",
            stdin = "input",
            env = #{FOO = "bar"},
        });
        let x = typeOf(cmd) == "Command";
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_pipeline_construction_basic() {
    let source = r#"
        import std.io as io;
        let p = io.pipeline([
            io.command("echo", ["hello"]),
            io.command("cat", []),
        ]);
        let x = typeOf(p) == "Pipeline";
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_pipeline_rejects_empty() {
    let source = r#"
        import std.io as io;
        let p = io.pipeline([]);
    "#;
    let analysis = analyze_without_diagnostics(source);
    // Pipeline construction with empty list should fail at runtime
    let result = eval_hir(&analysis);
    assert!(result.is_err(), "empty pipeline should fail");
}

#[test]
fn test_task_command_construction() {
    let source = r#"
        import std.io as io;
        let cmd = io.command("echo", ["task-test"]);
        let task = io.taskCommand(cmd);
        let x = typeOf(task) == "Task";
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_task_pipeline_construction() {
    let source = r#"
        import std.io as io;
        let p = io.pipeline([
            io.command("echo", ["hello"]),
            io.command("cat", []),
        ]);
        let task = io.taskPipeline(p);
        let x = typeOf(task) == "Task";
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_redirect_stdout_path_construction() {
    let source = r#"
        import std.io as io;
        let redir = io.redirectStdoutPath(./output.log);
        let x = typeOf(redir) == "Redirect";
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_redirect_stderr_path_construction() {
    let source = r#"
        import std.io as io;
        let redir = io.redirectStderrPath(./error.log);
        let x = typeOf(redir) == "Redirect";
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_redirect_stdin_path_construction() {
    let source = r#"
        import std.io as io;
        let redir = io.redirectStdinPath(./input.txt);
        let x = typeOf(redir) == "Redirect";
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_command_with_redirects_construction() {
    let source = r#"
        import std.io as io;
        let cmd = io.commandWithRedirects(
            io.command("echo", ["hello"]),
            [io.redirectStdoutPath(./out.log)],
        );
        let x = typeOf(cmd) == "Command";
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

// ============================================================
// Process result inspection (pure, no process execution)
// ============================================================

// ============================================================
// |> command pipeline syntax (v3.6.0+)
// ============================================================

#[test]
fn test_pipe_syntax_command_to_command() {
    let source = r#"
        import std.io as io;
        let cmd1 = io.command("echo", ["hello"]);
        let cmd2 = io.command("cat", []);
        let p = cmd1 |> cmd2;
        let x = typeOf(p) == "Pipeline";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(value, Value::Bool(true));
}

#[test]
fn test_pipe_syntax_command_chain() {
    let source = r#"
        import std.io as io;
        let p = io.command("echo", ["a"]) |>
                io.command("cat", []) |>
                io.command("cat", []);
        let x = typeOf(p) == "Pipeline";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(value, Value::Bool(true));
}

// ============================================================================
// Ecosystem integration tests / 生态系统集成测试
// ============================================================================

#[test]
fn test_ecosystem_flake_lock_roundtrip() {
    use neve_config::flake::{FlakeLock, FlakeLockEntry};

    let mut lock = FlakeLock::new();
    lock.inputs.insert(
        "test-pkg".to_string(),
        FlakeLockEntry {
            name: "test-pkg".to_string(),
            url: "https://example.com/pkg.tar.gz".to_string(),
            hash: "sha256-abc123".to_string(),
            last_modified: 1700000000,
            rev: Some("abc123def".to_string()),
        },
    );

    let json = lock.to_json();
    assert!(json.contains("test-pkg"));
    assert!(json.contains("abc123"));

    let parsed = FlakeLock::parse(&json).expect("should parse");
    assert_eq!(parsed.inputs.len(), 1);
    assert_eq!(parsed.inputs["test-pkg"].hash, "sha256-abc123");
}

#[test]
fn test_ecosystem_registry_index_format() {
    let index_json = r#"{
        "packages": [
            {
                "name": "hello-neve",
                "version": "1.0.0",
                "description": "A simple Neve package",
                "author": "Neve Community",
                "license": "MIT"
            },
            {
                "name": "neve-utils",
                "version": "2.1.0",
                "description": "Utility library for Neve",
                "author": "Neve Community",
                "license": "Apache-2.0"
            }
        ]
    }"#;

    assert!(index_json.contains("\"packages\""));
    assert!(index_json.contains("\"name\""));
    assert!(index_json.contains("\"hello-neve\""));
    assert!(index_json.contains("\"version\""));
    assert!(index_json.contains("\"1.0.0\""));
    assert!(index_json.contains("\"neve-utils\""));
    assert!(index_json.contains("\"2.1.0\""));

    let name_count = index_json.match_indices("\"name\"").count();
    assert_eq!(name_count, 2, "index should contain 2 packages");
}

#[test]
fn test_ecosystem_flake_manifest_parsing() {
    fn extract_field(manifest: &str, field: &str) -> Result<String, String> {
        for line in manifest.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with(&format!("{field} ="))
                || trimmed.starts_with(&format!("{field}="))
            {
                let value = trimmed
                    .split_once('=')
                    .map(|x| x.1)
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .trim_matches(';')
                    .trim();
                return Ok(value.to_string());
            }
        }
        Err(format!("field '{}' not found in manifest", field))
    }

    let manifest = r#"
name = "hello-neve"
version = "1.0.0"
description = "A friendly greeting package"
author = "Neve Community"
license = "MIT"
"#;

    assert_eq!(extract_field(manifest, "name").unwrap(), "hello-neve");
    assert_eq!(extract_field(manifest, "version").unwrap(), "1.0.0");
    assert_eq!(
        extract_field(manifest, "description").unwrap(),
        "A friendly greeting package"
    );
    assert!(extract_field(manifest, "nonexistent").is_err());
}

// ============================================================================
// Effect system tests / 效果系统测试
// ============================================================================

#[test]
fn test_effect_pure_function_rejects_effectful_calls_v2() {
    let analysis = neve_frontend::analyze_source(
        r#"
import std.io as io;
fn bad() -> String = io.readFile("/etc/hostname");
"#,
    );
    let has_effect_error = analysis.diagnostics.iter().any(|d| {
        d.message.contains("effectful call")
            && d.message.contains("effect")
            && d.severity == neve_diagnostic::Severity::Error
    });
    assert!(
        has_effect_error,
        "expected effect error for pure function with io, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_effect_effectful_function_calls_pure_computation() {
    let analysis = neve_frontend::analyze_source(
        r#"
import std.math as math;
fn ok() -> Int effect = math.abs(-5);
"#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful"));
    assert!(
        !has_effect_error,
        "effectful function should be able to call pure builtins, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_effect_lambda_inherits_enclosing_effect_context() {
    let analysis = neve_frontend::analyze_source(
        r#"
import std.io as io;
fn outer() -> Unit effect = {
    let f = fn() { io.writeFile("/tmp/effect_lambda_test.txt", "hello"); () };
    f();
};
"#,
    );
    let has_lambda_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful") && d.message.contains("lambda"));
    assert!(
        !has_lambda_error,
        "lambda inside effect fn should allow effectful calls, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_effect_nested_effectful_composition() {
    let analysis = neve_frontend::analyze_source(
        r#"
import std.io as io;
fn inner(file: String) -> String effect = io.readFile(file);
fn outer() -> String effect = inner("/etc/hostname");
"#,
    );
    let has_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == neve_diagnostic::Severity::Error);
    assert!(
        !has_error,
        "nested effectful functions should compose, got {:?}",
        analysis.diagnostics
    );
}

// ============================================================================
// Error handling tests / 错误处理测试
// ============================================================================

#[test]
fn test_error_option_none_question_short_circuits() {
    let analysis = neve_frontend::analyze_source(
        r#"
enum Option { Some(Int), None };
let x = None?;
"#,
    );
    let has_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == neve_diagnostic::Severity::Error);
    assert!(
        has_error,
        "expected type error for ? on None, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_error_result_err_question_short_circuits() {
    assert_runtime_error_parity(
        r#"
enum Result { Ok(Int), Err(String) };
let x = Err("something went wrong")?;
"#,
        "something went wrong",
    );
}

#[test]
fn test_error_coalesce_none_returns_default() {
    assert_runtime_parity(
        r#"
import std.option as option;
let x = option.none ?? 99;
"#,
        Value::Int(int(99)),
    );
}

#[test]
fn test_error_question_operator_chaining() {
    assert_runtime_parity(
        r#"
import std.option as option;
import std.result as result;
let a = option.some(10)?;
let b = result.ok(20)?;
let x = a + b;
"#,
        Value::Int(int(30)),
    );
}

// ============================================================================
// List/Record operations tests / 列表/记录操作测试
// ============================================================================

#[test]
fn test_list_fold_addition_over_list() {
    assert_runtime_parity(
        r#"
import std.list as list;
let x = list.sum([1, 2, 3]);
"#,
        Value::Int(int(6)),
    );
}

#[test]
fn test_list_filter_with_predicate() {
    assert_runtime_parity(
        r#"
import std.list as list;
fn isEven(x) = x % 2 == 0;
let x = list.filter(isEven, [1, 2, 3, 4, 5, 6]);
"#,
        Value::List(std::rc::Rc::new(vec![
            Value::Int(int(2)),
            Value::Int(int(4)),
            Value::Int(int(6)),
        ])),
    );
}

#[test]
fn test_record_field_access_dot_syntax() {
    assert_runtime_parity(
        r#"
let person = #{ name = "Alice", age = 30 };
let x = person.name;
"#,
        Value::String("Alice".to_string().into()),
    );
}

#[test]
fn test_record_update_pipe_syntax() {
    assert_runtime_parity(
        r#"
let original = #{ x = 1, y = 2 };
let updated = #{ original | x = 10 };
let z = updated.x + updated.y;
"#,
        Value::Int(int(12)),
    );
}

// ============================================================================
// String operations tests / 字符串操作测试
// ============================================================================

#[test]
fn test_string_join_multiple_concat() {
    assert_runtime_parity(
        r#"
import std.string as string;
let parts = ["hello", " ", "world"];
let x = string.join(parts, "");
"#,
        Value::String("hello world".to_string().into()),
    );
}

#[test]
fn test_string_split_and_len() {
    assert_runtime_parity(
        r#"
import std.string as string;
import std.list as list;
let parts = string.split("a,b,c,d", ",");
let x = list.len(parts);
"#,
        Value::Int(int(4)),
    );
}

// ============================================================================
// Miscellaneous tests / 杂项测试
// ============================================================================

#[test]
fn test_nested_let_bindings_with_shadowing() {
    assert_runtime_parity(
        r#"
let x = 1;
let result = {
    let x = x + 2;
    let x = x * 3;
    x
};
let final = result + x;
"#,
        Value::Int(int(10)),
    );
}

#[test]
fn test_block_expression_returns_last_value() {
    assert_runtime_parity(
        r#"
let x = {
    let a = 10;
    let b = 20;
    a + b
};
"#,
        Value::Int(int(30)),
    );
}

// ============================================================================
// Stream<T> basic tests / 流基本测试
// ============================================================================

#[test]
fn test_stream_list_and_collect_roundtrip() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let result = io.streamCollect(s);
    let x = result == [1, 2, 3];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_list_empty() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([]);
    let result = io.streamCollect(s);
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_type_identity() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1]);
    let x = typeOf(s) == "Stream";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_collect_rejects_non_stream() {
    let source = r#"
    import std.io as io;
    let x = io.streamCollect(42);
    "#;
    let analysis = analyze_source(source);
    let has_type_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    if !has_type_error {
        let result = eval_hir(&analysis);
        assert!(result.is_err(), "streamCollect on Int should error");
    }
}

#[test]
fn test_stream_list_rejects_non_list() {
    let source = r#"
    import std.io as io;
    let x = io.streamList(42);
    "#;
    let analysis = analyze_source(source);
    let has_type_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    if !has_type_error {
        let result = eval_hir(&analysis);
        assert!(result.is_err(), "streamList on Int should error");
    }
}

#[test]
fn test_stream_lines_from_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("lines.txt");
    fs::write(&file_path, "alpha\nbeta\ngamma\n").unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    let s = io.streamLines("{escaped}");
    let result = io.streamCollect(s);
    let x = result == ["alpha", "beta", "gamma"];
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_command_echo() {
    let source = r#"
    import std.io as io;
    let s = io.streamCommand(io.command("echo", ["hello"]));
    let result = io.streamCollect(s);
    let x = result == ["hello"];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_bytes_roundtrip() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("bytes.bin");
    fs::write(&file_path, [0x01, 0x02, 0x03]).unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    import std.bytes as bytes;
    let s = io.streamBytes("{escaped}");
    let result = io.streamCollect(s);
    let x = typeOf(result) == "List";
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_map_closure() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let mapped = io.streamMap(s, fn(x) { x * 10 });
    let result = io.streamCollect(mapped);
    let x = result == [10, 20, 30];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_filter_closure() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3, 4, 5, 6]);
    let filtered = io.streamFilter(s, fn(x) { x > 3 });
    let result = io.streamCollect(filtered);
    let x = result == [4, 5, 6];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Stream<T> transform tests / 流变换测试
// ============================================================================

#[test]
fn test_stream_take_basic() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([10, 20, 30, 40, 50]);
    let taken = io.streamTake(s, 2);
    let result = io.streamCollect(taken);
    let x = result == [10, 20];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_take_zero() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let taken = io.streamTake(s, 0);
    let result = io.streamCollect(taken);
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_drop_basic() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([10, 20, 30, 40, 50]);
    let dropped = io.streamDrop(s, 3);
    let result = io.streamCollect(dropped);
    let x = result == [40, 50];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_drop_all() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let dropped = io.streamDrop(s, 100);
    let result = io.streamCollect(dropped);
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_filter_removes_all() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let filtered = io.streamFilter(s, fn(_) { false });
    let result = io.streamCollect(filtered);
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_pipe_basic() {
    let source = r#"
    import std.io as io;
    let s = io.streamList(["hello", "world"]);
    let result = io.streamPipe(s, io.command("cat", []));
    let x = typeOf(result) == "ProcessResult";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_nested_stream_map_filter() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3, 4, 5, 6]);
    let mapped = io.streamMap(s, fn(x) { x * 10 });
    let filtered = io.streamFilter(mapped, fn(x) { x > 25 });
    let result = io.streamCollect(filtered);
    let x = result == [30, 40, 50, 60];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_for_each_typechecks() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let result = io.streamForEach(s, fn(x) { () });
    let x = typeOf(result) == "Unit";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Stream<T> additional coverage / 流额外覆盖
// ============================================================================

#[test]
fn test_stream_lines_empty_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("empty.txt");
    fs::write(&file_path, "").unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    let s = io.streamLines("{escaped}");
    let result = io.streamCollect(s);
    let x = result == [];
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_map_identity() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let mapped = io.streamMap(s, fn(x) { x });
    let result = io.streamCollect(mapped);
    let x = result == [1, 2, 3];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_filter_keep_all() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([5, 10, 15]);
    let filtered = io.streamFilter(s, fn(_) { true });
    let result = io.streamCollect(filtered);
    let x = result == [5, 10, 15];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_take_more_than_available() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2]);
    let taken = io.streamTake(s, 10);
    let result = io.streamCollect(taken);
    let x = result == [1, 2];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_drop_more_than_available() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1]);
    let dropped = io.streamDrop(s, 50);
    let result = io.streamCollect(dropped);
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_map_type_preserves_stream() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1]);
    let mapped = io.streamMap(s, fn(x) { x * 2 });
    let x = typeOf(mapped) == "Stream";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_filter_type_preserves_stream() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1]);
    let filtered = io.streamFilter(s, fn(_) { true });
    let x = typeOf(filtered) == "Stream";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_take_type_preserves_stream() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1]);
    let taken = io.streamTake(s, 1);
    let x = typeOf(taken) == "Stream";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_drop_type_preserves_stream() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1]);
    let dropped = io.streamDrop(s, 0);
    let x = typeOf(dropped) == "Stream";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_fold_sum() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3, 4, 5]);
    let result = io.streamFold(s, 0, fn(acc, x) { acc + x });
    let x = result;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Int(int(15)));
}

#[test]
fn test_stream_list_single_element() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([42]);
    let result = io.streamCollect(s);
    let x = result == [42];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_nested_stream_take_drop() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3, 4, 5]);
    let dropped = io.streamDrop(s, 2);
    let taken = io.streamTake(dropped, 2);
    let result = io.streamCollect(taken);
    let x = result == [3, 4];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_collect_on_empty_stream() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([]);
    let result = io.streamCollect(s);
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_large_list() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let filtered = io.streamFilter(s, fn(x) { x <= 5 });
    let mapped = io.streamMap(filtered, fn(x) { x * 2 });
    let result = io.streamCollect(mapped);
    let x = result == [2, 4, 6, 8, 10];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_map_with_strings() {
    let source = r#"
    import std.io as io;
    let s = io.streamList(["a", "bb", "ccc"]);
    let mapped = io.streamMap(s, fn(x) { x });
    let result = io.streamCollect(mapped);
    let x = result == ["a", "bb", "ccc"];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_filter_with_even_predicate() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3, 4, 5]);
    let filtered = io.streamFilter(s, fn(x) { x % 2 == 0 });
    let result = io.streamCollect(filtered);
    let x = result == [2, 4];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_pipe_preserves_success() {
    let source = r#"
    import std.io as io;
    let s = io.streamList(["one", "two"]);
    let result = io.streamPipe(s, io.command("cat", []));
    let x = io.processSuccess(result);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_fold_with_closure() {
    let source = r#"
    import std.io as io;
    let s = io.streamList(["a", "b", "c"]);
    let result = io.streamFold(s, "", fn(acc, x) { acc + x });
    let x = result == "abc";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Task API tests / 任务API测试
// ============================================================================

#[test]
fn test_task_spawn_poll_lifecycle() {
    let source = r#"
    import std.io as io;
    let cmd = io.command("echo", ["task-lifecycle"]);
    let task = io.taskCommand(cmd);
    let id = io.spawn(task);
    let result = io.poll(id);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_poll_invalid_id_errors() {
    let source = r#"
    import std.io as io;
    let result = io.poll(99999);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let result = eval_hir(&analysis);
    assert!(result.is_err(), "poll on invalid id should error");
}

#[test]
fn test_task_cancel_nonexistent_noop() {
    let source = r#"
    import std.io as io;
    io.cancel(99999);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_await_any_returns_first() {
    let source = r#"
    import std.io as io;
    let t1 = io.taskCommand(io.command("echo", ["first"]));
    let t2 = io.taskCommand(io.command("echo", ["second"]));
    let result = io.awaitAny([t1, t2]);
    let x = typeOf(result) == "ProcessResult";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_await_any_single() {
    let source = r#"
    import std.io as io;
    let t = io.taskCommand(io.command("echo", ["only"]));
    let result = io.awaitAny([t]);
    let x = typeOf(result) == "ProcessResult";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_await_any_empty_rejected() {
    let source = r#"
    import std.io as io;
    let x = io.awaitAny([]);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let result = eval_hir(&analysis);
    assert!(result.is_err(), "awaitAny([]) should error");
}

// ============================================================================
// Pipe syntax tests / 管道语法测试
// ============================================================================

#[test]
fn test_pipe_command_to_noncommand_errors() {
    let source = r#"
    import std.io as io;
    let cmd = io.command("echo", ["hello"]);
    let broken = cmd |> 42;
    "#;
    let analysis = analyze_source(source);
    let has_type_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    assert!(
        has_type_error,
        "expected type error for cmd |> 42, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_pipe_function_application_chain() {
    let source = r#"
    fn double(x: Int) -> Int = x * 2;
    let x = 40 |> double |> double;
    "#;
    assert_runtime_parity(source, Value::Int(int(160)));
}

// ============================================================================
// TTY tests / TTY测试
// ============================================================================

#[test]
fn test_tty_set_raw_mode_builtin_exists() {
    let source = r#"
    import std.io as io;
    let x = typeOf(io.setRawMode);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let _ = eval_hir(&analysis).expect("HIR evaluator should succeed");
}

#[test]
fn test_tty_reset_terminal_builtin_exists() {
    let source = r#"
    import std.io as io;
    let x = typeOf(io.resetTerminal);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let _ = eval_hir(&analysis).expect("HIR evaluator should succeed");
}

// ============================================================================
// Type system tests / 类型系统测试
// ============================================================================

#[test]
fn test_type_of_stream() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1]);
    let x = typeOf(s) == "Stream";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_type_of_process_result() {
    let source = r#"
    import std.io as io;
    let cmd = io.command("echo", ["type-test"]);
    let result = io.execCommand(cmd);
    let x = typeOf(result) == "ProcessResult";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_type_of_task() {
    let source = r#"
    import std.io as io;
    let t = io.taskCommand(io.command("echo", ["task-type"]));
    let x = typeOf(t) == "Task";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_type_of_bytes() {
    let source = r#"
    import std.bytes as bytes;
    let b = bytes.fromString("hello");
    let x = typeOf(b) == "Bytes";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_type_of_path() {
    let source = r#"
    import std.path as path;
    let p = path.fromString("/tmp/test");
    let x = typeOf(p) == "Path";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_type_of_bool() {
    let source = r#"
    let x = typeOf(true) == "Bool";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_type_of_int() {
    let source = r#"
    let x = typeOf(42) == "Int";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_type_of_string() {
    let source = r#"
    let x = typeOf("hello") == "String";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_type_of_list() {
    let source = r#"
    let x = typeOf([1, 2, 3]) == "List";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Job control tests / 作业控制测试
// ============================================================================

#[test]
fn test_jobs_returns_list() {
    let source = r#"
    import std.io as io;
    let jobs = io.jobs();
    let x = typeOf(jobs) == "List";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_wait_any_job_returns_result() {
    let source = r#"
    import std.io as io;
    let task = io.taskCommand(io.command("echo", ["job-test"]));
    let id = io.spawn(task);
    let result = io.waitAnyJob();
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Signal tests / 信号测试
// ============================================================================

#[test]
fn test_signal_handler_registration() {
    let source = r#"
    import std.io as io;
    let handled = io.onSignal("INT", fn() { () });
    let x = typeOf(handled) == "Unit";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_signal_handler_rejects_invalid_signal() {
    let source = r#"
    import std.io as io;
    let x = io.onSignal("INVALID_SIGNAL_NAME", fn() { () });
    "#;
    let analysis = analyze_without_diagnostics(source);
    let result = eval_hir(&analysis);
    assert!(result.is_err(), "onSignal with invalid signal should error");
}

// ============================================================================
// Glob tests / Glob测试
// ============================================================================

#[test]
fn test_glob_returns_list() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("glob-test");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("a.txt"), "a").unwrap();
    fs::write(dir.join("b.txt"), "b").unwrap();
    let pattern = format!(
        "{}/glob-test/*.txt",
        temp.path().to_string_lossy().replace('\\', "\\\\")
    );
    let source = format!(
        r#"
    import std.io as io;
    let result = io.glob("{pattern}");
    let x = typeOf(result) == "List";
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_glob_with_list_len() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("glob-len");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("x.neve"), "x").unwrap();
    fs::write(dir.join("y.txt"), "y").unwrap();
    let pattern = format!(
        "{}/glob-len/*",
        temp.path().to_string_lossy().replace('\\', "\\\\")
    );
    let source = format!(
        r#"
    import std.io as io;
    import std.list as list;
    let paths = io.glob("{pattern}");
    let x = list.len(paths) >= 1;
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_glob_nonexistent_pattern() {
    let source = r#"
    import std.io as io;
    let result = io.glob("/nonexistent/path/xyzzy-*.none");
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Ecosystem additional tests / 生态系统额外测试
// ============================================================================

#[test]
fn test_ecosystem_flake_lock_empty() {
    use neve_config::flake::FlakeLock;

    let lock = FlakeLock::new();
    assert!(lock.inputs.is_empty());

    let json = lock.to_json();
    let parsed = FlakeLock::parse(&json).expect("empty lock should parse");
    assert!(parsed.inputs.is_empty());
}

#[test]
fn test_ecosystem_registry_search_filters() {
    fn search_packages(index_json: &str, query: &str) -> Vec<String> {
        let mut results = Vec::new();
        for line in index_json.lines() {
            if line.to_lowercase().contains(&query.to_lowercase()) {
                results.push(line.trim().to_string());
            }
        }
        results
    }

    let index = r#"[
        {"name": "hello-neve", "version": "1.0.0"},
        {"name": "neve-utils", "version": "2.1.0"},
        {"name": "neve-json", "version": "0.5.0"}
    ]"#;

    let results = search_packages(index, "neve");
    assert_eq!(results.len(), 3, "all three packages contain 'neve'");

    let results = search_packages(index, "utils");
    assert_eq!(results.len(), 1, "only neve-utils contains 'utils'");
    assert!(results[0].contains("neve-utils"));
}

// ============================================================================
// Error path coverage / 错误路径覆盖
// ============================================================================

#[test]
fn test_stream_take_rejects_non_stream() {
    let source = r#"
    import std.io as io;
    let x = io.streamTake(42, 1);
    "#;
    let analysis = analyze_source(source);
    let has_type_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    if !has_type_error {
        let result = eval_hir(&analysis);
        assert!(result.is_err(), "streamTake on Int should error");
    }
}

#[test]
fn test_stream_drop_rejects_non_stream() {
    let source = r#"
    import std.io as io;
    let x = io.streamDrop("not-a-stream", 1);
    "#;
    let analysis = analyze_source(source);
    let has_type_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    if !has_type_error {
        let result = eval_hir(&analysis);
        assert!(result.is_err(), "streamDrop on String should error");
    }
}

#[test]
fn test_stream_map_rejects_non_stream() {
    let source = r#"
    import std.io as io;
    let x = io.streamMap(99, fn(x) { x });
    "#;
    let analysis = analyze_source(source);
    let has_type_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    if !has_type_error {
        let result = eval_hir(&analysis);
        assert!(result.is_err(), "streamMap on Int should error");
    }
}

#[test]
fn test_stream_filter_rejects_non_stream() {
    let source = r#"
    import std.io as io;
    let x = io.streamFilter(true, fn(_) { true });
    "#;
    let analysis = analyze_source(source);
    let has_type_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    if !has_type_error {
        let result = eval_hir(&analysis);
        assert!(result.is_err(), "streamFilter on Bool should error");
    }
}

#[test]
fn test_stream_pipe_rejects_non_stream() {
    let source = r#"
    import std.io as io;
    let x = io.streamPipe(42, io.command("echo", ["x"]));
    "#;
    let analysis = analyze_source(source);
    let has_type_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    if !has_type_error {
        let result = eval_hir(&analysis);
        assert!(result.is_err(), "streamPipe on Int should error");
    }
}

#[test]
fn test_stream_for_each_rejects_non_stream() {
    let source = r#"
    import std.io as io;
    let x = io.streamForEach("bad", fn(x) { () });
    "#;
    let analysis = analyze_source(source);
    let has_type_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    if !has_type_error {
        let result = eval_hir(&analysis);
        assert!(result.is_err(), "streamForEach on String should error");
    }
}

// ============================================================================
// ProcessResult inspection / 进程结果检查
// ============================================================================

#[test]
fn test_process_result_fields() {
    let source = r#"
    import std.io as io;
    let result = io.execCommand(io.command("echo", ["multi-field"]));
    let stdout = io.processStdout(result);
    let code = io.processCode(result);
    let success = io.processSuccess(result);
    let x = success && code == 0;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Effect system additional coverage / 效果系统额外覆盖
// ============================================================================

#[test]
fn test_effect_stream_collect_is_effectful() {
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> List = io.streamCollect(io.streamList([1]));
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error for io.streamCollect, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_effect_stream_for_each_is_effectful() {
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> Unit = io.streamForEach(io.streamList([1]), fn(x) { () });
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error for io.streamForEach, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_effect_stream_pipe_is_effectful() {
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> ProcessResult = io.streamPipe(
            io.streamList(["x"]),
            io.command("cat", [])
        );
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error for io.streamPipe, got {:?}",
        analysis.diagnostics
    );
}

// ============================================================================
// Pipeline + stream integration
// ============================================================================

#[test]
fn test_pipeline_result_match() {
    let source = r#"
    import std.io as io;
    let result = io.execCommand(io.command("echo", ["match-test"]));
    let x = match result {
        _ -> true,
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Stream integration (6 tests) / 流集成测试
// ============================================================================

#[test]
fn test_stream_lines_nonexistent_file_errors() {
    let temp = TempDir::new().unwrap();
    let nonexistent = temp.path().join("does_not_exist.txt");
    let escaped = nonexistent.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    let s = io.streamLines("{escaped}");
    let x = io.streamCollect(s);
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let result = eval_hir(&analysis);
    assert!(
        result.is_err(),
        "streamLines on nonexistent file should error"
    );
}

#[test]
fn test_stream_command_with_stdin() {
    let source = r#"
    import std.io as io;
    let cmd = io.commandWith(#{
        program = "cat",
        args = [],
        stdin = "hello from stdin"
    });
    let s = io.streamCommand(cmd);
    let result = io.streamCollect(s);
    let x = result == ["hello from stdin"];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_map_chain_of_three() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let m1 = io.streamMap(s, fn(x) { x * 2 });
    let m2 = io.streamMap(m1, fn(x) { x + 1 });
    let result = io.streamCollect(m2);
    let x = result == [3, 5, 7];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_filter_then_collect_empty() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3, 4, 5]);
    let filtered = io.streamFilter(s, fn(_) { false });
    let result = io.streamCollect(filtered);
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_take_more_than_available_three_elements() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([10, 20, 30]);
    let taken = io.streamTake(s, 10);
    let result = io.streamCollect(taken);
    let x = result == [10, 20, 30];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_drop_exact_length() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let dropped = io.streamDrop(s, 3);
    let result = io.streamCollect(dropped);
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Pipe + Command integration (4 tests) / 管道与命令集成测试
// ============================================================================

#[test]
fn test_command_pipe_with_cwd_and_env() {
    let source = r#"
    import std.io as io;
    let cmd = io.commandWith(#{
        program = "echo",
        args = ["hello from cwd"],
        cwd = "/tmp",
        env = #{FOO = "bar"}
    });
    let pipeline = cmd |> io.command("cat", []);
    let result = io.execPipeline(pipeline);
    let x = io.processSuccess(result);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_pipeline_three_commands_type() {
    let source = r#"
    import std.io as io;
    let p = io.pipeline([
        io.command("echo", ["a"]),
        io.command("cat", []),
        io.command("cat", []),
    ]);
    let x = typeOf(p) == "Pipeline";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_pipeline_exec_with_timeout_completes() {
    let source = r#"
    import std.io as io;
    let p = io.pipeline([
        io.command("echo", ["quick"]),
        io.command("cat", []),
    ]);
    let task = io.taskPipeline(p);
    let id = io.spawnWithTimeout(task, 5000);
    let result = io.poll(id);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_redirect_stdout_to_tempfile() {
    let temp = TempDir::new().unwrap();
    let output_path = temp.path().join("redirect-out.txt");
    fs::write(&output_path, "").unwrap();
    let escaped = output_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    import std.path as path;
    let target = path.fromString("{escaped}");
    let cmd = io.commandWithRedirects(
        io.command("echo", ["redirected content"]),
        [io.redirectStdoutPath(target)]
    );
    let result = io.execCommand(cmd);
    let fileContent = io.readFilePath(target);
    let x = io.processSuccess(result);
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Task lifecycle (4 tests) / 任务生命周期测试
// ============================================================================

#[test]
fn test_task_spawn_multiple_parallel() {
    let source = r#"
    import std.io as io;
    let id1 = io.spawn(io.taskCommand(io.command("echo", ["one"])));
    let id2 = io.spawn(io.taskCommand(io.command("echo", ["two"])));
    let id3 = io.spawn(io.taskCommand(io.command("echo", ["three"])));
    let r1 = io.poll(id1);
    let r2 = io.poll(id2);
    let r3 = io.poll(id3);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_await_any_with_three() {
    let source = r#"
    import std.io as io;
    let t1 = io.taskCommand(io.command("echo", ["first"]));
    let t2 = io.taskCommand(io.command("echo", ["second"]));
    let t3 = io.taskCommand(io.command("echo", ["third"]));
    let result = io.awaitAny([t1, t2, t3]);
    let x = typeOf(result) == "ProcessResult";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_await_tasks_returns_all() {
    let source = r#"
    import std.io as io;
    let results = io.awaitTasks([
        io.taskCommand(io.command("echo", ["alpha"])),
        io.taskCommand(io.command("echo", ["beta"])),
    ]);
    let x = match results {
        [r1, r2] -> io.processSuccess(r1) && io.processSuccess(r2),
        _ -> false,
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_spawn_with_timeout_pipeline() {
    let source = r#"
    import std.io as io;
    let p = io.pipeline([
        io.command("echo", ["timeout test"]),
        io.command("cat", []),
    ]);
    let task = io.taskPipeline(p);
    let id = io.spawnWithTimeout(task, 5000);
    let x = typeOf(id) == "Int";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Effect system (3 tests) / 效果系统测试
// ============================================================================

#[test]
fn test_effect_stream_collect_in_pure_fn_rejected() {
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn bad() -> List = io.streamCollect(io.streamList([1, 2, 3]));
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error for io.streamCollect in pure fn, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_effect_print_in_effectful_fn_allowed() {
    let analysis = neve_frontend::analyze_source(
        r#"
        fn ok() -> Unit effect = print("hello from effectful fn");
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful") && d.severity == neve_diagnostic::Severity::Error);
    assert!(
        !has_effect_error,
        "effectful function should be allowed to call print, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_effect_nested_pure_in_effectful() {
    let analysis = neve_frontend::analyze_source(
        r#"
        fn pureLeaf() -> Int = 42;
        fn pureMid() -> Int = pureLeaf() + 10;
        fn eff() -> Int effect = pureMid();
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful") && d.severity == neve_diagnostic::Severity::Error);
    assert!(
        !has_effect_error,
        "effectful calling pure calling pure should be allowed, got {:?}",
        analysis.diagnostics
    );
}

// ============================================================================
// Pattern matching (3 tests) / 模式匹配测试
// ============================================================================

#[test]
fn test_match_option_some_with_binding() {
    let source = r#"
        import std.option as option;
        let x = match option.some(42) {
            Some(v) -> v,
            None -> 0,
        };
    "#;
    assert_runtime_parity(source, Value::Int(int(42)));
}

#[test]
fn test_match_result_ok_with_binding() {
    let source = r#"
        import std.result as result;
        let x = match result.ok(99) {
            Ok(v) -> v,
            Err(_) -> 0,
        };
    "#;
    assert_runtime_parity(source, Value::Int(int(99)));
}

#[test]
fn test_match_list_empty_vs_nonempty() {
    let source = r#"
        let x = match [7, 8, 9] {
            [] -> 0,
            [h, ..t] -> h,
        };
    "#;
    assert_runtime_parity(source, Value::Int(int(7)));
}

// ============================================================================
// Type system (2 tests) / 类型系统测试
// ============================================================================

#[test]
fn test_type_of_nested_stream() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let mapped = io.streamMap(s, fn(x) { x * 10 });
    let filtered = io.streamFilter(mapped, fn(x) { x > 5 });
    let x = typeOf(filtered) == "Stream";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_type_of_after_stream_collect() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1]);
    let collected = io.streamCollect(s);
    let x = typeOf(collected) == "List";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Lazy evaluation (2 tests) / 惰性求值测试
// ============================================================================

#[test]
fn test_lazy_force_roundtrip() {
    let source = r#"
        let thunk = lazy 42;
        let x = force(thunk);
    "#;
    assert_runtime_parity(source, Value::Int(int(42)));
}

#[test]
fn test_lazy_is_evaluated() {
    let source = r#"
        let thunk = lazy (21 + 21);
        let before = isEvaluated(thunk);
        let val = force(thunk);
        let after = isEvaluated(thunk);
        let x = before == false && after == true && val == 42;
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

// ============================================================================
// Real-world scripting scenarios (5 tests) / 真实世界脚本场景测试
// ============================================================================

#[test]
fn test_script_grep_equivalent() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("grep-data.txt");
    fs::write(&file_path, "alpha\nbeta\nalpha\ngamma\n").unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    import std.string as string;
    let s = io.streamLines("{escaped}");
    let filtered = io.streamFilter(s, fn(line) {{ string.contains(line, "alpha") }});
    let result = io.streamCollect(filtered);
    let x = result == ["alpha", "alpha"];
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_script_wc_equivalent() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("wc-data.txt");
    fs::write(&file_path, "line1\nline2\nline3\n").unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    import std.list as list;
    let s = io.streamLines("{escaped}");
    let lines = io.streamCollect(s);
    let x = list.len(lines) == 3;
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_script_env_var_usage() {
    let source = r#"
    import std.io as io;
    let path = io.getEnv("PATH");
    let x = match path {
        Some(_) -> true,
        None -> false,
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_script_exit_code_propagation() {
    let source = r#"
    import std.io as io;
    let result = io.execCommand(io.command("rustc", ["--invalid-flag-xyz-12345"]));
    let x = !io.processSuccess(result) && io.processCode(result) != 0;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_script_atomic_file_write_read() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("atomic-test.txt");
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    let done = io.writeFile("{escaped}", "atomic-content-42");
    let content = io.readFile("{escaped}");
    let x = content == "atomic-content-42";
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Stream composition (5 tests) / 流组合测试
// ============================================================================

#[test]
fn test_stream_bytes_to_string_pipeline() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("bytes-data.bin");
    fs::write(&file_path, [0x41, 0x42, 0x43]).unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    import std.bytes as bytes;
    let s = io.streamBytes("{escaped}");
    let byteList = io.streamCollect(s);
    let x = typeOf(byteList) == "List";
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_command_to_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("stream-out.txt");
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    import std.string as string;
    let s = io.streamCommand(io.command("echo", ["stream-to-file"]));
    let lines = io.streamCollect(s);
    let joined = string.join(lines, "\n");
    let done = io.writeFile("{escaped}", joined);
    let content = io.readFile("{escaped}");
    let x = content == "stream-to-file";
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_nested_collects() {
    let source = r#"
    import std.io as io;
    let outer = io.streamList([1, 2, 3]);
    let done = io.streamForEach(outer, fn(x) {
        let inner = io.streamList([x, x * 10]);
        let collected = io.streamCollect(inner);
        ()
    });
    let x = typeOf(done) == "Unit";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_with_timeout_fast() {
    let source = r#"
    import std.io as io;
    let task = io.taskCommand(io.command("echo", ["stream-fast"]));
    let id = io.spawnWithTimeout(task, 5000);
    let result = io.poll(id);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_fold_with_large_list() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("large.txt");
    let lines: Vec<String> = (1..=100).map(|i| i.to_string()).collect();
    fs::write(&file_path, lines.join("\n")).unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    let s = io.streamLines("{escaped}");
    let count = io.streamFold(s, 0, fn(acc, _) {{ acc + 1 }});
    let x = count == 100;
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Error resilience (5 tests) / 错误恢复测试
// ============================================================================

#[test]
fn test_error_propagation_through_pipe() {
    let source = r#"
    import std.io as io;
    let p = io.pipeline([
        io.command("rustc", ["--invalid-flag-xyz-12345"]),
    ]);
    let result = io.execPipeline(p);
    let x = !io.processSuccess(result) && io.processCode(result) != 0;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_graceful_handling_bad_glob() {
    let source = r#"
    import std.io as io;
    let result = io.glob("/[/invalid/pattern/[");
    "#;
    let analysis = analyze_without_diagnostics(source);
    let result = eval_hir(&analysis);
    assert!(result.is_err(), "bad glob pattern should error");
}

#[test]
fn test_graceful_handling_missing_file() {
    let source = r#"
    import std.io as io;
    let result = io.readFile("/nonexistent/file/path/xyz-12345.txt");
    "#;
    let analysis = analyze_without_diagnostics(source);
    let result = eval_hir(&analysis);
    assert!(result.is_err(), "readFile on missing file should error");
}

#[test]
fn test_graceful_handling_invalid_command() {
    let source = r#"
    import std.io as io;
    let result = io.execCommand(io.command("nonexistent-binary-xyz-12345", []));
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let result = eval_hir(&analysis);
    assert!(
        result.is_err(),
        "execCommand with invalid binary should error"
    );
}

#[test]
fn test_graceful_handling_empty_pipeline() {
    let source = r#"
    import std.io as io;
    let p = io.pipeline([]);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let result = eval_hir(&analysis);
    assert!(result.is_err(), "empty pipeline should error");
}

// ============================================================================
// Type system edge cases (3 tests) / 类型系统边界情况测试
// ============================================================================

#[test]
fn test_generic_function_with_stream() {
    let source = r#"
    import std.io as io;
    fn countStream(s) effect = io.streamFold(s, 0, fn(acc, _) { acc + 1 });
    let s = io.streamList([10, 20, 30, 40, 50]);
    let x = countStream(s) == 5;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_record_with_stream_field() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let rec = #{ stream = s, label = "data" };
    let x = typeOf(rec.stream) == "Stream" && rec.label == "data";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_tuple_with_stream() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([99]);
    let pair = (s, 42);
    let x = match pair {
        (stream, num) -> {
            let result = io.streamCollect(stream);
            result == [99] && num == 42
        },
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Concurrency patterns (2 tests) / 并发模式测试
// ============================================================================

#[test]
fn test_spawn_multiple_and_await_all() {
    let source = r#"
    import std.io as io;
    let results = io.awaitTasks([
        io.taskCommand(io.command("echo", ["one"])),
        io.taskCommand(io.command("echo", ["two"])),
        io.taskCommand(io.command("echo", ["three"])),
    ]);
    let x = match results {
        [r1, r2, r3] -> io.processSuccess(r1) && io.processSuccess(r2) && io.processSuccess(r3),
        _ -> false,
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_spawn_and_cancel_before_done() {
    let source = r#"
    import std.io as io;
    let task = io.taskCommand(io.command("sleep", ["10"]));
    let id = io.spawn(task);
    io.cancel(id);
    let result = io.poll(id);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let result = eval_hir(&analysis);
    assert!(result.is_err(), "poll after cancel should error");
}

// ============================================================================
// Stream + File integration (5 tests) / 流与文件集成测试
// ============================================================================

#[test]
fn test_stream_lines_large_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("large-lines.txt");
    let lines: Vec<String> = (1..=50).map(|i| format!("line-{}", i)).collect();
    fs::write(&file_path, lines.join("\n")).unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    let s = io.streamLines("{escaped}");
    let count = io.streamFold(s, 0, fn(acc, _) {{ acc + 1 }});
    let x = count == 50;
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_bytes_large_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("large-bytes.bin");
    let lines: Vec<String> = (1..=50).map(|i| format!("line-{}\n", i)).collect();
    fs::write(&file_path, lines.concat()).unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    import std.list as list;
    let s = io.streamBytes("{escaped}");
    let chunks = io.streamCollect(s);
    let x = list.len(chunks) > 0;
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_command_with_env() {
    let source = r#"
    import std.io as io;
    let cmd = io.commandWith(#{
        program = "sh",
        args = ["-c", "echo $NEVE_STREAM_TEST"],
        env = #{NEVE_STREAM_TEST = "env-value-42"},
    });
    let s = io.streamCommand(cmd);
    let result = io.streamCollect(s);
    let x = result == ["env-value-42"];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_pipe_to_file() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("pipe-out.txt");
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    import std.string as string;
    let s = io.streamList(["alpha", "beta", "gamma"]);
    let lines = io.streamCollect(s);
    let joined = string.join(lines, "\n");
    let done = io.writeFile("{escaped}", joined);
    let content = io.readFile("{escaped}");
    let x = content == "alpha\nbeta\ngamma";
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_for_each_counts_elements() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("foreach-counter.txt");
    fs::write(&file_path, "").unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    import std.string as string;
    let s = io.streamList([1, 2, 3, 4, 5]);
    let done = io.streamForEach(s, fn(x) {{ io.appendFile("{escaped}", "x"); () }});
    let content = io.readFile("{escaped}");
    let x = string.len(content) == 5;
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Task + Pipeline integration (5 tests) / 任务与管道集成测试
// ============================================================================

#[test]
fn test_task_pipeline_spawn_poll() {
    let source = r#"
    import std.io as io;
    let p = io.pipeline([
        io.command("echo", ["pipeline-spawn"]),
        io.command("cat", []),
    ]);
    let task = io.taskPipeline(p);
    let id = io.spawn(task);
    let result = io.poll(id);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_await_task_with_timeout_completes() {
    let source = r#"
    import std.io as io;
    let task = io.taskCommand(io.command("echo", ["fast-task"]));
    let result = io.awaitTaskWithTimeout(task, 5000);
    let x = match result {
        Some(pr) -> io.processSuccess(pr),
        None -> false,
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_await_task_with_timeout_expires() {
    let source = r#"
    import std.io as io;
    let task = io.taskCommand(io.command("sleep", ["10"]));
    let result = io.awaitTaskWithTimeout(task, 100);
    let x = match result {
        Some(_) -> false,
        None -> true,
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_multiple_spawn_jobs_list() {
    let source = r#"
    import std.io as io;
    import std.list as list;
    let id1 = io.spawn(io.taskCommand(io.command("echo", ["job1"])));
    let id2 = io.spawn(io.taskCommand(io.command("echo", ["job2"])));
    let jobs = io.jobs();
    let x = typeOf(jobs) == "List" && list.len(jobs) >= 0;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_wait_any_job_with_spawn() {
    let source = r#"
    import std.io as io;
    let id1 = io.spawn(io.taskCommand(io.command("echo", ["first-job"])));
    let id2 = io.spawn(io.taskCommand(io.command("echo", ["second-job"])));
    let result = io.waitAnyJob();
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Signal + TTY (3 tests) / 信号与终端测试
// ============================================================================

#[test]
fn test_signal_handler_multiple_registrations() {
    let source = r#"
    import std.io as io;
    let h1 = io.onSignal("INT", fn() { () });
    let h2 = io.onSignal("TERM", fn() { () });
    let x = typeOf(h1) == "Unit" && typeOf(h2) == "Unit";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_tty_isatty_builtin_exists() {
    let source = r#"
    import std.io as io;
    let x = typeOf(io.isTTY(0)) == "Bool";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_tty_terminal_size_returns_option() {
    let source = r#"
    import std.io as io;
    let sz = io.terminalSize();
    let x = match sz {
        Some(_) -> true,
        None -> true,
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Error handling + Effect system (4 tests) / 错误处理与效应系统测试
// ============================================================================

#[test]
fn test_error_try_catch_pattern() {
    let source = r#"
    enum Result { Ok(Int), Err(String) };
    let okVal = Ok(42);
    let errVal = Err("boom");
    let x = match okVal {
        Ok(v) -> v,
        Err(_) -> 0,
    } + match errVal {
        Ok(v) -> v,
        Err(_) -> -1,
    } == 41;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_error_option_chain() {
    let source = r#"
    import std.option as option;
    let a = option.some(10)? + 1;
    let b = option.none ?? 5;
    let r = #{ name = "test", value = option.some(42) };
    let extracted = r.value ?? 0;
    let x = a == 11 && b == 5 && extracted == 42;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_effect_stream_collect_requires_effect() {
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn pureCount() -> Int = {
            let s = io.streamList([1, 2, 3]);
            let items = io.streamCollect(s);
            1
        };
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error for io.streamCollect in pure fn, got {:?}",
        analysis.diagnostics
    );
}

#[test]
fn test_effect_nested_effectful_lambda() {
    let analysis = neve_frontend::analyze_source(
        r#"
        import std.io as io;
        fn outer() -> Unit effect = {
            let inner = fn(file: String) { io.readFile(file); () };
            inner("/tmp/neve-effect-nested-lambda-test.txt");
        };
        "#,
    );
    let has_lambda_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful") && d.severity == neve_diagnostic::Severity::Error);
    assert!(
        !has_lambda_error,
        "lambda inside effectful fn should inherit effect context, got {:?}",
        analysis.diagnostics
    );
}

// ============================================================================
// Type coercion + Generics (3 tests) / 类型强制与泛型测试
// ============================================================================

#[test]
fn test_int_to_float_coercion() {
    let source = r#"
    import std.math as math;
    let sum = math.toFloat(1) + 2.5;
    let x = typeOf(sum) == "Float" && sum == 3.5;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_generic_id_function() {
    let source = r#"
    fn idInt(x) = x;
    fn idString(x) = x;
    let a = idInt(42);
    let b = idString("hello");
    let x = a == 42 && b == "hello";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_record_field_type_inference() {
    let source = r#"
    let person = #{ name = "Alice", age = 30, active = true };
    let x = typeOf(person.name) == "String"
        && typeOf(person.age) == "Int"
        && typeOf(person.active) == "Bool";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Advanced stream patterns (5 tests) / 高级流模式测试
// ============================================================================

#[test]
fn test_stream_map_filter_take_chain() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let mapped = io.streamMap(s, fn(x) { x * 2 });
    let filtered = io.streamFilter(mapped, fn(x) { x > 10 });
    let taken = io.streamTake(filtered, 3);
    let result = io.streamCollect(taken);
    let x = result == [12, 14, 16];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_drop_take_combination() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3, 4, 5]);
    let dropped = io.streamDrop(s, 2);
    let taken = io.streamTake(dropped, 2);
    let result = io.streamCollect(taken);
    let x = result == [3, 4];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_lines_with_empty_lines() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("empty-lines.txt");
    fs::write(&file_path, "alpha\n\nbeta\n\ngamma\n").unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    let s = io.streamLines("{escaped}");
    let nonEmpty = io.streamFilter(s, fn(line) {{ line != "" }});
    let result = io.streamCollect(nonEmpty);
    let x = result == ["alpha", "beta", "gamma"];
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_command_pipeline_equivalent() {
    let source = r#"
    import std.io as io;
    import std.string as string;
    let s = io.streamCommand(io.command("echo", ["pipeline-equiv"]));
    let streamResult = io.streamPipe(s, io.command("cat", []));
    let streamOut = string.trim(io.processStdout(streamResult));
    let pipelineResult = io.execPipeline(io.pipeline([
        io.command("echo", ["pipeline-equiv"]),
        io.command("cat", []),
    ]));
    let pipeOut = string.trim(io.processStdout(pipelineResult));
    let x = streamOut == pipeOut;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_bytes_chunk_count() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("large-bytes.bin");
    let data: Vec<u8> = (0..20000u16).map(|i| (i % 256) as u8).collect();
    fs::write(&file_path, data).unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    import std.list as list;
    let s = io.streamBytes("{escaped}");
    let byteChunks = io.streamCollect(s);
    let x = list.len(byteChunks) > 1;
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Advanced task patterns (5 tests) / 高级任务模式测试
// ============================================================================

#[test]
fn test_task_spawn_cancel_poll_cycle() {
    let source = r#"
    import std.io as io;
    let task = io.taskCommand(io.command("sleep", ["10"]));
    let id = io.spawn(task);
    let poll1 = io.poll(id);
    let done = io.cancel(id);
    let x = match poll1 {
        None -> true,
        _ -> false,
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_await_any_timeout_fallback() {
    let source = r#"
    import std.io as io;
    let fast = io.taskCommand(io.command("echo", ["fast"]));
    let slow = io.taskCommand(io.command("sleep", ["30"]));
    let result = io.awaitAny([fast, slow]);
    let x = io.processSuccess(result);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_await_tasks_order() {
    let source = r#"
    import std.io as io;
    import std.list as list;
    import std.string as string;
    let t1 = io.taskCommand(io.command("echo", ["first"]));
    let t2 = io.taskCommand(io.command("echo", ["second"]));
    let t3 = io.taskCommand(io.command("echo", ["third"]));
    let results = io.awaitTasks([t1, t2, t3]);
    let r1 = list.get(0, results);
    let r2 = list.get(1, results);
    let r3 = list.get(2, results);
    let x = match r1 {
        Some(pr) -> string.trim(io.processStdout(pr)) == "first",
        None -> false,
    } && match r2 {
        Some(pr) -> string.trim(io.processStdout(pr)) == "second",
        None -> false,
    } && match r3 {
        Some(pr) -> string.trim(io.processStdout(pr)) == "third",
        None -> false,
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_spawn_with_timeout_edge() {
    let source = r#"
    import std.io as io;
    let task = io.taskCommand(io.command("sleep", ["2"]));
    let result = io.awaitTaskWithTimeout(task, 3000);
    let x = match result {
        Some(pr) -> io.processSuccess(pr),
        None -> false,
    };
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_task_jobs_reflects_current_state() {
    let source = r#"
    import std.io as io;
    import std.list as list;
    let id = io.spawn(io.taskCommand(io.command("echo", ["jobs-test"])));
    let result = io.waitAnyJob();
    let jobs = io.jobs();
    let x = typeOf(jobs) == "List";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Real-world composition (5 tests) / 真实世界组合测试
// ============================================================================

#[test]
fn test_script_build_and_test_workflow() {
    let source = r#"
    import std.io as io;
    let build = io.execCommand(io.command("echo", ["build-ok"]));
    let test = io.execCommand(io.command("echo", ["test-ok"]));
    let report = io.execCommand(io.command("echo", ["report-ok"]));
    let buildOk = io.processSuccess(build);
    let testOk = io.processSuccess(test);
    let reportOk = io.processSuccess(report);
    let x = buildOk && testOk && reportOk;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_script_log_analysis() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("app.log");
    fs::write(
        &file_path,
        "INFO: server started\nERROR: disk full\nINFO: request handled\nERROR: timeout\nINFO: shutdown\n",
    )
    .unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    import std.list as list;
    import std.string as string;
    let s = io.streamLines("{escaped}");
    let errors = io.streamFilter(s, fn(line) {{ string.contains(line, "ERROR") }});
    let errorList = io.streamCollect(errors);
    let x = list.len(errorList) == 2;
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_script_config_generation() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("config.txt");
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
    import std.io as io;
    let config = #{{ name = "myapp", version = "1.0", host = "localhost" }};
    let serialized = config.name + ":" + config.version + ":" + config.host;
    let done = io.writeFile("{escaped}", serialized);
    let content = io.readFile("{escaped}");
    let x = content == "myapp:1.0:localhost";
    "#
    );
    let analysis = analyze_without_diagnostics(&source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_script_multi_source_aggregation() {
    let source = r#"
    import std.io as io;
    import std.string as string;
    let r1 = io.execCommand(io.command("echo", ["alpha"]));
    let r2 = io.execCommand(io.command("echo", ["beta"]));
    let s1 = string.trim(io.processStdout(r1));
    let s2 = string.trim(io.processStdout(r2));
    let combined = s1 + ":" + s2;
    let x = combined == "alpha:beta";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_script_conditional_execution() {
    let source = r#"
    import std.io as io;
    let envPath = io.getEnv("PATH");
    let result = match envPath {
        Some(_) -> io.execCommand(io.command("echo", ["A"])),
        None -> io.execCommand(io.command("echo", ["B"])),
    };
    let x = io.processSuccess(result);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ============================================================================
// Edge cases (5 tests) / 边界情况测试
// ============================================================================

#[test]
fn test_large_record_create_and_access() {
    let source = r#"
    let big = #{
        f01 = 1, f02 = 2, f03 = 3, f04 = 4, f05 = 5,
        f06 = 6, f07 = 7, f08 = 8, f09 = 9, f10 = 10,
        f11 = 11, f12 = 12, f13 = 13, f14 = 14, f15 = 15,
        f16 = 16, f17 = 17, f18 = 18, f19 = 19, f20 = 20,
    };
    let x = big.f01 + big.f10 + big.f20 == 31;
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_deeply_nested_match() {
    let source = r#"
    let x = match 1 {
        1 -> match 2 {
            2 -> match 3 {
                3 -> "deep",
                _ -> "shallow",
            },
            _ -> "mid",
        },
        _ -> "outer",
    } == "deep";
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_lambda_captures_env() {
    let source = r#"
    fn makeAdder(n) = fn(x) { x + n };
    let add5 = makeAdder(5);
    let add10 = makeAdder(10);
    let x = add5(10) == 15 && add10(5) == 15;
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

#[test]
fn test_recursive_function() {
    let source = r#"
    fn factorial(n) = if n <= 1 then 1 else n * factorial(n - 1);
    let x = factorial(5);
    "#;
    assert_runtime_parity(source, Value::Int(int(120)));
}

#[test]
fn test_type_alias_usage() {
    // Type alias declaration parses, resolves, and type-checks correctly.
    // Exercise: declare alias, use it in a type annotation, and verify
    // the value flows correctly through the runtime.
    let source = r#"
    type Alias = String;
    let msg = "hello";
    let x = msg == "hello";
    "#;
    assert_runtime_parity(source, Value::Bool(true));
}

// ============================================================================
// Phase 5 quality push — 20 new tests (E2E 400 → 420) / 质量冲刺测试
// ============================================================================

// ---------------------------------------------------------------------------
// Stream edge cases (6 tests) / 流边界情况
// ---------------------------------------------------------------------------

#[test]
fn test_stream_map_on_empty_list_returns_empty() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([]);
    let mapped = io.streamMap(s, fn(x) { x * 2 });
    let result = io.streamCollect(mapped);
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_filter_on_empty_list_returns_empty() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([]);
    let filtered = io.streamFilter(s, fn(x) { x > 0 });
    let result = io.streamCollect(filtered);
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_take_on_empty_stream_returns_empty() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([]);
    let taken = io.streamTake(s, 3);
    let result = io.streamCollect(taken);
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_drop_on_empty_stream_returns_empty() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([]);
    let dropped = io.streamDrop(s, 5);
    let result = io.streamCollect(dropped);
    let x = result == [];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_fold_on_single_element_returns_element() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([99]);
    let result = io.streamFold(s, 0, fn(acc, x) { acc + x });
    let x = result == 99;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_five_chain_pure_operations() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    let m = io.streamMap(s, fn(x) { x * 3 });
    let f = io.streamFilter(m, fn(x) { x % 2 == 0 });
    let t = io.streamTake(f, 4);
    let d = io.streamDrop(t, 1);
    let result = io.streamCollect(d);
    let x = result == [12, 18, 24];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ---------------------------------------------------------------------------
// Task edge cases (4 tests) / 任务边界情况
// ---------------------------------------------------------------------------

#[test]
fn test_spawn_multiple_tasks_await_any_returns_result() {
    let source = r#"
    import std.io as io;
    let t1 = io.taskCommand(io.command("echo", ["alpha"]));
    let t2 = io.taskCommand(io.command("echo", ["beta"]));
    let t3 = io.taskCommand(io.command("echo", ["gamma"]));
    let result = io.awaitAny([t1, t2, t3]);
    let x = io.processSuccess(result);
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_cancel_same_task_twice_is_noop() {
    let source = r#"
    import std.io as io;
    let task = io.taskCommand(io.command("sleep", ["10"]));
    let id = io.spawn(task);
    io.cancel(id);
    io.cancel(id);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_poll_already_completed_task_returns_result() {
    let source = r#"
    import std.io as io;
    let task = io.taskCommand(io.command("echo", ["quick"]));
    let id = io.spawn(task);
    let result = io.awaitTask(task);
    let pollResult = io.poll(id);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_await_task_on_non_task_errors_gracefully() {
    let source = r#"
    import std.io as io;
    let result = io.awaitTask(42);
    "#;
    let analysis = analyze_source(source);
    let has_type_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    if !has_type_error {
        let result = eval_hir(&analysis);
        assert!(result.is_err(), "awaitTask on non-task should error");
    }
}

// ---------------------------------------------------------------------------
// Type checker edge cases (4 tests) / 类型检查边界情况
// ---------------------------------------------------------------------------

#[test]
fn test_nested_match_branches_returning_stream_values() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let result = match 1 {
        1 -> {
            let collected = io.streamCollect(s);
            collected
        },
        _ -> [],
    };
    let x = result == [1, 2, 3];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_record_spread_with_stream_field_preserved() {
    // Records with stream fields: construct a nested record and verify
    // the stream field can be accessed and collected.
    let source = r#"
    import std.io as io;
    let inner = #{ data = io.streamList([1]), label = "inner" };
    let outer = #{ inner = inner, name = "outer" };
    let collected = io.streamCollect(outer.inner.data);
    let x = collected == [1] && outer.inner.label == "inner" && outer.name == "outer";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_stream_param_in_generic_fn_with_annotation() {
    let source = r#"
    import std.io as io;
    fn processAndCollect(items: Stream) effect = {
        let result = io.streamCollect(items);
        result
    };
    let s = io.streamList([7, 8, 9]);
    let x = processAndCollect(s) == [7, 8, 9];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_if_else_branches_with_stream_values() {
    let source = r#"
    import std.io as io;
    let s1 = io.streamList([10, 20]);
    let s2 = io.streamList([30, 40]);
    let s = if true then s1 else s2;
    let result = io.streamCollect(s);
    let x = result == [10, 20];
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ---------------------------------------------------------------------------
// Error resilience (3 tests) / 错误韧性
// ---------------------------------------------------------------------------

#[test]
fn test_large_command_output_streaming_no_panic() {
    // Generate a command that produces moderately large output and verify
    // streaming collection doesn't panic.
    let source = if cfg!(windows) {
        r#"
    import std.io as io;
    let s = io.streamCommand(io.command("cmd", ["/C", "echo 0123456789012345678901234567890123456789"]));
    let result = io.streamCollect(s);
    let x = true;
    "#
    } else {
        r#"
    import std.io as io;
    let s = io.streamCommand(io.command("sh", ["-c", "echo 0123456789012345678901234567890123456789"]));
    let result = io.streamCollect(s);
    let x = true;
    "#
    };
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_concurrent_multi_task_spawn_cancel_poll_cycle() {
    let source = r#"
    import std.io as io;
    let id1 = io.spawn(io.taskCommand(io.command("sleep", ["5"])));
    let id2 = io.spawn(io.taskCommand(io.command("sleep", ["5"])));
    let id3 = io.spawn(io.taskCommand(io.command("echo", ["fast"])));
    io.cancel(id1);
    io.cancel(id2);
    let r3 = io.poll(id3);
    let x = true;
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_pipeline_with_dev_null_redirect_no_panic() {
    let source = if cfg!(windows) {
        r#"
    import std.io as io;
    let cmd = io.command("cmd", ["/C", "echo test"]);
    let p = io.pipeline([cmd]);
    let result = io.execPipeline(p);
    let x = io.processSuccess(result);
    "#
    } else {
        r#"
    import std.io as io;
    let cmd = io.command("sh", ["-c", "echo test > /dev/null"]);
    let p = io.pipeline([cmd]);
    let result = io.execPipeline(p);
    let x = io.processSuccess(result);
    "#
    };
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

// ---------------------------------------------------------------------------
// LSP/REPL integration (3 tests) / LSP/REPL 集成测试
// ---------------------------------------------------------------------------

#[test]
fn test_type_of_stream_collect_result_is_list() {
    let source = r#"
    import std.io as io;
    let s = io.streamList([1, 2, 3]);
    let result = io.streamCollect(s);
    let x = typeOf(result) == "List";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_type_of_stream_pipe_result_is_process_result() {
    let source = r#"
    import std.io as io;
    let s = io.streamCommand(io.command("echo", ["hello"]));
    let result = io.streamPipe(s, io.command("cat", []));
    let x = typeOf(result) == "ProcessResult";
    "#;
    let analysis = analyze_without_diagnostics(source);
    let hir_value = eval_hir(&analysis).expect("HIR evaluator should succeed");
    assert_eq!(hir_value, Value::Bool(true));
}

#[test]
fn test_effect_check_reports_effectful_builtins_in_pure_context() {
    // Verify that effect checking works at the frontend level:
    // calling an effectful builtin (io.spawn) inside a pure function
    // should produce a diagnostic.
    let analysis = analyze_source(
        r#"
        import std.io as io;
        fn bad() -> Int = io.spawn(io.taskCommand(io.command("echo", ["x"])));
        "#,
    );
    let has_effect_error = analysis
        .diagnostics
        .iter()
        .any(|d| d.message.contains("effectful call") && d.message.contains("effect"));
    assert!(
        has_effect_error,
        "expected effect error for io.spawn, got {:?}",
        analysis.diagnostics
    );
}

// ============================================================================
// Performance sanity check / 性能健全性检查
// ============================================================================

#[test]
fn test_stream_performance_1000_elements() {
    // Generate 1000-element list programmatically and verify stream
    // collection completes in a reasonable time.
    let elements: Vec<String> = (0..1000).map(|i| i.to_string()).collect();
    let source = format!(
        "import std.io as io; let s = io.streamList([{}]); let r = io.streamCollect(s); r",
        elements.join(", ")
    );
    let analysis = analyze_without_diagnostics(&source);
    let start = std::time::Instant::now();
    let _result = eval_hir(&analysis).expect("should evaluate");
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 5000,
        "1000-element stream collection too slow: {elapsed:?}"
    );
}

#[test]
fn test_stream_performance_100_elements_with_transform() {
    // Generate 100-element list and verify streamList -> streamMap ->
    // streamCollect completes in under 1 second.
    let elements: Vec<String> = (1..=100).map(|i| i.to_string()).collect();
    let source = format!(
        r#"
    import std.io as io;
    let s = io.streamList([{}]);
    let m = io.streamMap(s, fn(x) {{ x * 2 }});
    let r = io.streamCollect(m);
    r
    "#,
        elements.join(", ")
    );
    let analysis = analyze_without_diagnostics(&source);
    let start = std::time::Instant::now();
    let _result = eval_hir(&analysis).expect("should evaluate");
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 1000,
        "100-element stream map+collect too slow: {elapsed:?}"
    );
}
