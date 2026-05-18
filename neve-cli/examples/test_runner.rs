/// Run with: cargo run --example test_runner -- [args]
// Usage: ./scripts/test.rs [--clippy] [--all]
use std::process::{Command, exit};

fn run_step(name: &str, program: &str, args: &[&str]) -> bool {
    print!("  [{name}] ... ");
    let output = Command::new(program)
        .args(args)
        .output()
        .expect("failed to execute");

    if output.status.success() {
        println!("OK");
        true
    } else {
        println!("FAILED");
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        false
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let has_clippy = args.iter().any(|a| a == "--clippy");
    let has_all = args.iter().any(|a| a == "--all");

    println!("=== Neve Test Pipeline (Rust) ===");
    println!();

    // Step 1: check
    if !run_step("check", "cargo", &["check", "--workspace"]) {
        exit(1);
    }
    println!();

    // Step 2: test
    if !run_step("test", "cargo", &["test", "--workspace"]) {
        exit(1);
    }
    println!();

    // Step 3: fmt
    if !run_step("fmt", "cargo", &["fmt", "--all", "--", "--check"]) {
        println!("  Run 'cargo fmt --all' to fix formatting");
    }
    println!();

    // Optional: clippy
    if has_clippy || has_all {
        if !run_step(
            "clippy",
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ) {
            exit(1);
        }
        println!();
    }

    println!("=== All checks passed ===");
}
