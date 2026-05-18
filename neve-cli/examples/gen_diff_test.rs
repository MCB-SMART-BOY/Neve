/// Run with: cargo run --example gen_diff_test -- [args]
// Generates random Neve expressions, runs through Rust and Lean, compares.
//
// Compile: rustc scripts/gen-diff-test.rs -o /tmp/neve-gen-diff
// Usage: ./scripts/gen-diff-test.rs [-n 100] [-d 4] [-s 42] [--effects]
use std::env;
use std::fs;

use std::path::PathBuf;
use std::process::{Command, exit};

// ============================================================
// Simple xorshift64 RNG — zero dependencies
// ============================================================
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }
    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
    fn rand_int(&mut self, min: i64, max: i64) -> i64 {
        (self.next() % (max - min + 1) as u64) as i64 + min
    }
    fn rand_bool(&mut self) -> bool {
        self.next().is_multiple_of(2)
    }
    fn rand_choice(&mut self, n: usize) -> usize {
        (self.next() as usize) % n
    }
}

// ============================================================
// Expression generator
// ============================================================
struct GenState {
    rng: Rng,
}

impl GenState {
    fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed),
        }
    }

    fn gen_int(&mut self, depth: usize) -> (String, String) {
        if depth == 0 {
            let n = self.rng.rand_int(-100, 100);
            let ln = if n < 0 {
                format!("({} : Int)", n)
            } else {
                n.to_string()
            };
            return (n.to_string(), format!("(Expr.lit_int {})", ln));
        }
        match self.rng.rand_choice(4) {
            0 => {
                let n = self.rng.rand_int(-100, 100);
                let ln = if n < 0 {
                    format!("({} : Int)", n)
                } else {
                    n.to_string()
                };
                (n.to_string(), format!("(Expr.lit_int {})", ln))
            }
            1 => {
                let (l, ll) = self.gen_int(depth - 1);
                let (r, rl) = self.gen_int(depth - 1);
                (
                    format!("({l} + {r})"),
                    format!("(Expr.binop BinOp.Add {ll} {rl})"),
                )
            }
            2 => {
                let (l, ll) = self.gen_int(depth - 1);
                let (r, rl) = self.gen_int(depth - 1);
                (
                    format!("({l} - {r})"),
                    format!("(Expr.binop BinOp.Sub {ll} {rl})"),
                )
            }
            _ => {
                let (l, ll) = self.gen_int(depth - 1);
                let (r, rl) = self.gen_int(depth - 1);
                (
                    format!("({l} * {r})"),
                    format!("(Expr.binop BinOp.Mul {ll} {rl})"),
                )
            }
        }
    }

    fn gen_bool(&mut self, depth: usize) -> (String, String) {
        if depth == 0 {
            let b = if self.rng.rand_bool() {
                "true"
            } else {
                "false"
            };
            return (b.to_string(), format!("(Expr.lit_bool {b})"));
        }
        match self.rng.rand_choice(4) {
            0 => {
                let b = if self.rng.rand_bool() {
                    "true"
                } else {
                    "false"
                };
                (b.to_string(), format!("(Expr.lit_bool {b})"))
            }
            1 => {
                let (l, ll) = self.gen_int(depth - 1);
                let (r, rl) = self.gen_int(depth - 1);
                (
                    format!("({l} == {r})"),
                    format!("(Expr.binop BinOp.Eq {ll} {rl})"),
                )
            }
            2 => {
                let (l, ll) = self.gen_bool(depth - 1);
                let (r, rl) = self.gen_bool(depth - 1);
                (
                    format!("({l} && {r})"),
                    format!("(Expr.binop BinOp.And {ll} {rl})"),
                )
            }
            _ => {
                let (l, ll) = self.gen_bool(depth - 1);
                let (r, rl) = self.gen_bool(depth - 1);
                (
                    format!("({l} || {r})"),
                    format!("(Expr.binop BinOp.Or {ll} {rl})"),
                )
            }
        }
    }

    fn gen_expr(&mut self, depth: usize) -> (String, String) {
        if self.rng.rand_bool() {
            self.gen_int(depth)
        } else {
            self.gen_bool(depth)
        }
    }
}

// ============================================================
// Test runner
// ============================================================

fn project_dir() -> PathBuf {
    let this_file = PathBuf::from(file!());
    // scripts/gen-diff-test.rs -> scripts/.. -> project root
    this_file
        .parent()
        .unwrap_or(&PathBuf::from("."))
        .parent()
        .unwrap_or(&PathBuf::from("."))
        .to_path_buf()
}

fn neve_bin() -> PathBuf {
    let release = project_dir().join("target/release/neve");
    if release.exists() {
        release
    } else {
        PathBuf::from("cargo")
    }
}

fn formal_dir() -> PathBuf {
    project_dir().join("formal")
}

