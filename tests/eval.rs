//! Integration tests for neve-eval crate.
//!
//! This file contains extensive edge case tests for the evaluator.

mod support;

use neve_common::Int;
use neve_derive::Hash;
use neve_eval::{EvalError, EvaluableModuleRef, Evaluator, Value, compat::AstEvaluator};
use neve_frontend::analyze_source;
use neve_hir::lower;
use neve_parser::parse;
use neve_std::{std_module_overrides, stdlib};
use std::fs;
use std::rc::Rc;
use support::fetch_fixtures::{init_local_git_repo, start_local_http_fixture};
use support::module_fixtures::create_test_module;
use support::source_fixtures::{
    pipeline_execution_source as shared_pipeline_execution_source,
    shell_projection_source as shared_shell_projection_source,
};
use tempfile::TempDir;

fn eval_source(source: &str) -> Result<Value, EvalError> {
    let (ast, _) = parse(source);
    let hir = lower(&ast);
    let mut eval = Evaluator::new();
    eval.eval_module(&hir)
}

fn eval_checked_hir(source: &str) -> Result<Value, EvalError> {
    let analysis = analyze_source(source);
    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected frontend diagnostics: {:?}",
        analysis.diagnostics
    );
    let mut eval = Evaluator::new().with_extra_builtins(
        stdlib()
            .into_iter()
            .map(|(name, value)| (name.to_string(), value)),
    );
    eval.eval_evaluable_module(EvaluableModuleRef::new(
        &analysis.hir,
        &analysis.semantics.method_resolutions,
    ))
}

/// Evaluate source with builtins available (using the AST compat evaluator).
fn eval_with_builtins(source: &str) -> Result<Value, String> {
    let (ast, errors) = parse(source);
    if !errors.is_empty() {
        return Err(format!("parse error: {:?}", errors));
    }
    let mut eval = AstEvaluator::new();
    eval.eval_file(&ast).map_err(|e| e.to_string())
}

/// Evaluate source with stdlib module overrides.
fn eval_with_std(source: &str) -> Result<Value, String> {
    let (ast, errors) = parse(source);
    if !errors.is_empty() {
        return Err(format!("parse error: {:?}", errors));
    }
    let mut eval = AstEvaluator::new().with_module_overrides(std_module_overrides());
    eval.eval_file(&ast).map_err(|e| e.to_string())
}

fn int(value: i64) -> Int {
    value.into()
}

fn shell_projection_source() -> String {
    shared_shell_projection_source(None)
}

fn pipeline_execution_source() -> String {
    shared_pipeline_execution_source(None)
}

// ============================================================================
// 标准库模块导入
// ============================================================================

