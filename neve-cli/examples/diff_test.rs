/// Run with: cargo run --example diff_test -- [args]
// Usage: ./scripts/diff-test.rs
//
// Runs Neve expressions and verifies output matches expected values.
use std::process::{Command, exit};

struct Test {
    expr: &'static str,
    expected: &'static str,
    desc: &'static str,
}

fn run_neve_eval(expr: &str) -> String {
    let tmp = std::env::temp_dir().join(format!("neve_diff_{}.neve", std::process::id()));
    std::fs::write(&tmp, expr).expect("write tempfile");

    let output = Command::new("cargo")
        .args(["run", "-q", "-p", "neve", "--", "run"])
        .arg(&tmp)
        .output()
        .expect("failed to execute neve");

    let _ = std::fs::remove_file(&tmp);

    let stdout = String::from_utf8_lossy(&output.stdout)
        .trim()
        .trim_start_matches("✓ ")
        .trim_start_matches("[OK] ")
        .to_string();
    let stderr = String::from_utf8_lossy(&output.stderr)
        .trim()
        .trim_start_matches("✓ ")
        .trim_start_matches("[OK] ")
        .to_string();

    if stdout.is_empty() { stderr } else { stdout }
}

fn main() {
    let tests = vec![
        // Arithmetic
        Test {
            expr: "1 + 2",
            expected: "3",
            desc: "simple add",
        },
        Test {
            expr: "(3 + 4) * 2",
            expected: "14",
            desc: "add then mul",
        },
        Test {
            expr: "let x = 10 + 20; x * 2",
            expected: "60",
            desc: "let binding",
        },
        Test {
            expr: "40 + 2",
            expected: "42",
            desc: "the answer",
        },
        // Boolean
        Test {
            expr: "1 == 1",
            expected: "true",
            desc: "eq true",
        },
        Test {
            expr: "1 == 2",
            expected: "false",
            desc: "eq false",
        },
        Test {
            expr: "true && false",
            expected: "false",
            desc: "and false",
        },
        Test {
            expr: "true || false",
            expected: "true",
            desc: "or true",
        },
        // String
        Test {
            expr: r#""hello""#,
            expected: "\"hello\"",
            desc: "string literal",
        },
        // List
        Test {
            expr: "[1, 2, 3]",
            expected: "[1, 2, 3]",
            desc: "list literal",
        },
        // Function
        Test {
            expr: "let f = fn(x) { x + 1 }; f(41)",
            expected: "42",
            desc: "closure",
        },
        // Block
        Test {
            expr: "{ let x = 5; let y = 3; x + y }",
            expected: "8",
            desc: "block expr",
        },
    ];

    println!("=== Differential Test (Rust) ===");
    println!();

    let mut passed = 0;
    let mut failed = 0;

    for t in &tests {
        print!("  {} ... ", t.desc);
        let actual = run_neve_eval(t.expr);

        if actual == t.expected {
            println!("PASS");
            passed += 1;
        } else {
            println!("FAIL — expected='{}', got='{}'", t.expected, actual);
            failed += 1;
        }
    }

    println!();
    println!("{} passed, {} failed", passed, failed);

    if failed > 0 {
        exit(1);
    }
}
