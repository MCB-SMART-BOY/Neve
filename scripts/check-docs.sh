#!/usr/bin/env bash
# Documentation standard checker.
# 文档标准校验器 —— 执行 docs/contributor/contributing.md 中的文档标准。
#
# Usage / 用法:
#   scripts/check-docs.sh [--release] [--strict]
#
# Checks / 检查项:
#   counts     文档中的计数声明必须等于 scripts/counts.sh 的派生值
#   version    当前版本声明必须等于 Cargo.toml [workspace.package] version
#   hub        docs/**/*.md（除 docs/README.md）必须在 docs/README.md 有索引项
#   header     docs 页面必须带统一页头块
#   links      相对 Markdown 链接必须可解析
#   registry   n3v3-cli 的 include_str!/TOPICS 与 `n3v3 doc` help 主题一致
#   changelog  docs/project/changelog.md 必须含当前版本条目（--release 额外要求）
#   examples   examples/**/*.n3v3 必须通过 `n3v3 check --allow-effects`
#   snippets   ```n3v3-check 代码块必须通过 `n3v3 check --allow-effects`
#
# Exit status / 退出码: 0 = 全部通过（含 SKIP）；1 = 存在 FAIL。
#
# Fact variables (version, e2e_tests, ...) come from scripts/counts.sh via `eval`;
# static analysis cannot trace them.
# 事实变量（version、e2e_tests 等）通过 `eval` 从 scripts/counts.sh 读取，静态分析无法追踪。
# shellcheck disable=SC2034,SC2154
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

release=0
strict=0
for arg in "$@"; do
  case "$arg" in
    --release) release=1 ;;
    --strict) strict=1 ;;
    *) printf 'check-docs.sh: unknown argument: %s\n' "$arg" >&2; exit 2 ;;
  esac
done

fails=0
skips=0
DETAILS=()
flush_details() {
  local item
  for item in ${DETAILS[@]+"${DETAILS[@]}"}; do
    printf '       - %s\n' "$item"
  done
  DETAILS=()
}
pass() { printf '[PASS] %s\n' "$1"; flush_details; }
fail() { printf '[FAIL] %s\n' "$1"; fails=$((fails + 1)); flush_details; }
skip() { printf '[SKIP] %s\n' "$1"; skips=$((skips + 1)); flush_details; }
detail() { DETAILS+=("$1"); }

# ---- fact sources -----------------------------------------------------------
# counts.sh is the single source for numbers; it emits key=value lines.
# counts.sh 是数字的唯一来源，输出 key=value 行。
eval "$(scripts/counts.sh)"

# Files whose numbers must be current. `docs/project/changelog.md` and
# `.claude/audit-report.md` are historical snapshots and keep their own numbers.
# 计数需要反映当前状态的文档；变更日志与审计快照属历史记录，保留当时数字。
HISTORICAL_FILES="docs/project/changelog.md .claude/audit-report.md"

# Files that must state the current product version. 必须声明当前产品版本的文件。
CURRENT_VERSION_FILES=(
  README.md
  CLAUDE.md
  docs/README.md
  docs/reference/stability.md
  .claude/skills/README.md
)

mapfile -t DOC_FILES < <(
  {
    find docs -name '*.md'
    printf '%s\n' README.md CLAUDE.md
    find .claude/skills .claude/agents -name '*.md'
    printf '%s\n' .claude/rules.md .claude/forward-plan.md .claude/settings.json
  } | sort -u
)

is_historical() {
  case " $HISTORICAL_FILES " in
    *" $1 "*) return 0 ;;
    *) return 1 ;;
  esac
}

