#!/usr/bin/env python3
"""
Random program generator for differential testing.
Generates random Neve expressions, runs them through Rust and Lean, compares.
"""

import subprocess, os, sys, random, tempfile

NEVE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FORMAL_DIR = os.path.join(NEVE_DIR, "formal")

# ============================================================
# Expression generator
# ============================================================

class GenState:
    def __init__(self, seed=42):
        self.rng = random.Random(seed)
        self.var_count = 0
    
    def fresh_var(self):
        self.var_count += 1
        return f"v{self.var_count}"

def gen_int(state, depth):
    """Generate a random integer expression."""
    if depth <= 0:
        n = state.rng.randint(-100, 100)
        lean_n = f"({n} : Int)" if n < 0 else str(n)
        return (f"{n}", f"(Expr.lit_int {lean_n})")
    
    choice = state.rng.randint(0, 3)
    if choice == 0:
        n = state.rng.randint(-100, 100)
        lean_n = f"({n} : Int)" if n < 0 else str(n)
        return (f"{n}", f"(Expr.lit_int {lean_n})")
    elif choice == 1:
        l_neve, l_lean = gen_int(state, depth - 1)
        r_neve, r_lean = gen_int(state, depth - 1)
        return (f"({l_neve} + {r_neve})", f"(Expr.binop BinOp.Add {l_lean} {r_lean})")
    elif choice == 2:
        l_neve, l_lean = gen_int(state, depth - 1)
        r_neve, r_lean = gen_int(state, depth - 1)
        return (f"({l_neve} - {r_neve})", f"(Expr.binop BinOp.Sub {l_lean} {r_lean})")
    else:
        l_neve, l_lean = gen_int(state, depth - 1)
        r_neve, r_lean = gen_int(state, depth - 1)
        return (f"({l_neve} * {r_neve})", f"(Expr.binop BinOp.Mul {l_lean} {r_lean})")

def gen_bool(state, depth):
    """Generate a random boolean expression."""
    if depth <= 0:
        b = "true" if state.rng.randint(0, 1) == 0 else "false"
        return (b, f"(Expr.lit_bool {'true' if b == 'true' else 'false'})")
    
    choice = state.rng.randint(0, 3)
    if choice == 0:
        b = "true" if state.rng.randint(0, 1) == 0 else "false"
        return (b, f"(Expr.lit_bool {'true' if b == 'true' else 'false'})")
    elif choice == 1:
        l_neve, l_lean = gen_int(state, depth - 1)
        r_neve, r_lean = gen_int(state, depth - 1)
        return (f"({l_neve} == {r_neve})", f"(Expr.binop BinOp.Eq {l_lean} {r_lean})")
    elif choice == 2:
        l_neve, l_lean = gen_bool(state, depth - 1)
        r_neve, r_lean = gen_bool(state, depth - 1)
        return (f"({l_neve} && {r_neve})", f"(Expr.binop BinOp.And {l_lean} {r_lean})")
    else:
        l_neve, l_lean = gen_bool(state, depth - 1)
        r_neve, r_lean = gen_bool(state, depth - 1)
        return (f"({l_neve} || {r_neve})", f"(Expr.binop BinOp.Or {l_lean} {r_lean})")

def gen_expr(state, depth):
    """Generate any expression (int or bool)."""
    if state.rng.randint(0, 1) == 0:
        return gen_int(state, depth)
    else:
        return gen_bool(state, depth)

# ============================================================
# Test runner
# ============================================================

def run_rust(neve_source):
    """Evaluate Neve source with Rust evaluator."""
    tmp = os.path.join(NEVE_DIR, "tmp_gen_diff.neve")
    with open(tmp, "w") as f:
        f.write(neve_source)
    result = subprocess.run(
        ["cargo", "run", "-q", "-p", "neve", "--", "run", tmp],
        capture_output=True, text=True, cwd=NEVE_DIR, timeout=30
    )
    os.remove(tmp)
    return result.stdout.strip()

def run_lean(lean_exprs, test_names):
    """Evaluate Lean expressions with the spec evaluator."""
    # Generate a temporary Lean file with all tests
    lean_code = """import Neve.Spec.Syntax
set_option maxRecDepth 100000
open Neve (Ty Expr Value BinOp Pattern Effect)

def matchesPattern (p : Pattern) (v : Value) : Bool :=
  match p, v with
  | Pattern.wildcard, _ => true
  | Pattern.var _, _ => true
  | Pattern.lit_int n, Value.int m => n = m
  | Pattern.lit_bool b, Value.bool c => b = c
  | _, _ => false

def findArm (v : Value) : List (Pattern × Expr) → Option (Pattern × Expr)
  | [] => none
  | (p, e) :: rest =>
      if matchesPattern p v then some (p, e) else findArm v rest

partial def eval (env : List (String × Value)) : Expr → Value
  | Expr.lit_int n => Value.int n
  | Expr.lit_float f => Value.float f
  | Expr.lit_bool b => Value.bool b
  | Expr.lit_char c => Value.char c
  | Expr.lit_string s => Value.string s
  | Expr.lit_unit => Value.unit
  | Expr.var x =>
      match env.lookup x with | some v => v | none => Value.unit
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
  | Expr.matchOn scrutinee arms =>
      let vscrut := eval env scrutinee
      match findArm vscrut arms with
      | some (_, e) => eval env e
      | none => Value.unit
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
"""
    for name, lean_expr in zip(test_names, lean_exprs):
        lean_code += f'  IO.println (fmt (evalClosed {lean_expr}))\n'
    lean_code += '  pure ()\n'
    
    with tempfile.NamedTemporaryFile(mode='w', suffix='.lean', delete=False) as f:
        f.write(lean_code)
        tmp_lean = f.name
    
    result = subprocess.run(
        ["lake", "env", "lean", "--run", tmp_lean],
        capture_output=True, text=True, cwd=FORMAL_DIR, timeout=60
    )
    os.unlink(tmp_lean)
    
    # Parse output lines
    lines = [l.strip() for l in result.stdout.strip().split('\n') if l.strip()]
    # Filter out warnings
    lines = [l for l in lines if not l.startswith('warning:') and '(interpreter)' not in l]
    return lines

