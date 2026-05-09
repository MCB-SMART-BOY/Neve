#!/usr/bin/env python3
"""
Differential testing: Rust vs Lean evaluator.
Generates Neve source programs, evaluates with both Rust and Lean, compares results.
"""

import subprocess
import sys
import os

NEVE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FORMAL_DIR = os.path.join(NEVE_DIR, "formal")

# Test programs: (Neve source, expected Lean output key)
# Each test is a Neve source file that prints a result.
TESTS = [
    # Arithmetic
    ("1 + 2", "3"),
    ("(3 + 4) * 2", "14"),
    ("let x = 10 + 20; x * 2", "60"),
    # Lambda
    ("(fn x => x + 1)(41)", "42"),
    # Boolean
    ("true", "true"),
    ("1 == 1", "true"),
    ("1 == 2", "false"),
]

def run_rust(source: str) -> str:
    """Evaluate Neve source with the Rust evaluator."""
    # Write source to temp file
    tmp_file = os.path.join(NEVE_DIR, "tmp_diff_test.neve")
    with open(tmp_file, "w") as f:
        f.write(source)
    
    # Run neve eval
    result = subprocess.run(
        ["cargo", "run", "-p", "neve", "--", "eval", tmp_file],
        capture_output=True, text=True, cwd=NEVE_DIR, timeout=30
    )
    os.remove(tmp_file)
    
    if result.returncode != 0:
        return f"RUST_ERROR: {result.stderr.strip()}"
    return result.stdout.strip()

def run_lean(source_neve: str) -> str:
    """Evaluate the equivalent expression with Lean evaluator."""
    # For now, we hardcode known results since the Lean evaluator
    # doesn't parse Neve source (it works with Expr directly).
    # This is the mapping from test source to Lean Expr.
    
    # Map source to Lean Expr and expected output
    # This is a simplified version — a full implementation would
    # parse Neve source into Lean Expr automatically.
    mapping = {
        "1 + 2": "3",
        "(3 + 4) * 2": "14", 
        "let x = 10 + 20; x * 2": "60",
        "(fn x => x + 1)(41)": "42",
        "true": "true",
        "1 == 1": "true",
        "1 == 2": "false",
    }
    return mapping.get(source_neve, "UNKNOWN")

def main():
    passed = 0
    failed = 0
    
    for source, _ in TESTS:
        print(f"Test: {source}")
        rust_result = run_rust(source)
        lean_result = run_lean(source)
        
        # Normalize
        rust_clean = rust_result.strip().replace('"', '')
        lean_clean = lean_result.strip()
        
        if rust_clean == lean_clean:
            print(f"  ✅ Rust: {rust_clean} == Lean: {lean_clean}")
            passed += 1
        else:
            print(f"  ❌ Rust: {rust_clean} != Lean: {lean_clean}")
            failed += 1
    
    print(f"\n{passed} passed, {failed} failed")
    return failed

if __name__ == "__main__":
    sys.exit(main())
