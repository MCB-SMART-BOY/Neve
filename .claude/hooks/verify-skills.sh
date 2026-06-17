#!/usr/bin/env bash
# Verify that skill claims match actual code.
# Run after any code change that touches public APIs.
set -euo pipefail
cd "$(dirname "$0")/../.."

PASS=0
FAIL=0

check_file() {
    local label="$1" file="$2"
    if [ -f "$file" ]; then
        echo "  ✓ $label exists: $file"
        ((PASS++)) || true
    else
        echo "  ✗ $label MISSING: $file"
        ((FAIL++)) || true
    fi
}

check_count() {
    local label="$1" pattern="$2" file="$3" expected="$4"
    local actual
    actual=$(grep -c "$pattern" "$file" 2>/dev/null || echo "0")
    if [ "$actual" -ge "$expected" ]; then
        echo "  ✓ $label: $actual >= $expected"
        ((PASS++)) || true
    else
        echo "  ✗ $label: $actual < $expected (expected at least $expected)"
        ((FAIL++)) || true
    fi
}

echo "=== Skill Verification ==="

# --- neve-parser.md claims ---
check_file "lexer.rs"        crates/neve-lexer/src/lexer.rs
check_file "parser lib.rs"   crates/neve-parser/src/lib.rs
check_file "AST expr"        crates/neve-syntax/src/expr.rs
check_file "parser tests"    tests/parser.rs
check_count "parser tests count" '#\[test\]' tests/parser.rs 220

# --- neve-typeck.md claims ---
check_file "typeck mod.rs"   crates/neve-typeck/src/check/mod.rs
check_count "typeck tests"   '#\[test\]' tests/typeck.rs 280

# --- neve-eval.md claims ---
check_file "eval.rs"         crates/neve-eval/src/eval.rs
check_file "value.rs"        crates/neve-eval/src/value.rs
# ast_eval.rs removed in Phase D — all evaluation now through canonical HIR pipeline

# --- neve-std.md claims ---
check_file "std io.rs"       crates/neve-std/src/io/mod.rs
check_file "std lib.rs"      crates/neve-std/src/lib.rs

# --- neve-lsp.md claims ---
check_file "lsp lib.rs"      crates/neve-lsp/src/lib.rs

# --- neve-effect.md claims ---
check_file "effects.lean"    formal/Neve/Spec/Effects.lean

# --- neve-lean.md claims ---
check_file "lakefile.lean"   formal/lakefile.lean
check_count "lean modules"   '\.lean$' <(find formal -name '*.lean' 2>/dev/null | cat) 15

# --- neve-test.md claims ---
check_file "e2e tests"       tests/end_to_end.rs
check_count "e2e test count" '#\[test\]' tests/end_to_end.rs 540

echo ""
echo "=== Result: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] && echo "✓ All skill claims verified" || echo "✗ Skill claims out of date"
exit $FAIL