fn run_rust(neve_source: &str) -> String {
    let tmp = env::temp_dir().join(format!("gen_diff_{}.neve", std::process::id()));
    fs::write(&tmp, neve_source).expect("write tempfile");

    let bin = neve_bin();
    let output = if bin.file_name().is_some_and(|n| n == "neve") {
        Command::new(&bin)
            .args(["run", tmp.to_str().unwrap()])
            .output()
    } else {
        Command::new("cargo")
            .args([
                "run",
                "-q",
                "-p",
                "neve",
                "--",
                "run",
                tmp.to_str().unwrap(),
            ])
            .output()
    };
    let _ = fs::remove_file(&tmp);

    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .trim()
            .trim_start_matches("[OK] ")
            .trim_start_matches("\u{2713} ")
            .to_string(),
        Err(e) => format!("RUST_ERR: {e}"),
    }
}

fn run_lean(lean_exprs: &[String], _test_names: &[String]) -> Vec<String> {
    let mut lean_code = String::from(
        r#"import Neve.Spec.Syntax
set_option maxRecDepth 100000
open Neve (Ty Expr Value BinOp Pattern Effect)

partial def eval (env : List (String × Value)) : Expr → Value
  | Expr.lit_int n => Value.int n
  | Expr.lit_float f => Value.float f
  | Expr.lit_bool b => Value.bool b
  | Expr.lit_char c => Value.char c
  | Expr.lit_string s => Value.string s
  | Expr.lit_unit => Value.unit
  | Expr.var _ => Value.unit
  | Expr.lam x body => Value.closure x body env
  | Expr.app f arg =>
      match eval env f with
      | Value.closure x body env' =>
          let varg := eval env arg
          eval ((x, varg) :: env') body
      | _ => Value.unit
  | Expr.letIn x val body =>
      let vval := eval env val
      eval ((x, vval) :: env) body
  | Expr.binop BinOp.Add l r =>
      match eval env l, eval env r with
      | Value.int n, Value.int m => Value.int (n + m)
      | _, _ => Value.unit
  | Expr.binop BinOp.Sub l r =>
      match eval env l, eval env r with
      | Value.int n, Value.int m => Value.int (n - m)
      | _, _ => Value.unit
  | Expr.binop BinOp.Mul l r =>
      match eval env l, eval env r with
      | Value.int n, Value.int m => Value.int (n * m)
      | _, _ => Value.unit
  | Expr.binop BinOp.Eq l r =>
      let vl := eval env l
      let vr := eval env r
      Value.bool (vl == vr)
  | Expr.binop BinOp.And l r =>
      match eval env l with
      | Value.bool false => Value.bool false
      | Value.bool true => eval env r
      | _ => Value.unit
  | Expr.binop BinOp.Or l r =>
      match eval env l with
      | Value.bool true => Value.bool true
      | Value.bool false => eval env r
      | _ => Value.unit
  | _ => Value.unit

def evalClosed (e : Expr) : Value := eval [] e

def fmt (v : Value) : String :=
  match v with
  | Value.int n => toString n
  | Value.bool true => "true"
  | Value.bool false => "false"
  | Value.unit => "()"
  | _ => "<complex>"

def main : IO Unit := do
"#,
    );

    for expr in lean_exprs {
        lean_code.push_str(&format!("  IO.println (fmt (evalClosed {}))\n", expr));
    }
    lean_code.push_str("  pure ()\n");

    let tmp = env::temp_dir().join(format!("gen_lean_{}.lean", std::process::id()));
    fs::write(&tmp, &lean_code).expect("write lean");

    let output = Command::new("lake")
        .args(["env", "lean", "--run", tmp.to_str().unwrap()])
        .current_dir(formal_dir())
        .output();
    let _ = fs::remove_file(&tmp);

    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("warning:") && !l.contains("(interpreter)"))
            .map(|l| l.to_string())
            .collect(),
        Err(e) => vec![format!("LEAN_ERR: {e}")],
    }
}

// ============================================================
// Effects property tests
// ============================================================

struct EffectTest {
    name: String,
    src: String,
    expected: String,
}

fn generate_effects_tests(n: usize, seed: u64) -> Vec<EffectTest> {
    let mut rng = Rng::new(seed);
    let mut tests = Vec::new();
    for i in 0..n {
        match rng.rand_choice(5) {
            0 => {
                let msg = format!("hello_{i}");
                tests.push(EffectTest { name: "execCommand".into(),
                    src: format!("let result = io.execCommand(io.command(\"echo\", [\"{msg}\"])); io.processStdout(result)"),
                    expected: msg });
            }
            1 => tests.push(EffectTest { name: "pipeline".into(),
                src: "let p = io.pipeline([io.command(\"echo\", [\"neve\"]), io.command(\"cat\", [])]); let r = io.execPipeline(p); toString(io.processSuccess(r))".into(),
                expected: "true".into() }),
            2 => tests.push(EffectTest { name: "stdin-small".into(),
                src: "let cmd = io.commandWith(\"cat\", [], stdin=\"hello\", env=#{}); let r = io.execCommand(cmd); io.processStdout(r)".into(),
                expected: "hello".into() }),
            3 => tests.push(EffectTest { name: "exit-code".into(),
                src: "let r = io.execCommand(io.command(\"true\", [])); toString(io.processSuccess(r))".into(),
                expected: "true".into() }),
            _ => tests.push(EffectTest { name: "env-check".into(),
                src: "let r = io.execCommand(io.command(\"env\", [])); let out = io.processStdout(r); if out == \"\" then \"empty\" else \"has-env\"".into(),
                expected: "has-env".into() }),
        }
    }
    tests
}

