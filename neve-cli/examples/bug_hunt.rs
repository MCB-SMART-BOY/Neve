/// Run with: cargo run --example bug_hunt -- [args]
// Usage: ./scripts/bug-hunt.rs
//
// Runs security regression tests by executing neve scripts and
// verifying stdout/stderr/exit codes against expected values.
use std::process::{Command, exit};

struct TestCase {
    name: &'static str,
    source: &'static str,
    expected: &'static str,
    desc: &'static str,
}

fn run_neve(source: &str) -> (String, String, i32) {
    let tmp = std::env::temp_dir().join(format!("neve_bug_hunt_{}.neve", std::process::id()));
    std::fs::write(&tmp, source).expect("write tempfile");

    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "neve", "--", "run"])
        .arg(&tmp)
        .output()
        .expect("failed to execute neve");

    let _ = std::fs::remove_file(&tmp);

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let code = output.status.code().unwrap_or(-1);

    // Combine stdout and stderr for matching (neve uses print/println)
    let combined = if stdout.is_empty() {
        stderr.clone()
    } else {
        stdout
    };
    (combined, stderr, code)
}

fn check(test: &TestCase, bugs: &mut Vec<String>) {
    print!("  [{}] ... ", test.name);
    let (output, stderr, code) = run_neve(test.source);

    let out_lower = output.to_lowercase();
    let exp_lower = test.expected.to_lowercase();
    let ok = out_lower.contains(&exp_lower) || out_lower == exp_lower;

    if ok {
        println!("PASS  {}", test.desc);
    } else {
        let msg = format!(
            "BUG   {}: expected='{}', got='{}', stderr='{}'",
            test.name,
            test.expected,
            &output[..output.len().min(80)],
            &stderr[..stderr.len().min(200)]
        );
        bugs.push(msg);
        println!(
            "FAIL  {} — expected='{}', got='{}'",
            test.desc,
            test.expected,
            &output[..output.len().min(60)]
        );
    }
    let _ = code;
}

fn main() {
    let tests = vec![
        // H-1: stdin limits
        TestCase {
            name: "stdin-ok",
            source: r#"
import std.io as io;
let cmd = io.commandWith("cat", [], cwd=None, stdin="hi", env=#{});
let r = io.execCommand(cmd);
io.processStdout(r)
"#,
            expected: "hi",
            desc: "small stdin works",
        },
        TestCase {
            name: "no-stdin",
            source: r#"
import std.io as io;
let r = io.execCommand(io.command("echo", ["hello"]));
io.processStdout(r)
"#,
            expected: "hello",
            desc: "no stdin works",
        },
        // H-2: output capture
        TestCase {
            name: "capture-stdout",
            source: r#"
import std.io as io;
let r = io.execCommand(io.command("echo", ["hello world"]));
io.processStdout(r)
"#,
            expected: "hello world",
            desc: "stdout captured",
        },
        TestCase {
            name: "capture-stderr",
            source: r#"
import std.io as io;
let r = io.execCommand(io.command("sh", ["-c", "echo err>&2"]));
io.processStderr(r)
"#,
            expected: "err",
            desc: "stderr captured",
        },
        TestCase {
            name: "exit-code-0",
            source: r#"
import std.io as io;
let r = io.execCommand(io.command("true", []));
toString(io.processSuccess(r))
"#,
            expected: "true",
            desc: "exit 0 is success",
        },
        TestCase {
            name: "exit-code-1",
            source: r#"
import std.io as io;
let r = io.execCommand(io.command("false", []));
toString(io.processSuccess(r))
"#,
            expected: "false",
            desc: "exit 1 is failure",
        },
        // M-4: env filtering
        TestCase {
            name: "safe-env",
            source: r#"
import std.io as io;
let cmd = io.commandWith("sh", ["-c", "echo $FOO"], cwd=None, stdin=None, env=#{FOO="bar"});
let r = io.execCommand(cmd);
io.processStdout(r)
"#,
            expected: "bar",
            desc: "safe env passes",
        },
        TestCase {
            name: "ld-preload-stripped",
            source: r#"
import std.io as io;
let cmd = io.commandWith("sh", ["-c", "test -z $LD_PRELOAD && echo stripped || echo LEAK"], cwd=None, stdin=None, env=#{LD_PRELOAD="/tmp/evil.so"});
let r = io.execCommand(cmd);
io.processStdout(r)
"#,
            expected: "stripped",
            desc: "LD_PRELOAD stripped from child env",
        },
        // spawn/await lifecycle
        TestCase {
            name: "spawn-await",
            source: r#"
import std.io as io;
let c = io.command("echo", ["spawned"]);
let t = io.taskCommand(c);
let r = io.awaitTask(t);
io.processStdout(r)
"#,
            expected: "spawned",
            desc: "spawn/await works",
        },
    ];

    println!("=== Bug Hunt: Security Regression Tests (Rust) ===");
    println!();

    let mut bugs = Vec::new();
    for test in &tests {
        check(test, &mut bugs);
    }

    println!();
    if bugs.is_empty() {
        println!("All {} tests passed. No bugs found.", tests.len());
    } else {
        println!("{} POTENTIAL BUGS FOUND:", bugs.len());
        for bug in &bugs {
            println!("  {bug}");
        }
        exit(1);
    }
}