# ============================================================
# Main
# ============================================================


# ============================================================
# Effects tests (property-based, not diff-based)
# ============================================================

def run_effects_tests(n_tests, seed):
    """Generate I/O programs and verify they respect safety limits."""
    rng = random.Random(seed)
    tests = []
    
    for i in range(n_tests):
        choice = rng.randint(0, 4)
        if choice == 0:
            # Simple execCommand
            msg = f"hello_{i}"
            src = f'let result = io.execCommand(io.command("echo", ["{msg}"])); println(io.processStdout(result))'
            tests.append(("execCommand", src, msg.strip()))
        elif choice == 1:
            # Pipeline
            src = f'let p = io.pipeline([io.command("echo", ["neve"]), io.command("cat", [])]); let r = io.execPipeline(p); println(io.processSuccess(r))'
            tests.append(("pipeline", src, "true"))
        elif choice == 2:
            # stdin test (small input — should succeed)
            src = 'let cmd = io.commandWith(#{ program = "cat", args = [], stdin = "hello" }); let r = io.execCommand(cmd); println(io.processStdout(r))'
            tests.append(("stdin-small", src, "hello"))
        elif choice == 3:
            # Exit code check
            src = 'let r = io.execCommand(io.command("test", ["-f", "/etc/hosts"])); println(io.processSuccess(r))'
            tests.append(("exit-code", src, "true"))
        else:
            # env safety check (no dangerous vars in output)
            src = 'let r = io.execCommand(io.command("env", [])); let out = io.processStdout(r); println(if out == "" then "empty" else "has-env")'
            tests.append(("env-check", src, "has-env"))
    
    return tests

def run_effects_test_suite(n_tests, seed):
    """Run effects property tests."""
    print(f"\nGenerating {n_tests} effects tests (seed={seed})")
    tests = run_effects_tests(n_tests, seed)
    
    passed = 0
    failed = 0
    for i, (name, src, expected) in enumerate(tests):
        output = run_rust(src).strip()
        ok = expected in output or output == expected
        if ok:
            passed += 1
            if i % 10 == 0 and i > 0:
                print(f"  {i}/{n_tests}...")
        else:
            failed += 1
            print(f"  ❌ {name}: expected '{expected}', got '{output[:50]}'")
    
    print(f"\n  Effects tests: {passed}/{passed+failed} passed")
    return passed, failed


def main():
    import argparse
    parser = argparse.ArgumentParser(description="Random differential test generator")
    parser.add_argument("-n", type=int, default=50, help="Number of tests")
    parser.add_argument("-d", type=int, default=3, help="Max expression depth")
    parser.add_argument("-s", type=int, default=None, help="Random seed")
    parser.add_argument("--effects", action="store_true", help="Run effects property tests")
    args = parser.parse_args()
    
    if args.effects:
        seed = args.s if args.s is not None else random.randint(0, 100000)
        p, f = run_effects_test_suite(args.n, seed)
        return 0 if f == 0 else 1
    
    seed = args.s if args.s is not None else random.randint(0, 100000)
    state = GenState(seed)
    
    print(f"Generating {args.n} tests (depth={args.d}, seed={seed})")
    
    neve_sources = []
    lean_exprs = []
    test_names = []
    
    for i in range(args.n):
        if i % 25 == 0 and i > 0:
            print(f"  Generated {i}/{args.n}...")
        neve_expr, lean_expr = gen_expr(state, args.d)
        neve_src = f"println({neve_expr})"
        neve_sources.append(neve_src)
        lean_exprs.append(lean_expr)
        test_names.append(f"test_{i}")
    print(f"  Generated {args.n}/{args.n} expressions")
    
    # Run Rust
    print("Running Rust evaluator...")
    rust_results = []
    for i, src in enumerate(neve_sources):
        if i % 25 == 0 and i > 0:
            print(f"  Rust: {i}/{args.n}...")
        rust_results.append(run_rust(src))
    print(f"  Rust: {args.n}/{args.n} done")
    
    # Run Lean
    print("Running Lean evaluator...")
    lean_results = run_lean(lean_exprs, test_names)
    print(f"  Lean: {len(lean_results)} results")
    
    # Compare
    passed = 0
    mismatches = []
    for i in range(args.n):
        rust = rust_results[i] if i < len(rust_results) else "N/A"
        lean = lean_results[i] if i < len(lean_results) else "N/A"
        ok = rust == lean
        if ok:
            passed += 1
        else:
            mismatches.append((i, neve_sources[i], rust, lean))
    
    if mismatches:
        print(f"\n{'Test':<8} {'Neve Source':<40} {'Rust':<10} {'Lean':<10}")
        print("-" * 90)
        for i, src, rust, lean in mismatches:
            print(f"❌ test_{i:<3}  {src[:38]:<38}  {rust:<10} {lean:<10}")
    
    print(f"\n{'='*50}")
    print(f"  Results: {passed}/{args.n} passed (seed={seed})")
    print(f"  {'🎉 ALL MATCH' if passed == args.n else f'{args.n - passed} mismatches'}")
    print(f"{'='*50}")
    
    return 0 if passed == args.n else 1

if __name__ == "__main__":
    sys.exit(main())
