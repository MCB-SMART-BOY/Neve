//! CLI contract tests for error classification and exit codes.
//! CLI 错误分类与退出码的契约测试。
//!
//! Contract (`n3v3-cli/src/main.rs`): `parse error` exits 2, `type error` exits 3,
//! `eval error`/`runtime error` exit 4, anything else exits 1.
//! 契约（`n3v3-cli/src/main.rs`）：`parse error` 退出 2，`type error` 退出 3，
//! `eval error`/`runtime error` 退出 4，其他退出 1。

use std::fs;
use std::process::Command;

/// Run the CLI binary on `source` written to a temporary `.n3v3` file.
/// 把 `source` 写入临时 `.n3v3` 文件并运行 CLI。
fn run_cli(source: &str, args: &[&str]) -> (i32, String) {
    let temp_dir = tempfile::TempDir::new().expect("temp dir should be created");
    let file = temp_dir.path().join("case.n3v3");
    fs::write(&file, source).expect("source should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_n3v3"))
        .args(args)
        .arg(&file)
        .output()
        .expect("n3v3 binary should run");

    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.code().unwrap_or(-1), combined)
}

/// A lexer error (invalid escape) is a syntax error, not a type error: it must
/// report `parse error` and exit 2.
/// 词法错误（非法转义）属于语法错误而非类型错误：必须报 `parse error` 并退出 2。
#[test]
fn test_lexer_error_is_reported_as_parse_error() {
    let (code, output) = run_cli("let x = \"a\\q\"\n", &["check"]);
    assert_eq!(
        code, 2,
        "lexer error must exit 2 (parse error), got {code}; output:\n{output}"
    );
    assert!(
        output.contains("parse error"),
        "lexer error must be reported as a parse error; output:\n{output}"
    );
    assert!(
        !output.contains("type error"),
        "lexer error must not be reported as a type error; output:\n{output}"
    );
    // The detail must survive, not just the count: `n3v3 run` used to lose a
    // lexer-only diagnostic because the entry filter only matched `Parser`.
    // 明细不能丢失，只剩计数：`n3v3 run` 曾因过滤条件只匹配 `Parser` 而丢掉纯词法诊断。
    assert!(
        output.contains("invalid escape sequence"),
        "lexer detail must be printed; output:\n{output}"
    );
}

/// A parser error still exits 2.
/// 语法错误仍然退出 2。
#[test]
fn test_parser_error_exits_two() {
    let (code, output) = run_cli("let x = \n", &["check"]);
    assert_eq!(code, 2, "parser error must exit 2; output:\n{output}");
    assert!(output.contains("parse error"), "output:\n{output}");
}

/// A type error exits 3.
/// 类型错误退出 3。
#[test]
fn test_type_error_exits_three() {
    let (code, output) = run_cli("let x: Int = \"text\"\n", &["check", "--allow-effects"]);
    assert_eq!(code, 3, "type error must exit 3; output:\n{output}");
    assert!(output.contains("type error"), "output:\n{output}");
}

/// `n3v3 run` uses the same exit-code mapping as `n3v3 check`: a lexer error is a
/// syntax error and must exit 2, not 1 or 3.
/// `n3v3 run` 与 `n3v3 check` 使用同一套退出码映射：词法错误属于语法错误，必须退出 2。
#[test]
fn test_run_lexer_error_exits_two() {
    let (code, output) = run_cli("let x = \"a\\q\"\n", &["run"]);
    assert_eq!(
        code, 2,
        "`n3v3 run` syntax error must exit 2; output:\n{output}"
    );
    assert!(output.contains("parse error"), "output:\n{output}");
    assert!(
        output.contains("invalid escape sequence"),
        "`n3v3 run` must print the lexer detail, not only the count; output:\n{output}"
    );
}

/// The effect gate lives in the CLI, so this is the only place the classification
/// of an effectful builtin can be observed: a pure function calling `io.tempDir`
/// must be rejected without `--allow-effects` and accepted with it.
/// 副作用闸门位于 CLI，因此只有在此处才能观察到副作用内置的分类：纯函数调用
/// `io.tempDir` 在无 `--allow-effects` 时必须被拒绝，加上该标志后必须通过。
#[test]
fn test_effectful_builtin_is_rejected_without_allow_effects() {
    let source = "use std.io = io;\nfn bad() -> Int = io.tempDir(fn(dir) { 42 });\n";

    let (code, output) = run_cli(source, &["check"]);
    assert!(
        output.contains("effectful call 'io.tempDir'"),
        "expected an effectful-call diagnostic; output:\n{output}"
    );
    assert_ne!(code, 0, "effect error must not exit 0; output:\n{output}");

    let (allowed_code, allowed_output) = run_cli(source, &["check", "--allow-effects"]);
    assert!(
        !allowed_output.contains("effectful call"),
        "--allow-effects must disable the effect gate; output:\n{allowed_output}"
    );
    assert_eq!(
        allowed_code, 0,
        "--allow-effects run must succeed; output:\n{allowed_output}"
    );
}