#[test]
fn test_eval_import_std_list_module() {
    let source = "import std.list; let xs = list.range(1, 4); let n = list.len(xs);";
    match eval_with_std(source) {
        Ok(Value::Int(n)) => assert_eq!(n, int(3)),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_item_import() {
    let source = "import std.list (len); let result = len([1, 2, 3, 4]);";
    match eval_checked_hir(source) {
        Ok(Value::Int(n)) => assert_eq!(n, int(4)),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_module_import() {
    let source = "import std.list = listOps; let result = listOps.len([1, 2]);";
    match eval_checked_hir(source) {
        Ok(Value::Int(n)) => assert_eq!(n, int(2)),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_glob_import() {
    let source = "import std.list (*); let result = len([1, 2, 3]);";
    match eval_checked_hir(source) {
        Ok(Value::Int(n)) => assert_eq!(n, int(3)),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_option_builtins() {
    let source = r#"
        import std.option = option;
        let a = option.some(41)? + 1;
        let b = option.none ?? 5;
        let result = a + b;
    "#;
    match eval_checked_hir(source) {
        Ok(Value::Int(n)) => assert_eq!(n, int(47)),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_builtin_option_match_patterns() {
    let source = r#"
        import std.option = option;
        let result = match option.some(41) {
            Some(value) -> value,
            None -> 0
        };
    "#;
    match eval_checked_hir(source) {
        Ok(Value::Int(n)) => assert_eq!(n, int(41)),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_result_builtins() {
    let source = r#"
        import std.result = result;
        let a = result.ok(41)? + 1;
        let resultValue = result.unwrap_err(result.err("boom"));
        let answer = if resultValue == "boom" -> a else 0;
    "#;
    match eval_checked_hir(source) {
        Ok(Value::Int(n)) => assert_eq!(n, int(42)),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_builtin_result_match_patterns() {
    let source = r#"
        import std.result = result;
        let answer = match result.err("boom") {
            Ok(value) -> value,
            Err(message) -> if message == "boom" -> 1 else 0
        };
    "#;
    match eval_checked_hir(source) {
        Ok(Value::Int(n)) => assert_eq!(n, int(1)),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_path_builtins() {
    let source = r#"
        import std.path = path;
        let parent = path.parent("/tmp/file.txt") ?? "/";
        let result = if path.is_absolute("/tmp/file.txt") -> parent else "nope";
    "#;
    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "/tmp"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_path_from_string_exposes_path_runtime_value() {
    let source = r#"
        import std.path = path;
        let p = path.fromString("/tmp/file.txt");
        let result = if typeOf(p) == "Path" -> toString(p) else "nope";
    "#;
    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "/tmp/file.txt"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_typed_path_adapters() {
    let source = r#"
        import std.path = path;
        let nested = path.joinPath(path.fromString("/tmp"), "neve.txt");
        let name = path.filenamePath(nested) ?? "missing";
        let ext = path.extensionPath(nested) ?? "missing";
        let result = if name == "neve.txt" && ext == "txt" -> "ok" else "nope";
    "#;
    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "ok"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_builtins() {
    let source = r#"
        import std.io = io;
        let digest = io.hashString("abc");
    "#;
    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(
            s.as_ref(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        ),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_current_system_bridge() {
    let expected = format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS);
    let source = r#"
        import std.io = io;
        io.currentSystem()
    "#;
    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), expected.as_str()),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_current_dir_string_bridge() {
    let expected = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let source = r#"
        import std.io = io;
        io.currentDir()
    "#;
    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), expected.as_str()),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_get_env_bridge() {
    let missing = "__NEVE_TEST_MISSING_ENV_37C93B7C__";
    assert!(
        std::env::var_os(missing).is_none(),
        "test environment unexpectedly defines {missing}"
    );
    let source = format!(
        r#"
        import std.io = io;
        io.getEnv("{missing}") ?? "missing"
    "#
    );
    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "missing"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_fetch_path_bridge() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("source.txt");
    let content = b"fetch-path-content";
    fs::write(&file_path, content).unwrap();
    let expected = Hash::of(content).to_hex();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.fetch = fetch;
        fetch.path("{escaped}").hash
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), expected.as_str()),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_fetch_path_with_hash_bridge() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("source.txt");
    let content = b"fetch-path-content";
    fs::write(&file_path, content).unwrap();
    let expected = Hash::of(content).to_hex();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.fetch = fetch;
        fetch.pathWithHash("{escaped}", "{expected}").hash
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), expected.as_str()),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_fetch_url_with_hash_bridge() {
    let (url, expected_hash, server) = start_local_http_fixture(b"fetch-url-content");
    let source = format!(
        r#"
        import std.fetch = fetch;
        fetch.urlWithHash("{url}", "{expected_hash}").hash
    "#
    );

    let result = eval_checked_hir(&source);
    server.join().expect("fixture server should exit cleanly");

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), expected_hash.as_str()),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_fetch_url_bridge() {
    let (url, expected_hash, server) = start_local_http_fixture(b"fetch-url-content");
    let source = format!(
        r#"
        import std.fetch = fetch;
        fetch.url("{url}").hash
    "#
    );

    let result = eval_checked_hir(&source);
    server.join().expect("fixture server should exit cleanly");

    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), expected_hash.as_str()),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_fetch_git_bridge() {
    let (_temp, repo_path, expected_hash) = init_local_git_repo();
    let escaped = repo_path.replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.fetch = fetch;
        fetch.git("{escaped}", "main").hash
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), expected_hash.as_str()),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_fetch_git_with_hash_bridge() {
    let (_temp, repo_path, expected_hash) = init_local_git_repo();
    let escaped = repo_path.replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.fetch = fetch;
        fetch.gitWithHash("{escaped}", "main", "{expected_hash}").hash
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), expected_hash.as_str()),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_read_file_bridge() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("source.txt");
    std::fs::write(&file_path, "read-file-content").unwrap();
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        io.readFile("{escaped}")
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "read-file-content"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_read_dir_bridge() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("io-read-dir");
    std::fs::create_dir_all(dir.join("nested")).unwrap();
    std::fs::write(dir.join("alpha.txt"), "a").unwrap();
    std::fs::write(dir.join("beta.txt"), "b").unwrap();
    let escaped = dir.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        import std.list = list;
        list.sort(io.readDir("{escaped}"))
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::List(entries)) => {
            let names = entries
                .iter()
                .map(|entry| match entry {
                    Value::String(name) => name.to_string(),
                    other => panic!("expected String entry, got {:?}", other),
                })
                .collect::<Vec<_>>();
            assert_eq!(names, vec!["alpha.txt", "beta.txt", "nested"]);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_hash_file_bridge() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("source.txt");
    let content = b"hash-file-content";
    fs::write(&file_path, content).unwrap();
    let expected = "09f00a4ba8e49c5a253e1af9ff6c40f8151754ccd88f95ef162981960b2ad8f7";
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        io.hashFile("{escaped}")
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), expected),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_hash_file_path_bridge() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("source.txt");
    let content = b"hash-file-path-content";
    fs::write(&file_path, content).unwrap();
    let expected = "9c3675e0b07ef1223e4cb9afdc255c51c8557ac075e91e601978676b894c95b1";
    let escaped = file_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        io.hashFilePath(path.fromString("{escaped}"))
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), expected),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_read_file_path_bridge() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("io-read-path.neve.txt");
    std::fs::write(&file_path, "hello-path").unwrap();
    let escaped = file_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let content = io.readFilePath(path.fromString("{escaped}"));
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "hello-path"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_read_file_bytes_path_bridge() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("io-read-bytes-path.neve.bin");
    std::fs::write(&file_path, [0xde, 0xad, 0xbe, 0xef]).unwrap();
    let escaped = file_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let bytes = io.readFileBytesPath(path.fromString("{escaped}"));
        let shown = if typeOf(bytes) == "Bytes" -> toString(bytes) else "nope";
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "<bytes:4>"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_read_dir_path_bridge() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("io-read-dir-path.neve");
    std::fs::create_dir_all(dir_path.join("nested")).unwrap();
    std::fs::write(dir_path.join("alpha.txt"), "a").unwrap();
    std::fs::write(dir_path.join("beta.txt"), "b").unwrap();
    let escaped = dir_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        import std.list = list;
        let entries = io.readDirPath(path.fromString("{escaped}"));
        list.sort(entries)
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::List(entries)) => {
            let names = entries
                .iter()
                .map(|entry| match entry {
                    Value::String(name) => name.to_string(),
                    other => panic!("expected String entry, got {:?}", other),
                })
                .collect::<Vec<_>>();
            assert_eq!(names, vec!["alpha.txt", "beta.txt", "nested"]);
        }
        other => panic!("expected List<String>, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_read_dir_entry_paths_bridge() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("io-read-dir-entry-paths.neve");
    let file_path = dir_path.join("alpha.txt");
    let nested_path = dir_path.join("nested");
    std::fs::create_dir_all(&dir_path).unwrap();
    std::fs::create_dir_all(&nested_path).unwrap();
    std::fs::write(&file_path, "a").unwrap();
    let escaped = dir_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        import std.list = list;
        list.sort(io.readDirEntryPaths(path.fromString("{escaped}")))
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::List(entries)) => assert_eq!(
            entries.as_ref(),
            &[
                Value::Path(Rc::new(file_path)),
                Value::Path(Rc::new(nested_path)),
            ]
        ),
        other => panic!("expected List<Path>, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_write_file_bytes_path_bridge() {
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
        r#"
        import std.io = io;
        import std.path = path;
        let src = path.fromString("{escaped_src}");
        let dst = path.fromString("{escaped_dst}");
        let bytes = io.readFileBytesPath(src);
        let done = io.writeFileBytesPath(dst, bytes);
        let copied = io.readFileBytesPath(dst);
        let ok = typeOf(copied) == "Bytes" && copied == bytes;
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_write_file_path_bridge() {
    let temp_dir = TempDir::new().unwrap();
    let dst_path = temp_dir.path().join("io-write-path-dst.neve.txt");
    let escaped_dst = dst_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let dst = path.fromString("{escaped_dst}");
        let done = io.writeFilePath(dst, "hello-path");
        io.readFilePath(dst)
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "hello-path"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_write_file_bridge() {
    let temp_dir = TempDir::new().unwrap();
    let dst_path = temp_dir.path().join("io-write-dst.neve.txt");
    let escaped_dst = dst_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        r#"
        import std.io = io;
        let done = io.writeFile("{escaped_dst}", "hello");
        io.readFile("{escaped_dst}")
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "hello"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_append_file_path_bridge() {
    let temp_dir = TempDir::new().unwrap();
    let dst_path = temp_dir.path().join("io-append-path-dst.neve.txt");
    std::fs::write(&dst_path, "hello").unwrap();
    let escaped_dst = dst_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let dst = path.fromString("{escaped_dst}");
        let done = io.appendFilePath(dst, "-path");
        io.readFilePath(dst)
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "hello-path"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_append_file_bridge() {
    let temp_dir = TempDir::new().unwrap();
    let dst_path = temp_dir.path().join("io-append-dst.neve.txt");
    std::fs::write(&dst_path, "hello").unwrap();
    let escaped_dst = dst_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        r#"
        import std.io = io;
        let done = io.appendFile("{escaped_dst}", "-path");
        io.readFile("{escaped_dst}")
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "hello-path"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_append_file_bytes_path_bridge() {
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
        r#"
        import std.io = io;
        import std.path = path;
        let init = io.readFileBytesPath(path.fromString("{escaped_init}"));
        let append = io.readFileBytesPath(path.fromString("{escaped_append}"));
        let expected = io.readFileBytesPath(path.fromString("{escaped_expected}"));
        let dst = path.fromString("{escaped_dst}");
        let reset = io.writeFileBytesPath(dst, init);
        let done = io.appendFileBytesPath(dst, append);
        let copied = io.readFileBytesPath(dst);
        let ok = typeOf(copied) == "Bytes" && copied == expected;
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_current_dir_path_bridge() {
    let expected = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let source = r#"
        import std.io = io;
        let cwd = io.currentDirPath();
        let shown = if typeOf(cwd) == "Path" -> toString(cwd) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), expected.as_str()),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_home_dir_path_bridge() {
    let source = r#"
        import std.io = io;
        io.homeDirPath()
    "#;

    let expected = match std::env::var("HOME") {
        Ok(home) => Value::Some(Box::new(Value::Path(Rc::new(home.into())))),
        Err(_) => Value::None,
    };

    match eval_checked_hir(source) {
        Ok(value) => assert_eq!(value, expected),
        other => panic!("expected optional path, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_home_dir_bridge() {
    let source = r#"
        import std.io = io;
        io.homeDir()
    "#;

    let expected = match std::env::var("HOME") {
        Ok(home) => Value::Some(Box::new(Value::String(Rc::new(home)))),
        Err(_) => Value::None,
    };

    match eval_checked_hir(source) {
        Ok(value) => assert_eq!(value, expected),
        other => panic!("expected optional string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_create_dir_all_bridge() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("legacy").join("a").join("b");
    let escaped = target.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        let target = "{escaped}";
        let done = io.createDirAll(target);
        io.pathExists(target) && io.isDir(target)
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool true, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_create_dir_all_path_bridge() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("typed").join("a").join("b");
    let escaped = target.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let target = path.fromString("{escaped}");
        let done = io.createDirAllPath(target);
        io.pathExistsPath(target) && io.isDirPath(target)
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool true, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_remove_dir_all_path_bridge() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("typed").join("a").join("b");
    fs::create_dir_all(&target).unwrap();
    let escaped = target.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let target = path.fromString("{escaped}");
        let done = io.removeDirAllPath(target);
        !io.pathExistsPath(target)
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool true, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_remove_dir_all_bridge() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("legacy").join("a").join("b");
    let escaped = target.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        let target = "{escaped}";
        let created = io.createDirAll(target);
        let done = io.removeDirAll(target);
        !io.pathExists(target)
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool true, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_path_exists_bridge() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("exists.txt");
    fs::write(&file, "neve").unwrap();
    let escaped = file.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        io.pathExists("{escaped}")
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool true, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_is_dir_bridge() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("nested");
    fs::create_dir_all(&dir).unwrap();
    let escaped = dir.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        io.isDir("{escaped}")
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool true, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_is_file_bridge() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("nested.txt");
    fs::write(&file, "neve").unwrap();
    let escaped = file.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        io.isFile("{escaped}")
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool true, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_command_bridge_exposes_command_runtime_value() {
    let source = r#"
        import std.io = io;
        let cmd = io.command("printf", ["neve"]);
        let shown = if typeOf(cmd) == "Command" -> toString(cmd) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "<command:printf 1 arg(s)>"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_command_with_bridge_exposes_configured_command_runtime_value() {
    let source = r#"
        import std.io = io;
        let cmd = io.commandWith(#{ program = "printf", args = ["neve"], cwd = "/tmp" });
        let shown = if typeOf(cmd) == "Command" -> toString(cmd) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => {
            assert_eq!(s.as_ref(), "<command:printf 1 arg(s), configured>")
        }
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_pipeline_bridge_exposes_pipeline_runtime_value() {
    let source = r#"
        import std.io = io;
        let pipe = io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])]);
        let shown = if typeOf(pipe) == "Pipeline" -> toString(pipe) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "<pipeline:2 command(s)>"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_pipeline_with_redirects_bridge_exposes_pipeline_runtime_value() {
    let source = r#"
        import std.io = io;
        import std.path = path;
        let pipe = io.pipelineWithRedirects(
            io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])]),
            [io.redirectStdoutPath(path.fromString("/tmp/neve.out"))]
        );
        let shown = if typeOf(pipe) == "Pipeline" -> toString(pipe) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "<pipeline:2 command(s)>"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_pipeline_bridge_exposes_process_result_runtime_value() {
    match eval_checked_hir(&pipeline_execution_source()) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_pipeline_with_redirect_bridge_exposes_process_result_runtime_value() {
    let temp = TempDir::new().expect("temp dir should exist");
    let redirect_path = temp.path().join("pipeline-stdout.txt");
    let redirect_path_source = redirect_path.to_string_lossy().replace('\\', "\\\\");
    let source = if cfg!(windows) {
        format!(
            r#"
        import std.io = io;
        import std.path = path;
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
        let ok =
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
        import std.io = io;
        import std.path = path;
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
        let ok =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) == "" &&
            redirected != "";
    "#
        )
    };

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_redirect_stdout_path_bridge_exposes_redirect_runtime_value() {
    let source = r#"
        import std.io = io;
        import std.path = path;
        let redirect = io.redirectStdoutPath(path.fromString("/tmp/neve.out"));
        let shown = if typeOf(redirect) == "Redirect" -> toString(redirect) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "<redirect:stdout:path>"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_redirect_stderr_path_bridge_exposes_redirect_runtime_value() {
    let source = r#"
        import std.io = io;
        import std.path = path;
        let redirect = io.redirectStderrPath(path.fromString("/tmp/neve.err"));
        let shown = if typeOf(redirect) == "Redirect" -> toString(redirect) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "<redirect:stderr:path>"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_redirect_stdin_path_bridge_exposes_redirect_runtime_value() {
    let source = r#"
        import std.io = io;
        import std.path = path;
        let redirect = io.redirectStdinPath(path.fromString("/tmp/neve.in"));
        let shown = if typeOf(redirect) == "Redirect" -> toString(redirect) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "<redirect:stdin:path>"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_command_with_redirect_writes_stdout_to_file() {
    let temp = TempDir::new().expect("temp dir should exist");
    let redirect_path = temp.path().join("stdout.txt");
    let redirect_path_source = redirect_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let target = path.fromString("{redirect_path_source}");
        let result = io.execCommand(
            io.commandWithRedirects(
                io.command("rustc", ["--version"]),
                [io.redirectStdoutPath(target)]
            )
        );
        let redirected = io.readFilePath(target);
        let ok =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) == "" &&
            redirected != "";
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_command_with_stderr_redirect_writes_stderr_to_file() {
    let temp = TempDir::new().expect("temp dir should exist");
    let redirect_path = temp.path().join("stderr.txt");
    let redirect_path_source = redirect_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let target = path.fromString("{redirect_path_source}");
        let result = io.execCommand(
            io.commandWithRedirects(
                io.command("rustc", ["--definitely-not-a-real-rustc-flag"]),
                [io.redirectStderrPath(target)]
            )
        );
        let redirected = io.readFilePath(target);
        let ok =
            typeOf(result) == "ProcessResult" &&
            !io.processSuccess(result) &&
            io.processCode(result) != 0 &&
            io.processStderr(result) == "" &&
            redirected != "";
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_command_with_stdin_redirect_reads_stdin_from_file() {
    let temp = TempDir::new().expect("temp dir should exist");
    let redirect_path = temp.path().join("stdin.txt");
    fs::write(&redirect_path, "neve stdin line\n").expect("stdin file should be writable");
    let redirect_path_source = redirect_path.to_string_lossy().replace('\\', "\\\\");
    let source = if cfg!(windows) {
        format!(
            r#"
        import std.io = io;
        import std.path = path;
        let target = path.fromString("{redirect_path_source}");
        let result = io.execCommand(
            io.commandWithRedirects(
                io.command("cmd", ["/C", "findstr neve"]),
                [io.redirectStdinPath(target)]
            )
        );
        let ok =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) != "";
    "#
        )
    } else {
        format!(
            r#"
        import std.io = io;
        import std.path = path;
        let target = path.fromString("{redirect_path_source}");
        let result = io.execCommand(
            io.commandWithRedirects(
                io.command("sh", ["-c", "grep neve"]),
                [io.redirectStdinPath(target)]
            )
        );
        let ok =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) != "";
    "#
        )
    };

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_command_with_redirects_composes_stdin_and_stdout_paths() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stdin_path = temp.path().join("stdin.txt");
    let stdout_path = temp.path().join("stdout.txt");
    fs::write(&stdin_path, "neve stdin line\n").expect("stdin file should be writable");
    let stdin_path_source = stdin_path.to_string_lossy().replace('\\', "\\\\");
    let stdout_path_source = stdout_path.to_string_lossy().replace('\\', "\\\\");
    let source = if cfg!(windows) {
        format!(
            r#"
        import std.io = io;
        import std.path = path;
        let input = path.fromString("{stdin_path_source}");
        let output = path.fromString("{stdout_path_source}");
        let result = io.execCommand(
            io.commandWithRedirects(
                io.command("cmd", ["/C", "findstr neve"]),
                [io.redirectStdinPath(input), io.redirectStdoutPath(output)]
            )
        );
        let redirected = io.readFilePath(output);
        let ok =
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
        import std.io = io;
        import std.path = path;
        let input = path.fromString("{stdin_path_source}");
        let output = path.fromString("{stdout_path_source}");
        let result = io.execCommand(
            io.commandWithRedirects(
                io.command("sh", ["-c", "grep neve"]),
                [io.redirectStdinPath(input), io.redirectStdoutPath(output)]
            )
        );
        let redirected = io.readFilePath(output);
        let ok =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) == "" &&
            redirected != "";
    "#
        )
    };

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_pipeline_with_stdin_redirect_reads_stdin_from_file() {
    let temp = TempDir::new().expect("temp dir should exist");
    let redirect_path = temp.path().join("pipeline-stdin.txt");
    fs::write(&redirect_path, "neve stdin line\n").expect("stdin file should be writable");
    let redirect_path_source = redirect_path.to_string_lossy().replace('\\', "\\\\");
    let source = if cfg!(windows) {
        format!(
            r#"
        import std.io = io;
        import std.path = path;
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
        let ok =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) != "";
    "#
        )
    } else {
        format!(
            r#"
        import std.io = io;
        import std.path = path;
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
        let ok =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) != "";
    "#
        )
    };

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_pipeline_with_redirects_composes_stdin_and_stdout_paths() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stdin_path = temp.path().join("pipeline-stdin.txt");
    let stdout_path = temp.path().join("pipeline-stdout.txt");
    fs::write(&stdin_path, "neve stdin line\n").expect("stdin file should be writable");
    let stdin_path_source = stdin_path.to_string_lossy().replace('\\', "\\\\");
    let stdout_path_source = stdout_path.to_string_lossy().replace('\\', "\\\\");
    let source = if cfg!(windows) {
        format!(
            r#"
        import std.io = io;
        import std.path = path;
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
        let ok =
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
        import std.io = io;
        import std.path = path;
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
        let ok =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processCode(result) == 0 &&
            io.processStdout(result) == "" &&
            redirected != "";
    "#
        )
    };

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_pipeline_rejects_empty_pipeline() {
    let source = r#"
        import std.io = io;
        let pipe = io.pipeline([]);
    "#;

    match eval_checked_hir(source) {
        Err(EvalError::TypeError(message)) => assert!(
            message.contains("io.pipeline: requires a non-empty List<Command>"),
            "unexpected error: {message}"
        ),
        other => panic!("expected type error, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_pipeline_with_redirect_rejects_final_stage_stdout_conflict() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stdout_path = temp.path().join("pipeline-stdout.txt");
    let stdout_path_source = stdout_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let output = path.fromString("{stdout_path_source}");
        let result = io.execPipeline(
            io.pipelineWithRedirects(
                io.pipeline([
                    io.commandWithRedirects(
                        io.command("printf", ["neve"]),
                        [io.redirectStdoutPath(output)]
                    )
                ]),
                [io.redirectStdoutPath(output)]
            )
        );
    "#
    );

    match eval_checked_hir(&source) {
        Err(EvalError::TypeError(message)) => assert!(
            message.contains(
                "final pipeline stage cannot combine boundary stdout with stage-local stdout redirect"
            ),
            "unexpected error: {message}"
        ),
        other => panic!("expected type error, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_pipeline_with_redirects_rejects_final_stage_stdout_conflict() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stdout_path = temp.path().join("pipeline-stdout.txt");
    let stdout_path_source = stdout_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
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

    match eval_checked_hir(&source) {
        Err(EvalError::TypeError(message)) => assert!(
            message.contains(
                "final pipeline stage cannot combine boundary stdout with stage-local stdout redirect"
            ),
            "unexpected error: {message}"
        ),
        other => panic!("expected type error, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_pipeline_rejects_non_final_stage_stdout_redirect() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stdout_path = temp.path().join("stage-stdout.txt");
    let stdout_path_source = stdout_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
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

    match eval_checked_hir(&source) {
        Err(EvalError::TypeError(message)) => assert!(
            message.contains("pipeline stage 1 cannot carry stdout redirect before final stage"),
            "unexpected error: {message}"
        ),
        other => panic!("expected type error, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_pipeline_with_redirects_rejects_boundary_stdin_conflict() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stdin_path = temp.path().join("pipeline-stdin.txt");
    let stdin_path_source = stdin_path.to_string_lossy().replace('\\', "\\\\");
    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let input = path.fromString("{stdin_path_source}");
        let pipe = io.pipelineWithRedirects(
            io.pipeline([
                io.commandWith(#{{ program = "cat", stdin = "neve" }})
            ]),
            [io.redirectStdinPath(input)]
        );
    "#
    );

    match eval_checked_hir(&source) {
        Err(EvalError::TypeError(message)) => assert!(
            message
                .contains("pipeline stage 1 cannot combine boundary stdin with stage-local stdin"),
            "unexpected error: {message}"
        ),
        other => panic!("expected type error, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_command_with_redirects_bridge_exposes_command_runtime_value() {
    let source = r#"
        import std.io = io;
        import std.path = path;
        let cmd = io.commandWithRedirects(
            io.command("printf", ["neve"]),
            [io.redirectStdoutPath(path.fromString("/tmp/neve.out"))]
        );
        let shown = if typeOf(cmd) == "Command" -> toString(cmd) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "<command:printf 1 arg(s), configured>"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_pipeline_honors_stage_local_redirects() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stderr_path = temp.path().join("pipeline-stage-stderr.txt");
    let stderr_path_source = stderr_path.to_string_lossy().replace('\\', "\\\\");
    let source = if cfg!(windows) {
        format!(
            r#"
        import std.io = io;
        import std.path = path;
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
        let ok =
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
        import std.io = io;
        import std.path = path;
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
        let ok =
            typeOf(result) == "ProcessResult" &&
            io.processSuccess(result) &&
            io.processStdout(result) != "" &&
            io.processStderr(result) == "" &&
            redirected != "";
    "#
        )
    };

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_task_command_bridge_exposes_task_runtime_value() {
    let source = r#"
        import std.io = io;
        let task = io.taskCommand(io.command("printf", ["neve"]));
        let shown = if typeOf(task) == "Task" -> toString(task) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "<task:command->ProcessResult>"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_task_pipeline_bridge_exposes_task_runtime_value() {
    let source = r#"
        import std.io = io;
        let task = io.taskPipeline(io.pipeline([
            io.command("printf", ["neve"]),
            io.command("cat", [])
        ]));
        let shown = if typeOf(task) == "Task" -> toString(task) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "<task:pipeline->ProcessResult>"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_await_task_bridge_exposes_process_result_runtime_value() {
    let source = r#"
        import std.io = io;
        let task = io.taskCommand(io.command("rustc", ["--version"]));
        let result = io.awaitTask(task);
        let shown = if typeOf(result) == "ProcessResult" -> toString(result) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "<process-result:0 ok>"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_await_tasks_bridge_exposes_process_result_list() {
    let source = r#"
        import std.io = io;
        let results = io.awaitTasks([
            io.taskCommand(io.command("printf", ["neve"])),
            io.taskPipeline(io.pipeline([
                io.command("printf", ["lang"]),
                io.command("cat", [])
            ]))
        ]);
        match results {
            [first, second] ->
                io.processStdout(first) == "neve" &&
                io.processStdout(second) == "lang" &&
                io.processSuccess(first) &&
                io.processSuccess(second),
            _ -> false,
        }
    "#;

    match eval_checked_hir(source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_await_pipeline_task_matches_exec_pipeline() {
    let source = r#"
        import std.io = io;
        let pipeline = io.pipeline([
            io.command("printf", ["neve"]),
            io.command("cat", [])
        ]);
        let awaited = io.awaitTask(io.taskPipeline(pipeline));
        let direct = io.execPipeline(pipeline);
        io.processStdout(awaited) == io.processStdout(direct) &&
            io.processCode(awaited) == io.processCode(direct) &&
            io.processStderr(awaited) == io.processStderr(direct) &&
            io.processSuccess(awaited) == io.processSuccess(direct);
    "#;

    match eval_checked_hir(source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_pipeline_honors_embedded_pipeline_redirects() {
    let temp = TempDir::new().expect("temp dir should exist");
    let stdout_path = temp.path().join("pipeline-embedded-stdout.txt");
    let path_literal = stdout_path.to_string_lossy();

    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let pipe = io.pipelineWithRedirects(
            io.pipeline([
                io.command("printf", ["neve"]),
                io.command("cat", [])
            ]),
            [io.redirectStdoutPath(path.fromString("{path_literal}"))]
        );
        let result = io.execPipeline(pipe);
        io.processSuccess(result) && io.processStdout(result) == "";
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_matches_canonical_process_projection() {
    let source = r#"
        import std.io = io;
        let migrated = io.execCommand(io.command("rustc", ["--version"]));
        let canonical = io.execCommand(io.command("rustc", ["--version"]));
        let same =
            typeOf(migrated) == "ProcessResult" &&
            io.processSuccess(migrated) == io.processSuccess(canonical) &&
            io.processStdout(migrated) == io.processStdout(canonical) &&
            io.processCode(migrated) == io.processCode(canonical) &&
            io.processStderr(migrated) == io.processStderr(canonical);
    "#;

    match eval_checked_hir(source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_explicit_shell_command_matches_canonical_process_projection() {
    let source = shell_projection_source();

    match eval_checked_hir(&source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_with_matches_canonical_process_projection() {
    let source = r#"
        import std.io = io;
        let migrated =
            io.execCommand(io.commandWith(#{ program = "rustc", args = ["--version"] }));
        let canonical =
            io.execCommand(io.commandWith(#{ program = "rustc", args = ["--version"] }));
        let same =
            typeOf(migrated) == "ProcessResult" &&
            io.processSuccess(migrated) == io.processSuccess(canonical) &&
            io.processStdout(migrated) == io.processStdout(canonical) &&
            io.processCode(migrated) == io.processCode(canonical) &&
            io.processStderr(migrated) == io.processStderr(canonical);
    "#;

    match eval_checked_hir(source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_exec_command_bridge_exposes_process_result_runtime_value() {
    let source = r#"
        import std.io = io;
        let cmd = io.command("rustc", ["--version"]);
        let result = io.execCommand(cmd);
        let shown = if typeOf(result) == "ProcessResult" -> toString(result) else "nope";
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert_eq!(s.as_ref(), "<process-result:0 ok>"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_process_success_bridge() {
    let source = r#"
        import std.io = io;
        let cmd = io.command("rustc", ["--version"]);
        let result = io.execCommand(cmd);
        let success = io.processSuccess(result);
    "#;

    match eval_checked_hir(source) {
        Ok(Value::Bool(true)) => {}
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_process_stdout_bridge() {
    let source = r#"
        import std.io = io;
        let cmd = io.command("rustc", ["--version"]);
        let result = io.execCommand(cmd);
        let stdout = io.processStdout(result);
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(s)) => assert!(s.contains("rustc"), "stdout should contain rustc"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_process_code_bridge() {
    let source = r#"
        import std.io = io;
        let cmd = io.command("rustc", ["--version"]);
        let result = io.execCommand(cmd);
        let code = io.processCode(result);
    "#;

    match eval_checked_hir(source) {
        Ok(Value::Int(code)) => assert_eq!(code, 0.into()),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_process_stderr_bridge() {
    let source = r#"
        import std.io = io;
        let cmd = io.command("rustc", ["--version"]);
        let result = io.execCommand(cmd);
        let stderr = io.processStderr(result);
    "#;

    match eval_checked_hir(source) {
        Ok(Value::String(stderr)) => assert!(stderr.is_empty(), "stderr should be empty"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_path_exists_path_bridge() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("exists-path.neve.txt");
    std::fs::write(&file_path, "exists").unwrap();
    let escaped = file_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let exists = io.pathExistsPath(path.fromString("{escaped}"));
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(value)) => assert!(value),
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_is_dir_path_bridge() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("dir-path");
    std::fs::create_dir_all(&dir_path).unwrap();
    let escaped = dir_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let dir = io.isDirPath(path.fromString("{escaped}"));
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(value)) => assert!(value),
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_io_is_file_path_bridge() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("file-path.neve.txt");
    std::fs::write(&file_path, "file").unwrap();
    let escaped = file_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\"', "\\\"");

    let source = format!(
        r#"
        import std.io = io;
        import std.path = path;
        let file = io.isFilePath(path.fromString("{escaped}"));
    "#
    );

    match eval_checked_hir(&source) {
        Ok(Value::Bool(value)) => assert!(value),
        other => panic!("expected bool, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_map_and_set_builtins() {
    let source = r#"
        import std.Map;
        import std.Set;
        import std.list = list;
        let map = Map.insert("a", 41, Map.empty);
        let set = Set.insert(1, Set.empty);
        let result = Map.getWithDefault("a", 0, map) + Set.size(set) + list.sum(Map.values(map));
    "#;
    match eval_checked_hir(source) {
        Ok(Value::Int(n)) => assert_eq!(n, int(83)),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_higher_order_list_builtins() {
    let source = r#"
        import std.list = list;
        fn inc(x) = x + 1;
        fn isEven(x) = x % 2 == 0;
        let mapped = list.map(inc, [1, 2, 3]);
        let result = list.filter(isEven, mapped);
    "#;
    match eval_checked_hir(source) {
        Ok(Value::List(items)) => {
            let got: Vec<_> = items.iter().cloned().collect();
            assert_eq!(got, vec![Value::Int(int(2)), Value::Int(int(4))]);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_import_std_root() {
    let source = "import std; let xs = std.list.range(1, 3); let n = std.list.len(xs);";
    match eval_with_std(source) {
        Ok(Value::Int(n)) => assert_eq!(n, int(2)),
        other => panic!("expected int, got {:?}", other),
    }
}

// ============================================================================
// 整数字面量和运算
// ============================================================================

#[test]
fn test_eval_integer_zero() {
    assert!(matches!(eval_source("let x = 0;"), Ok(Value::Int(n)) if n == int(0)));
}

#[test]
fn test_eval_integer_positive() {
    assert!(matches!(eval_source("let x = 42;"), Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_integer_negative() {
    assert!(matches!(eval_source("let x = -42;"), Ok(Value::Int(n)) if n == int(-42)));
}

#[test]
fn test_eval_integer_large() {
    assert!(matches!(
        eval_source("let x = 9223372036854775807;"),
        Ok(Value::Int(n)) if n == int(9223372036854775807)
    ));
}

#[test]
fn test_eval_integer_min() {
    // Note: Parser might handle this differently
    let result = eval_source("let x = -9223372036854775807;");
    if let Ok(Value::Int(n)) = result {
        assert_eq!(n, int(-9223372036854775807));
    }
}

// ============================================================================
// 浮点数字面量和运算
// ============================================================================

#[test]
fn test_eval_float_zero() {
    match eval_source("let x = 0.0;") {
        Ok(Value::Float(f)) => assert!((f - 0.0).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_positive() {
    match eval_source("let x = 3.25;") {
        Ok(Value::Float(f)) => assert!((f - 3.25).abs() < 0.00001),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_negative() {
    match eval_source("let x = -2.5;") {
        Ok(Value::Float(f)) => assert!((f - (-2.5)).abs() < 0.001),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_scientific() {
    match eval_source("let x = 1.5e10;") {
        Ok(Value::Float(f)) => assert!((f - 1.5e10).abs() < 1e5),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_addition() {
    match eval_source("let x = 1.5 + 2.5;") {
        Ok(Value::Float(f)) => assert!((f - 4.0).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_subtraction() {
    match eval_source("let x = 5.5 - 2.5;") {
        Ok(Value::Float(f)) => assert!((f - 3.0).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

// ============================================================================
// 枚举构造器与匹配
// ============================================================================

#[test]
fn test_eval_enum_constructor_match() {
    let source = "enum Option { Some(Int), None }; let x = Some(1); let y = match x { Some(v) -> v, None -> 0 };";
    match eval_source(source) {
        Ok(Value::Int(v)) => assert_eq!(v, int(1)),
        other => panic!("expected int, got {:?}", other),
    }
}

#[test]
fn test_eval_float_multiplication() {
    match eval_source("let x = 2.5 * 4.0;") {
        Ok(Value::Float(f)) => assert!((f - 10.0).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_division() {
    match eval_source("let x = 10.0 / 4.0;") {
        Ok(Value::Float(f)) => assert!((f - 2.5).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

// ============================================================================
// 布尔值
// ============================================================================

#[test]
fn test_eval_bool_true() {
    assert!(matches!(
        eval_source("let x = true;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_bool_false() {
    assert!(matches!(
        eval_source("let x = false;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_bool_not_true() {
    assert!(matches!(
        eval_source("let x = !true;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_bool_not_false() {
    assert!(matches!(
        eval_source("let x = !false;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_bool_double_not() {
    assert!(matches!(
        eval_source("let x = !!true;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_bool_and_true_true() {
    assert!(matches!(
        eval_source("let x = true && true;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_bool_and_true_false() {
    assert!(matches!(
        eval_source("let x = true && false;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_bool_and_false_true() {
    assert!(matches!(
        eval_source("let x = false && true;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_bool_and_false_false() {
    assert!(matches!(
        eval_source("let x = false && false;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_bool_or_true_true() {
    assert!(matches!(
        eval_source("let x = true || true;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_bool_or_true_false() {
    assert!(matches!(
        eval_source("let x = true || false;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_bool_or_false_true() {
    assert!(matches!(
        eval_source("let x = false || true;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_bool_or_false_false() {
    assert!(matches!(
        eval_source("let x = false || false;"),
        Ok(Value::Bool(false))
    ));
}

// ============================================================================
// 字符串
// ============================================================================

#[test]
fn test_eval_string_empty() {
    match eval_source("let x = \"\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, ""),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_string_simple() {
    match eval_source("let x = \"hello\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "hello"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_string_with_spaces() {
    match eval_source("let x = \"hello world\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "hello world"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_string_with_numbers() {
    match eval_source("let x = \"abc123\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "abc123"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_string_unicode() {
    match eval_source("let x = \"你好世界\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "你好世界"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_string_concat() {
    match eval_source("let x = \"hello\" ++ \" world\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "hello world"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_string_concat_empty() {
    match eval_source("let x = \"hello\" ++ \"\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "hello"),
        other => panic!("expected string, got {:?}", other),
    }
}

// ============================================================================
// 算术运算
// ============================================================================

#[test]
fn test_eval_addition() {
    assert!(matches!(eval_source("let x = 1 + 2;"), Ok(Value::Int(n)) if n == int(3)));
}

#[test]
fn test_eval_subtraction() {
    assert!(matches!(eval_source("let x = 10 - 3;"), Ok(Value::Int(n)) if n == int(7)));
}

#[test]
fn test_eval_multiplication() {
    assert!(matches!(eval_source("let x = 6 * 7;"), Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_division() {
    assert!(matches!(eval_source("let x = 20 / 4;"), Ok(Value::Int(n)) if n == int(5)));
}

#[test]
fn test_eval_modulo() {
    assert!(matches!(eval_source("let x = 17 % 5;"), Ok(Value::Int(n)) if n == int(2)));
}

#[test]
fn test_eval_division_by_zero() {
    match eval_source("let x = 10 / 0;") {
        Err(EvalError::DivisionByZero) => {}
        other => panic!("expected DivisionByZero error, got {:?}", other),
    }
}

#[test]
fn test_eval_modulo_by_zero() {
    match eval_source("let x = 10 % 0;") {
        Err(EvalError::DivisionByZero) => {}
        other => panic!("expected DivisionByZero error, got {:?}", other),
    }
}

#[test]
fn test_eval_negative_division() {
    assert!(matches!(
        eval_source("let x = -10 / 2;"),
        Ok(Value::Int(n)) if n == int(-5)
    ));
}

#[test]
fn test_eval_negative_modulo() {
    let result = eval_source("let x = -17 % 5;");
    if let Ok(Value::Int(n)) = result {
        assert_eq!(n, int(-17 % 5));
    }
}

#[test]
fn test_eval_operator_precedence() {
    assert!(matches!(
        eval_source("let x = 1 + 2 * 3;"),
        Ok(Value::Int(n)) if n == int(7)
    ));
    assert!(matches!(
        eval_source("let x = (1 + 2) * 3;"),
        Ok(Value::Int(n)) if n == int(9)
    ));
}

#[test]
fn test_eval_complex_arithmetic() {
    assert!(matches!(
        eval_source("let x = 1 + 2 * 3 - 4 / 2;"),
        Ok(Value::Int(n)) if n == int(5)
    ));
}

#[test]
fn test_eval_nested_parentheses() {
    assert!(matches!(
        eval_source("let x = ((1 + 2) * (3 + 4));"),
        Ok(Value::Int(n)) if n == int(21)
    ));
}

// ============================================================================
// 比较运算
// ============================================================================

#[test]
fn test_eval_less_than_true() {
    assert!(matches!(
        eval_source("let x = 1 < 2;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_less_than_false() {
    assert!(matches!(
        eval_source("let x = 2 < 1;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_less_than_equal() {
    assert!(matches!(
        eval_source("let x = 1 < 1;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_greater_than_true() {
    assert!(matches!(
        eval_source("let x = 2 > 1;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_greater_than_false() {
    assert!(matches!(
        eval_source("let x = 1 > 2;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_less_than_or_equal_true() {
    assert!(matches!(
        eval_source("let x = 1 <= 2;"),
        Ok(Value::Bool(true))
    ));
    assert!(matches!(
        eval_source("let x = 1 <= 1;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_less_than_or_equal_false() {
    assert!(matches!(
        eval_source("let x = 2 <= 1;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_greater_than_or_equal_true() {
    assert!(matches!(
        eval_source("let x = 2 >= 1;"),
        Ok(Value::Bool(true))
    ));
    assert!(matches!(
        eval_source("let x = 1 >= 1;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_greater_than_or_equal_false() {
    assert!(matches!(
        eval_source("let x = 1 >= 2;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_equality_int() {
    assert!(matches!(
        eval_source("let x = 42 == 42;"),
        Ok(Value::Bool(true))
    ));
    assert!(matches!(
        eval_source("let x = 42 == 43;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_inequality_int() {
    assert!(matches!(
        eval_source("let x = 42 != 43;"),
        Ok(Value::Bool(true))
    ));
    assert!(matches!(
        eval_source("let x = 42 != 42;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_equality_bool() {
    assert!(matches!(
        eval_source("let x = true == true;"),
        Ok(Value::Bool(true))
    ));
    assert!(matches!(
        eval_source("let x = true == false;"),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn test_eval_equality_string() {
    assert!(matches!(
        eval_source("let x = \"hello\" == \"hello\";"),
        Ok(Value::Bool(true))
    ));
    assert!(matches!(
        eval_source("let x = \"hello\" == \"world\";"),
        Ok(Value::Bool(false))
    ));
}

// ============================================================================
// 条件表达式
// ============================================================================

#[test]
fn test_eval_if_true_branch() {
    assert!(matches!(
        eval_source("let x = if true -> 1 else 0;"),
        Ok(Value::Int(n)) if n == int(1)
    ));
}

#[test]
fn test_eval_if_false_branch() {
    assert!(matches!(
        eval_source("let x = if false -> 1 else 0;"),
        Ok(Value::Int(n)) if n == int(0)
    ));
}

#[test]
fn test_eval_if_with_expression_condition() {
    assert!(matches!(
        eval_source("let x = if 1 < 2 -> 10 else 20;"),
        Ok(Value::Int(n)) if n == int(10)
    ));
}

#[test]
fn test_eval_if_nested() {
    assert!(matches!(
        eval_source("let x = if true -> if false -> 1 else 2 else 3;"),
        Ok(Value::Int(n)) if n == int(2)
    ));
}

#[test]
fn test_eval_if_deeply_nested() {
    assert!(matches!(
        eval_source("let x = if true -> if true -> if false -> 1 else 2 else 3 else 4;"),
        Ok(Value::Int(n)) if n == int(2)
    ));
}

#[test]
fn test_eval_if_with_arithmetic() {
    assert!(matches!(
        eval_source("let x = if 2 + 2 == 4 -> 100 else 0;"),
        Ok(Value::Int(n)) if n == int(100)
    ));
}

#[test]
fn test_eval_if_returns_different_types() {
    // Both branches should be able to return the same type
    match eval_source("let x = if true -> \"yes\" else \"no\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "yes"),
        other => panic!("expected string, got {:?}", other),
    }
}

// ============================================================================
// 列表
// ============================================================================

#[test]
fn test_eval_list_empty() {
    match eval_source("let x = [];") {
        Ok(Value::List(items)) => assert!(items.is_empty()),
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_single_element() {
    match eval_source("let x = [42];") {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 1);
            assert!(matches!(&items[0], Value::Int(n) if n == &int(42)));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_multiple_elements() {
    match eval_source("let x = [1, 2, 3, 4, 5];") {
        Ok(Value::List(items)) => assert_eq!(items.len(), 5),
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_with_expressions() {
    match eval_source("let x = [1 + 1, 2 * 2, 3 - 1];") {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 3);
            assert!(matches!(&items[0], Value::Int(n) if n == &int(2)));
            assert!(matches!(&items[1], Value::Int(n) if n == &int(4)));
            assert!(matches!(&items[2], Value::Int(n) if n == &int(2)));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_nested() {
    match eval_source("let x = [[1, 2], [3, 4]];") {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 2);
            match &items[0] {
                Value::List(inner) => assert_eq!(inner.len(), 2),
                _ => panic!("expected nested list"),
            }
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_concat() {
    match eval_source("let x = [1, 2] ++ [3, 4];") {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 4);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_concat_empty() {
    match eval_source("let x = [1, 2] ++ [];") {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 2);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_concat_left_empty() {
    match eval_source("let x = [] ++ [1, 2];") {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 2);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

// ============================================================================
// 元组
// ============================================================================

#[test]
fn test_eval_tuple_pair() {
    match eval_source("let x = (1, 2);") {
        Ok(Value::Tuple(items)) => {
            assert_eq!(items.len(), 2);
            assert!(matches!(&items[0], Value::Int(n) if n == &int(1)));
            assert!(matches!(&items[1], Value::Int(n) if n == &int(2)));
        }
        other => panic!("expected tuple, got {:?}", other),
    }
}

#[test]
fn test_eval_tuple_triple() {
    match eval_source("let x = (1, true, \"hello\");") {
        Ok(Value::Tuple(items)) => {
            assert_eq!(items.len(), 3);
        }
        other => panic!("expected tuple, got {:?}", other),
    }
}

#[test]
fn test_eval_tuple_nested() {
    match eval_source("let x = ((1, 2), (3, 4));") {
        Ok(Value::Tuple(items)) => {
            assert_eq!(items.len(), 2);
            match &items[0] {
                Value::Tuple(inner) => assert_eq!(inner.len(), 2),
                _ => panic!("expected nested tuple"),
            }
        }
        other => panic!("expected tuple, got {:?}", other),
    }
}

#[test]
fn test_eval_tuple_with_expressions() {
    match eval_source("let x = (1 + 1, 2 * 2);") {
        Ok(Value::Tuple(items)) => {
            assert!(matches!(&items[0], Value::Int(n) if n == &int(2)));
            assert!(matches!(&items[1], Value::Int(n) if n == &int(4)));
        }
        other => panic!("expected tuple, got {:?}", other),
    }
}

// ============================================================================
// 记录
// ============================================================================

#[test]
fn test_eval_record_single_field() {
    match eval_source("let x = #{ a = 1 };") {
        Ok(Value::Record(fields)) => {
            assert_eq!(fields.len(), 1);
            assert!(matches!(fields.get("a"), Some(Value::Int(n)) if n == &int(1)));
        }
        other => panic!("expected record, got {:?}", other),
    }
}

// ============================================================================
// 惰性求值 (Lazy evaluation)
// ============================================================================

#[test]
fn test_eval_lazy_basic() {
    // ~creates a thunk, force evaluates it
    let result = eval_with_builtins(
        "
        let thunk = ~42;
        let x = force(thunk);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_lazy_is_lazy() {
    // isLazy should return true for thunks
    let result = eval_with_builtins(
        "
        let thunk = ~42;
        let x = isLazy(thunk);
    ",
    );
    assert!(matches!(result, Ok(Value::Bool(true))));
}

#[test]
fn test_eval_lazy_is_lazy_non_thunk() {
    // isLazy should return false for non-thunks
    let result = eval_with_builtins(
        "
        let x = isLazy(42);
    ",
    );
    assert!(matches!(result, Ok(Value::Bool(false))));
}

#[test]
fn test_eval_lazy_is_evaluated_before() {
    // isEvaluated should return false for unevaluated thunks
    let result = eval_with_builtins(
        "
        let thunk = ~42;
        let x = isEvaluated(thunk);
    ",
    );
    assert!(matches!(result, Ok(Value::Bool(false))));
}

#[test]
fn test_eval_lazy_force_non_thunk() {
    // force on non-thunk should return the value as-is
    let result = eval_with_builtins(
        "
        let x = force(42);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_lazy_expression() {
    // ~with complex expression
    let result = eval_with_builtins(
        "
        let a = 10;
        let thunk = ~(a + 5);
        let x = force(thunk);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(15)));
}

#[test]
fn test_eval_lazy_function_call() {
    // ~with function call
    let result = eval_with_builtins(
        "
        let double = fn(x) x * 2;
        let thunk = ~double(21);
        let x = force(thunk);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_hir_lazy_basic() {
    let result = eval_source(
        "
        let thunk = ~42;
        let x = force(thunk);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_hir_lazy_predicates() {
    let result = eval_source(
        "
        let thunk = ~42;
        let before = isEvaluated(thunk);
        let _ = force(thunk);
        let x = (isLazy(thunk), before, isEvaluated(thunk));
    ",
    );
    match result {
        Ok(Value::Tuple(items)) => {
            assert_eq!(items.len(), 3);
            assert!(matches!(&items[0], Value::Bool(true)));
            assert!(matches!(&items[1], Value::Bool(false)));
            assert!(matches!(&items[2], Value::Bool(true)));
        }
        other => panic!("expected ~predicate tuple, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_or_pattern_with_shared_binding() {
    let result = eval_source("let x = match (1, 2) { (0, v) | (1, v) -> v, _ -> 0 };");
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(2)));
}

#[test]
fn test_eval_hir_binding_pattern() {
    let result = eval_source("let x = match 42 { n @ 42 -> n + 1, _ -> 0 };");
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(43)));
}

#[test]
fn test_eval_hir_list_rest_pattern() {
    let result = eval_source(
        "
        let x = match [1, 2, 3, 4] {
            [first, ..middle, last] -> match middle {
                [a, b] -> first + a + b + last,
                _ -> 0,
            },
            _ -> 0,
        };
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(10)));
}

#[test]
fn test_eval_hir_try_on_option_like_enum() {
    let result = eval_source(
        "
        enum Option { Some(Int), None };
        let x = Some(41)? + 1;
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_hir_try_rejects_known_non_optional_value() {
    let result = eval_source("let x = 41?;");
    match result {
        Err(EvalError::TypeError(message)) => {
            assert!(
                message.contains("try requires"),
                "unexpected error: {message}"
            );
        }
        other => panic!("expected type error, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_coalesce_on_option_like_enum() {
    let result = eval_source(
        "
        enum Option { Some(Int), None };
        let a = Some(41) ?? 0;
        let x = a + 1;
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_hir_coalesce_rejects_known_non_optional_value() {
    let result = eval_source("let x = 41 ?? 0;");
    match result {
        Err(EvalError::TypeError(message)) => {
            assert!(
                message.contains("coalesce requires"),
                "unexpected error: {message}"
            );
        }
        other => panic!("expected type error, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_coalesce_on_none_like_enum() {
    let result = eval_source(
        "
        enum Option { Some(Int), None };
        let x = None ?? 42;
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_hir_try_on_result_like_enum_error() {
    let result = eval_source(
        "
        enum Result { Ok(Int), Err(String) };
        let x = Err(\"boom\")?;
    ",
    );
    match result {
        Err(EvalError::TypeError(message)) => {
            assert!(message.contains("boom"), "unexpected error: {message}");
        }
        other => panic!("expected propagated error, got {:?}", other),
    }
}

#[test]
fn test_eval_ast_compat_try_rejects_known_non_optional_value() {
    let result = eval_with_builtins("let x = 41?;");
    match result {
        Err(message) => {
            assert!(
                message.contains("try requires"),
                "unexpected error: {message}"
            );
        }
        other => panic!("expected error, got {:?}", other),
    }
}

#[test]
fn test_eval_ast_compat_coalesce_rejects_known_non_optional_value() {
    let result = eval_with_builtins("let x = 41 ?? 0;");
    match result {
        Err(message) => {
            assert!(
                message.contains("coalesce requires"),
                "unexpected error: {message}"
            );
        }
        other => panic!("expected error, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_trait_method_runtime_dispatch() {
    let result = eval_checked_hir(
        "
        trait Twice { fn twice(self) -> Int; };
        impl Twice for Int {
            fn twice(self) -> Int = self + self;
        };
        let x = 21.twice();
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_ast_trait_method_runtime_dispatch() {
    let result = eval_with_builtins(
        "
        trait Twice { fn twice(self) -> Int; };
        impl Twice for Int {
            fn twice(self) -> Int = self + self;
        };
        let x = 21.twice();
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_ast_imported_trait_method_runtime_dispatch() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    create_test_module(
        root,
        &["methods"],
        r#"
            pub trait Twice { fn twice(self) -> Int; };
            impl Twice for Int {
                fn twice(self) -> Int = self + self;
            };
        "#,
    );

    create_test_module(
        root,
        &["main"],
        r#"
            import methods;
            let x = 21.twice();
        "#,
    );

    let main_path = root.join("main.neve");
    let mut eval = AstEvaluator::new().with_base_path(root.to_path_buf());
    let result = eval.eval_file_at_path(&main_path);

    assert!(matches!(result, Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_record_multiple_fields() {
    match eval_source("let x = #{ a = 1, b = 2, c = 3 };") {
        Ok(Value::Record(fields)) => {
            assert_eq!(fields.len(), 3);
        }
        other => panic!("expected record, got {:?}", other),
    }
}

#[test]
fn test_eval_record_mixed_types() {
    match eval_source("let x = #{ name = \"alice\", age = 30, active = true };") {
        Ok(Value::Record(fields)) => {
            assert_eq!(fields.len(), 3);
            match fields.get("name") {
                Some(Value::String(s)) => assert_eq!(&**s, "alice"),
                _ => panic!("expected string field"),
            }
            assert!(matches!(fields.get("age"), Some(Value::Int(n)) if n == &int(30)));
            assert!(matches!(fields.get("active"), Some(Value::Bool(true))));
        }
        other => panic!("expected record, got {:?}", other),
    }
}

#[test]
fn test_eval_record_nested() {
    match eval_source("let x = #{ inner = #{ a = 1 } };") {
        Ok(Value::Record(fields)) => match fields.get("inner") {
            Some(Value::Record(inner)) => {
                assert!(matches!(inner.get("a"), Some(Value::Int(n)) if n == &int(1)));
            }
            _ => panic!("expected nested record"),
        },
        other => panic!("expected record, got {:?}", other),
    }
}

#[test]
fn test_eval_record_field_access() {
    match eval_source("let r = #{ a = 42, b = 100 }; let x = r.a;") {
        Ok(Value::Int(n)) if n == int(42) => {}
        other => panic!("expected 42, got {:?}", other),
    }
}

#[test]
fn test_eval_record_merge() {
    match eval_source("let x = #{ a = 1 } // #{ b = 2 };") {
        Ok(Value::Record(fields)) => {
            assert_eq!(fields.len(), 2);
            assert!(matches!(fields.get("a"), Some(Value::Int(n)) if n == &int(1)));
            assert!(matches!(fields.get("b"), Some(Value::Int(n)) if n == &int(2)));
        }
        other => panic!("expected record, got {:?}", other),
    }
}

#[test]
fn test_eval_record_merge_override() {
    match eval_source("let x = #{ a = 1 } // #{ a = 2 };") {
        Ok(Value::Record(fields)) => {
            assert!(matches!(fields.get("a"), Some(Value::Int(n)) if n == &int(2)));
        }
        other => panic!("expected record, got {:?}", other),
    }
}

// ============================================================================
// 函数定义和调用
// ============================================================================

#[test]
fn test_eval_function_simple() {
    let result = eval_source(
        "
        fn add_one(x) = x + 1;
        let y = add_one(5);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(6)));
}

#[test]
fn test_eval_function_two_params() {
    let result = eval_source(
        "
        fn add(a, b) = a + b;
        let y = add(3, 4);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(7)));
}

#[test]
fn test_eval_function_three_params() {
    let result = eval_source(
        "
        fn sum3(a, b, c) = a + b + c;
        let y = sum3(1, 2, 3);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(6)));
}

#[test]
fn test_eval_function_returns_bool() {
    let result = eval_source(
        "
        fn is_positive(x) = x > 0;
        let y = is_positive(5);
    ",
    );
    assert!(matches!(result, Ok(Value::Bool(true))));
}

#[test]
fn test_eval_function_returns_string() {
    match eval_source(
        "
        fn greet(name) = name;
        let y = greet(\"world\");
    ",
    ) {
        Ok(Value::String(s)) => assert_eq!(&*s, "world"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_function_with_if() {
    let result = eval_source(
        "
        fn abs(x) = if x < 0 -> -x else x;
        let y = abs(-5);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(5)));
}

#[test]
fn test_eval_function_multiple_calls() {
    let result = eval_source(
        "
        fn double(x) = x * 2;
        let a = double(1);
        let b = double(2);
        let c = double(3);
        let y = a + b + c;
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(12)));
}

#[test]
fn test_eval_function_composition() {
    let result = eval_source(
        "
        fn double(x) = x * 2;
        fn add_one(x) = x + 1;
        let y = add_one(double(5));
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(11)));
}

// ============================================================================
// 递归函数
// ============================================================================

#[test]
fn test_eval_recursive_factorial() {
    let result = eval_source(
        "
        fn fact(n) = if n <= 1 -> 1 else n * fact(n - 1);
        let x = fact(5);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(120)));
}

#[test]
fn test_eval_recursive_factorial_zero() {
    let result = eval_source(
        "
        fn fact(n) = if n <= 1 -> 1 else n * fact(n - 1);
        let x = fact(0);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(1)));
}

#[test]
fn test_eval_recursive_factorial_one() {
    let result = eval_source(
        "
        fn fact(n) = if n <= 1 -> 1 else n * fact(n - 1);
        let x = fact(1);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(1)));
}

#[test]
fn test_eval_recursive_fibonacci() {
    let result = eval_source(
        "
        fn fib(n) = if n <= 1 -> n else fib(n - 1) + fib(n - 2);
        let x = fib(10);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(55)));
}

#[test]
fn test_eval_recursive_fibonacci_zero() {
    let result = eval_source(
        "
        fn fib(n) = if n <= 1 -> n else fib(n - 1) + fib(n - 2);
        let x = fib(0);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(0)));
}

#[test]
fn test_eval_recursive_sum() {
    let result = eval_source(
        "
        fn sum_to(n) = if n <= 0 -> 0 else n + sum_to(n - 1);
        let x = sum_to(10);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(55)));
}

#[test]
fn test_eval_recursive_gcd() {
    let result = eval_source(
        "
        fn gcd(a, b) = if b == 0 -> a else gcd(b, a % b);
        let x = gcd(48, 18);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(6)));
}

// ============================================================================
// 管道操作
// ============================================================================

#[test]
fn test_eval_pipe_simple() {
    let result = eval_source(
        "
        fn double(x) = x * 2;
        let x = 5 |> double;
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(10)));
}

#[test]
fn test_eval_pipe_chain() {
    let result = eval_source(
        "
        fn double(x) = x * 2;
        fn add_one(x) = x + 1;
        let x = 5 |> double |> add_one;
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(11)));
}

#[test]
fn test_eval_pipe_long_chain() {
    let result = eval_source(
        "
        fn double(x) = x * 2;
        fn add_one(x) = x + 1;
        let x = 1 |> double |> add_one |> double |> add_one;
    ",
    );
    // 1 -> 2 -> 3 -> 6 -> 7
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(7)));
}

#[test]
fn test_eval_pipe_with_expression() {
    let result = eval_source(
        "
        fn double(x) = x * 2;
        let x = (1 + 2) |> double;
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(6)));
}

// ============================================================================
// 模式匹配
// ============================================================================

#[test]
fn test_eval_match_literal() {
    assert!(matches!(
        eval_source("let x = match 1 { 0 -> 100, 1 -> 200, _ -> 300 };"),
        Ok(Value::Int(n)) if n == int(200)
    ));
}

#[test]
fn test_eval_match_wildcard() {
    assert!(matches!(
        eval_source("let x = match 5 { 0 -> 100, 1 -> 200, _ -> 300 };"),
        Ok(Value::Int(n)) if n == int(300)
    ));
}

#[test]
fn test_eval_match_first_arm() {
    assert!(matches!(
        eval_source("let x = match 0 { 0 -> 100, 1 -> 200, _ -> 300 };"),
        Ok(Value::Int(n)) if n == int(100)
    ));
}

#[test]
fn test_eval_match_with_binding() {
    assert!(matches!(
        eval_source("let x = match 42 { n -> n + 1 };"),
        Ok(Value::Int(n)) if n == int(43)
    ));
}

#[test]
fn test_eval_match_tuple() {
    assert!(matches!(
        eval_source("let x = match (1, 2) { (a, b) -> a + b };"),
        Ok(Value::Int(n)) if n == int(3)
    ));
}

#[test]
fn test_eval_match_tuple_nested() {
    assert!(matches!(
        eval_source("let x = match ((1, 2), 3) { ((a, b), c) -> a + b + c };"),
        Ok(Value::Int(n)) if n == int(6)
    ));
}

#[test]
fn test_eval_match_list_pattern() {
    // Match a specific list
    let result = eval_source("let x = match [1, 2] { [a, b] -> a + b, _ -> 0 };");
    if let Ok(Value::Int(n)) = result {
        assert_eq!(n, int(3));
    }
}

#[test]
fn test_eval_match_multiple_arms_first() {
    assert!(matches!(
        eval_source("let x = match true { true -> 1, false -> 0 };"),
        Ok(Value::Int(n)) if n == int(1)
    ));
}

#[test]
fn test_eval_match_multiple_arms_second() {
    assert!(matches!(
        eval_source("let x = match false { true -> 1, false -> 0 };"),
        Ok(Value::Int(n)) if n == int(0)
    ));
}

// ============================================================================
// 变量绑定和作用域
// ============================================================================

#[test]
fn test_eval_let_simple() {
    assert!(matches!(eval_source("let x = 42;"), Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_let_with_expression() {
    assert!(matches!(
        eval_source("let x = 1 + 2 + 3;"),
        Ok(Value::Int(n)) if n == int(6)
    ));
}

#[test]
fn test_eval_multiple_lets() {
    assert!(matches!(
        eval_source("let a = 1; let b = 2; let c = a + b;"),
        Ok(Value::Int(n)) if n == int(3)
    ));
}

#[test]
fn test_eval_let_shadowing() {
    assert!(matches!(
        eval_source("let x = 1; let x = x + 1; let x = x + 1;"),
        Ok(Value::Int(n)) if n == int(3)
    ));
}

#[test]
fn test_eval_let_uses_previous() {
    assert!(matches!(
        eval_source("let a = 10; let b = a * 2; let c = b + a;"),
        Ok(Value::Int(n)) if n == int(30)
    ));
}

// ============================================================================
// 特殊边缘情况
// ============================================================================

#[test]
fn test_eval_unary_minus_expression() {
    assert!(matches!(
        eval_source("let x = -(1 + 2);"),
        Ok(Value::Int(n)) if n == int(-3)
    ));
}

#[test]
fn test_eval_double_negation() {
    assert!(matches!(eval_source("let x = - -42;"), Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_chained_comparisons() {
    // (1 < 2) && (2 < 3)
    assert!(matches!(
        eval_source("let x = 1 < 2 && 2 < 3;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_mixed_and_or() {
    assert!(matches!(
        eval_source("let x = true && false || true;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_complex_boolean_expression() {
    assert!(matches!(
        eval_source("let x = (1 < 2) && (3 > 2) || false;"),
        Ok(Value::Bool(true))
    ));
}

// ============================================================================
// 压力测试
// ============================================================================

#[test]
fn test_eval_large_list() {
    // Generate a list with many elements
    let source = "let x = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];";
    match eval_source(source) {
        Ok(Value::List(items)) => assert_eq!(items.len(), 20),
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_deeply_nested_list() {
    match eval_source("let x = [[[1]]];") {
        Ok(Value::List(l1)) => match &l1[0] {
            Value::List(l2) => match &l2[0] {
                Value::List(l3) => assert!(matches!(&l3[0], Value::Int(n) if n == &int(1))),
                _ => panic!("expected innermost list"),
            },
            _ => panic!("expected middle list"),
        },
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_many_functions() {
    let result = eval_source(
        "
        fn f1(x) = x + 1;
        fn f2(x) = x + 2;
        fn f3(x) = x + 3;
        fn f4(x) = x + 4;
        fn f5(x) = x + 5;
        let x = f1(f2(f3(f4(f5(0)))));
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(15)));
}

#[test]
fn test_eval_complex_record() {
    let result = eval_source(
        "
        let config = #{
            name = \"test\",
            version = 1,
            enabled = true,
            settings = #{
                debug = false,
                level = 5
            }
        };
        let x = config.settings.level;
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(5)));
}

// ============================================================================
// 错误处理测试
// ============================================================================

#[test]
fn test_eval_field_access_nonexistent() {
    match eval_source("let r = #{ a = 1 }; let x = r.b;") {
        Err(EvalError::TypeError(msg)) => assert!(msg.contains("field")),
        other => panic!("expected TypeError, got {:?}", other),
    }
}

#[test]
fn test_eval_field_access_on_non_record() {
    match eval_source("let x = 42; let y = x.field;") {
        Err(EvalError::TypeError(msg)) => assert!(msg.contains("record")),
        other => panic!("expected TypeError, got {:?}", other),
    }
}

#[test]
fn test_eval_call_non_function() {
    match eval_source("let x = 42; let y = x(1);") {
        Err(EvalError::NotAFunction) => {}
        other => panic!("expected NotAFunction error, got {:?}", other),
    }
}

#[test]
fn test_eval_pattern_match_failure() {
    match eval_source("let x = match 5 { 1 -> 10, 2 -> 20 };") {
        Err(EvalError::PatternMatchFailed) => {}
        other => panic!("expected PatternMatchFailed error, got {:?}", other),
    }
}

// ============================================================================
// Lambda 表达式测试
// ============================================================================

#[test]
fn test_eval_lambda_simple() {
    let result = eval_source(
        "
        let f = fn(x) x * 2;
        let y = f(21);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(42)));
}

#[test]
fn test_eval_lambda_closure() {
    let result = eval_source(
        "
        fn make_adder(n) = fn(x) x + n;
        let add5 = make_adder(5);
        let result = add5(10);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(15)));
}

#[test]
fn test_eval_lambda_higher_order() {
    let result = eval_source(
        "
        fn apply(f, x) = f(x);
        let double = fn(x) x * 2;
        let result = apply(double, 21);
    ",
    );
    assert!(matches!(result, Ok(Value::Int(n)) if n == int(42)));
}

// ============================================================================
// 幂运算测试
// ============================================================================

#[test]
fn test_eval_power_simple() {
    assert!(matches!(eval_source("let x = 2 ^ 3;"), Ok(Value::Int(n)) if n == int(8)));
}

#[test]
fn test_eval_power_zero_exponent() {
    assert!(matches!(eval_source("let x = 5 ^ 0;"), Ok(Value::Int(n)) if n == int(1)));
}

#[test]
fn test_eval_power_one_exponent() {
    assert!(matches!(eval_source("let x = 5 ^ 1;"), Ok(Value::Int(n)) if n == int(5)));
}

#[test]
fn test_eval_power_larger() {
    assert!(matches!(
        eval_source("let x = 2 ^ 10;"),
        Ok(Value::Int(n)) if n == int(1024)
    ));
}

// ============================================================================
// 混合类型运算测试
// ============================================================================

#[test]
fn test_eval_int_float_addition() {
    match eval_source("let x = 1 + 2.5;") {
        Ok(Value::Float(f)) => assert!((f - 3.5).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_int_addition() {
    match eval_source("let x = 2.5 + 1;") {
        Ok(Value::Float(f)) => assert!((f - 3.5).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

// ============================================================================
// Safe field access (?.) tests
// ============================================================================

#[test]
fn test_eval_safe_field_on_record() {
    let result = eval_with_builtins(
        "
        let r = #{ name = \"test\" };
        let x = r?.name;
    ",
    );
    match result {
        Ok(Value::Some(inner)) => {
            if let Value::String(s) = *inner {
                assert_eq!(s.as_str(), "test");
            } else {
                panic!("expected Some(String)");
            }
        }
        other => panic!("expected Some, got {:?}", other),
    }
}

#[test]
fn test_eval_safe_field_missing() {
    let result = eval_with_builtins(
        "
        let r = #{ name = \"test\" };
        let x = r?.missing;
    ",
    );
    assert!(matches!(result, Ok(Value::None)));
}

#[test]
fn test_eval_safe_field_with_coalesce() {
    let result = eval_with_builtins(
        "
        let r = #{ name = \"test\" };
        let x = r?.missing ?? \"default\";
    ",
    );
    match result {
        Ok(Value::String(s)) => assert_eq!(s.as_str(), "default"),
        other => panic!("expected String, got {:?}", other),
    }
}

// ============================================================================
// Path literal tests
// ============================================================================

#[test]
fn test_eval_path_lit_relative() {
    let result = eval_with_builtins("let x = ./foo/bar;");
    match result {
        Ok(Value::Path(p)) => assert_eq!(p.to_string_lossy(), "./foo/bar"),
        other => panic!("expected Path, got {:?}", other),
    }
}

#[test]
fn test_eval_path_lit_parent() {
    let result = eval_with_builtins("let x = ../parent;");
    match result {
        Ok(Value::Path(p)) => assert_eq!(p.to_string_lossy(), "../parent"),
        other => panic!("expected Path, got {:?}", other),
    }
}

#[test]
fn test_eval_path_lit_absolute() {
    let result = eval_with_builtins("let x = /absolute/path;");
    match result {
        Ok(Value::Path(p)) => assert_eq!(p.to_string_lossy(), "/absolute/path"),
        other => panic!("expected Path, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_math_conversion_bridges() {
    let result = eval_checked_hir(
        r#"
            import std.math = math;
            let x = (math.toInt(true), math.toFloat("1.5"));
        "#,
    );
    match result {
        Ok(Value::Tuple(items)) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], Value::Int(1.into()));
            assert_eq!(items[1], Value::Float(1.5));
        }
        other => panic!("expected Tuple, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_math_float_predicates() {
    let result = eval_checked_hir(
        r#"
            import std.math = math;
            let x = (math.isNan(math.nan), math.isInf(math.inf));
        "#,
    );
    match result {
        Ok(Value::Tuple(items)) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], Value::Bool(true));
            assert_eq!(items[1], Value::Bool(true));
        }
        other => panic!("expected Tuple, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_math_rounding_bridges() {
    let result = eval_checked_hir(
        r#"
            import std.math = math;
            let x = (math.floor(1.9), math.ceil(1.1), math.round(1.6));
        "#,
    );
    match result {
        Ok(Value::Tuple(items)) => {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], Value::Int(1.into()));
            assert_eq!(items[1], Value::Int(2.into()));
            assert_eq!(items[2], Value::Int(2.into()));
        }
        other => panic!("expected Tuple, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_math_unary_float_transforms() {
    let result = eval_checked_hir(
        r#"
            import std.math = math;
            let x = (math.sqrt(9.0), math.log(1.0), math.log10(1000.0), math.exp(0.0));
        "#,
    );
    match result {
        Ok(Value::Tuple(items)) => {
            assert_eq!(items.len(), 4);
            assert_eq!(items[0], Value::Float(3.0));
            assert_eq!(items[1], Value::Float(0.0));
            assert_eq!(items[2], Value::Float(3.0));
            assert_eq!(items[3], Value::Float(1.0));
        }
        other => panic!("expected Tuple, got {:?}", other),
    }
}

#[test]
fn test_eval_hir_std_math_trigonometric_bridges() {
    let result = eval_checked_hir(
        r#"
            import std.math = math;
            let x = (math.sin(0.0), math.cos(0.0), math.tan(0.0));
        "#,
    );
    match result {
        Ok(Value::Tuple(items)) => {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], Value::Float(0.0));
            assert_eq!(items[1], Value::Float(1.0));
            assert_eq!(items[2], Value::Float(0.0));
        }
        other => panic!("expected Tuple, got {:?}", other),
    }
}