# ---- counts -----------------------------------------------------------------
# Each metric: canonical forms in EN/ZH. Every match must equal the derived value.
# 每个指标：中英两种规范写法；每个匹配都必须等于派生值。
check_metric() {
  local label=$1 expected=$2
  shift 2
  local pattern match number bad=0
  for file in "${DOC_FILES[@]}"; do
    is_historical "$file" && continue
    for pattern in "$@"; do
      while IFS= read -r match; do
        [ -n "$match" ] || continue
        # `grep -n` prefixes the line number; the claim number is in the text.
        # grep -n 会在前面加行号，声明中的数字取自匹配文本。
        number=$(printf '%s' "${match#*:}" | grep -o -E '[0-9]+' | head -1)
        if [ "$number" != "$expected" ]; then
          bad=$((bad + 1))
          detail "$file:${match%%:*} claims '${match#*:}' (expected ${expected})"
        fi
      done < <(grep -n -o -E "$pattern" "$file" 2>/dev/null || true)
    done
  done
  if [ "$bad" -eq 0 ]; then
    pass "counts: ${label} = ${expected}"
  else
    fail "counts: ${label} (expected ${expected})"
  fi
  flush_details
}

check_metric "E2E tests" "$e2e_tests" \
  '[0-9]+ E2E tests' '[0-9]+ 个 E2E 测试' '[0-9]+ E2E 测试'
check_metric "parser tests" "$parser_tests" \
  '[0-9]+ parser tests' '[0-9]+ 个 parser 测试'
check_metric "diagnostic codes" "$error_codes" \
  '[0-9]+ (diagnostic|error) codes' '[0-9]+ 个(诊断码|错误码)' '[0-9]+ (诊断码|错误码)'
check_metric "LSP methods" "$lsp_methods" \
  '[0-9]+ LSP methods' '[0-9]+ 个 LSP 方法' '[0-9]+ LSP 方法'
check_metric "canonical keywords" "$keywords" \
  '[0-9]+ (canonical )?keywords' '[0-9]+ 个(规范)?关键词' '[0-9]+ (规范)?关键词'
check_metric "Stream<T> APIs" "$stream_apis" \
  '[0-9]+ Stream<T> APIs' '[0-9]+ 个 Stream<T> API'
check_metric "Lean modules" "$lean_modules" \
  '[0-9]+ Lean modules' '[0-9]+ 个 Lean 模块'

