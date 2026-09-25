#!/usr/bin/env bash
# n3v3 smoke-test driver — exercises every CLI path programmatically.
# Run from the repo root:  .claude/skills/run-n3v3/driver.sh [--release]
# Output: exit 0 if all checks pass; otherwise exit 1 with details on stderr.
set -euo pipefail

BIN="${CARGO_TARGET_DIR:-./target}/${1:-debug}/n3v3"
# Always build: an existing-but-stale binary would make this smoke test report on
# code that no longer exists. `cargo build` no-ops when the binary is fresh.
# 总是构建：已存在但过期的二进制会让本测试验证一份不存在的代码；新鲜时 cargo 是空操作。
cargo build -p n3v3 ${1:+--release} >/dev/null
if [ ! -x "$BIN" ]; then
  printf 'FAIL: binary %s missing after "cargo build -p n3v3"\n' "$BIN" >&2
  exit 1
fi

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "  ✓ $*"; }

echo "=== n3v3 Smoke Driver ==="
echo "binary: $BIN"
$BIN --version

# -- 1. eval --
echo ""
echo "--- eval ---"
out=$($BIN eval "1 + 2 * 3" 2>&1) || fail "eval crashed: $out"
echo "$out" | grep -q "7" || fail "eval: expected 7, got: $out"
pass "eval '1 + 2 * 3' = 7"

out=$($BIN eval '|x| x + 1' 2>&1) || fail "eval lambda crashed"
pass "eval lambda"

# -- 2. run file --
echo ""
echo "--- run ---"
cat > "$TMPDIR/smoke.n3v3" <<'EOF'
x = 42
f = |n| n * 2
result = f(x)
EOF
out=$($BIN run "$TMPDIR/smoke.n3v3" 2>&1) || fail "run crashed: $out"
echo "$out" | grep -q "84" || fail "run: expected 84 in output, got: $out"
pass "run smoke.n3v3 → 84"

# -- 3. check --
echo ""
echo "--- check ---"
out=$($BIN check "$TMPDIR/smoke.n3v3" 2>&1) || fail "check crashed: $out"
echo "$out" | grep -qi "ok" || fail "check: expected OK"
pass "check smoke.n3v3"

# -- 4. repl (piped) --
echo ""
echo "--- repl ---"
out=$(printf '1 + 2\n:quit\n' | $BIN repl 2>&1) || fail "repl crashed"
echo "$out" | grep -q "3" || fail "repl: expected 3 in output"
pass "repl: 1+2 = 3"

out=$(printf 'x = 10\nx * 2\n:quit\n' | $BIN repl 2>&1) || fail "repl multi-line crashed"
echo "$out" | grep -q "20" || fail "repl: expected 20 from multi-line"
pass "repl multi-line binding"

out=$(printf 'use std.list\nlist.map([1,2,3], |x| x*2)\n:quit\n' | $BIN repl 2>&1) || fail "repl stdlib crashed"
echo "$out" | grep -q '\[.*2.*4.*6.*\]' || fail "repl: expected [2,4,6] from list.map"
pass "repl std.list.map([1,2,3], |x| x*2)"

# -- 5. fmt --
echo ""
echo "--- fmt ---"
cp "$TMPDIR/smoke.n3v3" "$TMPDIR/fmt.n3v3"
$BIN fmt file "$TMPDIR/fmt.n3v3" > "$TMPDIR/fmt.n3v3.tmp" 2>/dev/null
mv "$TMPDIR/fmt.n3v3.tmp" "$TMPDIR/fmt.n3v3"
pass "fmt file smoke.n3v3"

# -- 6. fmt check (should pass after formatting) --
out=$($BIN fmt check "$TMPDIR/fmt.n3v3" 2>&1) || fail "fmt check after format: $out"
pass "fmt check (clean after fmt)"

# -- 7. lsp health --
echo ""
echo "--- lsp health ---"
out=$(timeout 3 $BIN lsp --check 2>&1) || true
echo "$out" | grep -q "ALL CHECKS PASSED" || fail "lsp health check failed"
pass "lsp --check"

# -- 8. V3.0 syntax (Phase 6) --
echo ""
echo "--- v3.0 syntax ---"
cat > "$TMPDIR/v3.n3v3" <<'EOF'
add(x: Int, y: Int) = x + y
use std.list
double = |n| n * 2
r = { name = "n3v3", version = "4.0" }
result = r & { version = "4.1" }
p = ./config.n3v3
EOF
out=$($BIN check "$TMPDIR/v3.n3v3" 2>&1) || fail "v3 syntax check: $out"
pass "v3.0 syntax (no-let, lambda, record, path literal, &merge, use)"

# -- 9. error case (should report diagnostics) --
echo ""
echo "--- error handling ---"
cat > "$TMPDIR/bad.n3v3" <<'EOF'
x = "hello" + 1
EOF
out=$($BIN check "$TMPDIR/bad.n3v3" 2>&1) && fail "bad.n3v3 should fail type-check"
echo "$out" | grep -qi "type\|mismatch" || fail "expected type error"
pass "diagnostics: type mismatch caught"

echo ""
echo "=== ALL CHECKS PASSED ==="