fn run_effects_suite(n: usize, seed: u64) -> (usize, usize) {
    println!("\nGenerating {n} effects tests (seed={seed})");
    let tests = generate_effects_tests(n, seed);
    let mut passed = 0;
    let mut failed = 0;
    for (i, t) in tests.iter().enumerate() {
        let output = run_rust(&t.src);
        if output.contains(&t.expected) || output == t.expected {
            passed += 1;
            if i % 10 == 0 && i > 0 {
                println!("  {i}/{n}...");
            }
        } else {
            failed += 1;
            let s = if output.len() > 50 {
                &output[..50]
            } else {
                &output
            };
            println!(
                "  ❌ {name}: expected '{exp}', got '{s}'",
                name = t.name,
                exp = t.expected
            );
        }
    }
    println!("\n  Effects: {passed}/{} passed", passed + failed);
    (passed, failed)
}

// ============================================================
// Main
// ============================================================

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut n_tests = 50usize;
    let mut depth = 3usize;
    let mut seed: Option<u64> = None;
    let mut effects = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                i += 1;
                if i < args.len() {
                    n_tests = args[i].parse().unwrap_or(50);
                }
            }
            "-d" => {
                i += 1;
                if i < args.len() {
                    depth = args[i].parse().unwrap_or(3);
                }
            }
            "-s" => {
                i += 1;
                if i < args.len() {
                    seed = Some(args[i].parse().unwrap_or(42));
                }
            }
            "--effects" => {
                effects = true;
            }
            _ => {}
        }
        i += 1;
    }

    let seed = seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() % 100000)
            .unwrap_or(42)
    });

    if effects {
        let (_, f) = run_effects_suite(n_tests, seed);
        if f > 0 {
            exit(1);
        }
        return;
    }

    let mut state = GenState::new(seed);
    println!("Generating {n_tests} tests (depth={depth}, seed={seed})");

    let mut neve_sources = Vec::new();
    let mut lean_exprs = Vec::new();
    let mut test_names = Vec::new();
    for i in 0..n_tests {
        if i % 25 == 0 && i > 0 {
            println!("  Generated {i}/{n_tests}...");
        }
        let (neve_expr, lean_expr) = state.gen_expr(depth);
        neve_sources.push(neve_expr);
        lean_exprs.push(lean_expr);
        test_names.push(format!("test_{i}"));
    }
    println!("  Generated {n_tests}/{n_tests} expressions");

    println!("Running Rust evaluator...");
    let mut rust_results = Vec::new();
    for (i, src) in neve_sources.iter().enumerate() {
        if i % 25 == 0 && i > 0 {
            println!("  Rust: {i}/{n_tests}...");
        }
        rust_results.push(run_rust(src));
    }
    println!("  Rust: {n_tests}/{n_tests} done");

    println!("Running Lean evaluator...");
    let lean_results = run_lean(&lean_exprs, &test_names);
    println!("  Lean: {} results", lean_results.len());

    let mut passed = 0;
    #[allow(clippy::needless_range_loop)]
    let mut mismatches = Vec::new();
    #[allow(clippy::needless_range_loop)]
    for i in 0..n_tests {
        let rust = rust_results.get(i).map(|s| s.as_str()).unwrap_or("N/A");
        let lean = lean_results.get(i).map(|s| s.as_str()).unwrap_or("N/A");
        if rust == lean {
            passed += 1;
        } else {
            mismatches.push((i, &neve_sources[i], rust.to_string(), lean.to_string()));
        }
    }

    if !mismatches.is_empty() {
        println!(
            "\n{:<8} {:<40} {:<10} {:<10}",
            "Test", "Neve Source", "Rust", "Lean"
        );
        println!("{}", "=".repeat(90));
        for (i, src, rust, lean) in &mismatches {
            let s = if src.len() > 38 { &src[..38] } else { src };
            println!("❌ test_{:<3}  {:<38}  {:<10} {:<10}", i, s, rust, lean);
        }
    }

    println!("\n{}", "=".repeat(50));
    println!("  Results: {passed}/{n_tests} passed (seed={seed})");
    let msg = if passed == n_tests {
        "🎉 ALL MATCH".to_string()
    } else {
        format!("{} mismatches", n_tests - passed)
    };
    println!("  {msg}");
    println!("{}", "=".repeat(50));
    if passed != n_tests {
        exit(1);
    }
}