# ---- version ----------------------------------------------------------------
check_version() {
  local bad=0 match number
  for file in "${CURRENT_VERSION_FILES[@]}"; do
    if ! grep -q -F "v${version}" "$file"; then
      bad=$((bad + 1))
      detail "$file does not mention the current version v${version}"
    fi
  done

  while IFS= read -r match; do
    [ -n "$match" ] || continue
    number=${match#*:}
    number=$(printf '%s' "$number" | grep -o -E 'v[0-9]+\.[0-9]+\.[0-9]+')
    if [ "$number" != "v${version}" ]; then
      bad=$((bad + 1))
      detail "${match%%:*} states '${match#*:}' (expected v${version})"
    fi
  done < <(
    grep -n -o -E 'Current version: v[0-9]+\.[0-9]+\.[0-9]+|n3v3（v[0-9]+\.[0-9]+\.[0-9]+' \
      "${DOC_FILES[@]}" 2>/dev/null || true
  )

  if [ "$bad" -eq 0 ]; then
    pass "version: all current-version claims are v${version}"
  else
    fail "version: current version is v${version}"
  fi
}

check_version

# ---- hub coverage -----------------------------------------------------------
check_hub() {
  local bad=0 page rel
  while IFS= read -r page; do
    rel=${page#docs/}
    if ! grep -q -F "$rel" docs/README.md; then
      bad=$((bad + 1))
      detail "$page is not indexed in docs/README.md"
    fi
  done < <(find docs -name '*.md' ! -path 'docs/README.md' | sort)
  if [ "$bad" -eq 0 ]; then
    pass "hub: every docs page is indexed in docs/README.md"
  else
    fail "hub: docs/README.md index coverage"
  fi
}

check_hub

# ---- page header ------------------------------------------------------------
# Every docs page except the hub itself carries the standard header block
# (centered title block). 除 hub 外的 docs 页面必须带统一页头块。
HEADER_LINES=40
check_header() {
  local bad=0 page
  while IFS= read -r page; do
    if ! awk -v limit="$HEADER_LINES" '
      /align="center"/ { found = 1 }
      NR >= limit { exit }
      END { exit(found ? 0 : 1) }
    ' "$page"; then
      bad=$((bad + 1))
      detail "$page: no standard header block in the first ${HEADER_LINES} lines"
    fi
  done < <(find docs -name '*.md' ! -path 'docs/README.md' | sort)
  if [ "$bad" -eq 0 ]; then
    pass "header: every docs page has the standard header block"
  else
    fail "header: docs page header blocks"
  fi
}

check_header

# ---- links ------------------------------------------------------------------
check_links() {
  local bad=0 file dir target match
  for file in "${DOC_FILES[@]}"; do
    dir=$(dirname -- "$file")
    while IFS= read -r match; do
      [ -n "$match" ] || continue
      target=${match#*:]\(}
      target=${target%\)}
      case "$target" in
        http*|mailto:*|'#'*|'/'*|'<'*|'') continue ;;
      esac
      target=${target%%#*}
      target=${target#<}
      target=${target%>}
      [ -n "$target" ] || continue
      if [ ! -e "${dir}/${target}" ]; then
        bad=$((bad + 1))
        detail "$file:${match%%:*} -> $target (missing)"
      fi
    done < <(grep -n -o -E '\]\([^)]+\)' "$file" 2>/dev/null || true)
  done
  if [ "$bad" -eq 0 ]; then
    pass "links: all relative Markdown links resolve"
  else
    fail "links: unresolved relative links"
  fi
}

check_links

# ---- doc registry (n3v3 doc) ------------------------------------------------
check_registry() {
  local doc_rs=n3v3-cli/src/commands/doc.rs
  local main_rs=n3v3-cli/src/main.rs
  local bad=0 path key topics_help
  [ -f "$doc_rs" ] || { fail "registry: $doc_rs not found"; return; }

  while IFS= read -r path; do
    [ -n "$path" ] || continue
    if [ ! -e "n3v3-cli/src/commands/$path" ]; then
      bad=$((bad + 1))
      detail "$doc_rs: include_str!(\"$path\") does not resolve"
    fi
  done < <(grep -o -E 'include_str!\("[^"]+"\)' "$doc_rs" | sed -E 's/include_str!\("(.*)"\)/\1/' || true)

  topics_help=$(sed -n '/Topic to view\|要查看的主题/p' "$main_rs" | tr ',' '\n' |
    grep -o -E '[a-z][a-z-]+' | sort -u || true)
  while IFS= read -r key; do
    [ -n "$key" ] || continue
    if ! printf '%s\n' "$topics_help" | grep -q -x -F "$key"; then
      bad=$((bad + 1))
      detail "$main_rs: 'n3v3 doc' help does not list topic '$key'"
    fi
  done < <(awk '
    /^const TOPICS/ { inside = 1 }
    inside { print }
    inside && /^\];/ { exit }
  ' "$doc_rs" | grep -o -E '"[a-z0-9-]+",' | tr -d '",' | sort -u)

  if [ "$bad" -eq 0 ]; then
    pass "registry: n3v3 doc embeds and help topics are consistent"
  else
    fail "registry: n3v3 doc registry consistency"
  fi
}

check_registry

# ---- changelog --------------------------------------------------------------
check_changelog() {
  local changelog=docs/project/changelog.md
  local bad=0 newest
  [ -f "$changelog" ] || { fail "changelog: $changelog not found"; return; }

  if ! grep -q -E "^## \[${version}\]" "$changelog"; then
    bad=$((bad + 1))
    detail "$changelog has no '## [${version}]' entry"
  fi

  if [ "$release" -eq 1 ]; then
    newest=$(grep -o -E '^## \[[0-9]+\.[0-9]+\.[0-9]+\]' "$changelog" | head -1 |
      grep -o -E '[0-9]+\.[0-9]+\.[0-9]+' || true)
    if [ "$newest" != "$version" ]; then
      bad=$((bad + 1))
      detail "$changelog newest release entry is '${newest:-none}' (expected ${version})"
    fi
    if ! grep -q -E '^## \[Unreleased\]' "$changelog"; then
      bad=$((bad + 1))
      detail "$changelog has no '## [Unreleased]' section"
    fi
  fi

  if [ "$bad" -eq 0 ]; then
    pass "changelog: entry for v${version}${release:+ (release mode)}"
  else
    fail "changelog: structure for v${version}"
  fi
}

check_changelog

# ---- executable examples ----------------------------------------------------
binary=${N3V3_BIN:-${CARGO_TARGET_DIR:-./target}/debug/n3v3}
run_n3v3_check() { # file -> 0/1
  "$binary" check --allow-effects --quiet "$1" >/dev/null 2>&1
}

check_examples() {
  if [ ! -x "$binary" ]; then
    if [ "$strict" -eq 1 ]; then
      fail "examples: $binary missing (run 'cargo build -p n3v3')"
    else
      skip "examples: $binary missing (run 'cargo build -p n3v3')"
    fi
    return
  fi
  local bad=0 total=0 fences=0 file output fence_count
  while IFS= read -r file; do
    total=$((total + 1))
    if ! run_n3v3_check "$file"; then
      bad=$((bad + 1))
      detail "$file failed 'n3v3 check'"
    fi
  done < <(find examples -name '*.n3v3' | sort)
  if [ "$total" -eq 0 ]; then
    fail "examples: no *.n3v3 file matched (the standard requires examples to pass 'n3v3 check')"
    return
  fi
  if [ "$bad" -eq 0 ]; then
    pass "examples: ${total}/${total} examples pass 'n3v3 check'"
  else
    fail "examples: ${bad}/${total} failed 'n3v3 check'"
  fi
}

check_snippets() {
  if [ ! -x "$binary" ]; then
    if [ "$strict" -eq 1 ]; then
      fail "snippets: $binary missing (run 'cargo build -p n3v3')"
    else
      skip "snippets: $binary missing (run 'cargo build -p n3v3')"
    fi
    return
  fi
  local bad=0 total=0 fences=0 file tmp block line
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' RETURN
  for file in "${DOC_FILES[@]}"; do
    case "$file" in *.md) ;; *) continue ;; esac
    fences=$((fences + $(grep -c '^```n3v3-check' "$file" || true)))
    grep -q '^```n3v3-check' "$file" || continue
    rm -f "$tmp"/block_*.n3v3
    # Extract each ```n3v3-check block to its own file. Names encode the source
    # page (path-safe) and the block's first line, so failures point at a
    # location instead of a temporary path.
    # 把每个 ```n3v3-check 块抽成独立文件；文件名编码来源页面（路径安全化）与
    # 块首行号，失败信息因此指向真实位置而不是临时路径。
    tag=${file//[^A-Za-z0-9_-]/_}
    awk -v out="$tmp/block" -v tag="$tag" '
      /^[[:space:]]*```n3v3-check[[:space:]]*$/ { inside = 1; line = FNR + 1; n++; next }
      /^[[:space:]]*```[[:space:]]*$/ { inside = 0 }
      inside { print > (out "_" tag "_L" line "_" n ".n3v3") }
    ' "$file"
    for block in "$tmp"/block_*.n3v3; do
      [ -e "$block" ] || continue
      total=$((total + 1))
      if ! run_n3v3_check "$block"; then
        bad=$((bad + 1))
        line=$(printf '%s\n' "$block" | sed -E 's/.*_L([0-9]+)_[0-9]+\.n3v3$/\1/')
        detail "$file:${line} n3v3-check block failed 'n3v3 check'"
      fi
    done
  done
  if [ "$fences" -eq 0 ]; then
    fail "snippets: no 'n3v3-check' fence found in the documentation (the standard requires executable snippets)"
    return
  fi
  if [ "$fences" -gt 0 ] && [ "$total" -eq 0 ]; then
    fail "snippets: ${fences} n3v3-check fence(s) found but none extracted (extraction is broken)"
    return
  fi
  if [ "$bad" -eq 0 ]; then
    pass "snippets: ${total} n3v3-check block(s) pass 'n3v3 check'"
  else
    fail "snippets: ${bad}/${total} n3v3-check block(s) failed"
  fi
}

check_examples
check_snippets

# ---- summary ----------------------------------------------------------------
printf 'summary: %d failed, %d skipped\n' "$fails" "$skips"
if [ "$fails" -gt 0 ]; then
  exit 1
fi
if [ "$strict" -eq 1 ] && [ "$skips" -gt 0 ]; then
  printf 'check-docs.sh: strict mode treats skipped checks as failures\n' >&2
  exit 1
fi
exit 0
