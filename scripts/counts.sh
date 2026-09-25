#!/usr/bin/env bash
# Canonical project facts, derived from code.
# 规范的项目事实计数：全部从代码派生，是文档声明的唯一来源。
#
# Documentation must not restate these numbers by hand: claims in docs are checked
# against this script by `scripts/check-docs.sh` (see docs/contributor/contributing.md).
# 文档不得手工复述这些数字：`scripts/check-docs.sh` 会用本脚本的取值校验文档声明。
#
# Usage / 用法:
#   scripts/counts.sh          # key=value lines, sorted by key / 按 key 排序的 key=value 行
#   scripts/counts.sh <key>    # a single fact, e.g. `scripts/counts.sh lsp_methods` / 单个事实
#   scripts/counts.sh --json   # same facts as a JSON object / 同一批事实的 JSON 对象
#
# Derivation rules / 派生规则:
#   version           Cargo.toml [workspace.package] version — the product version / 产品版本
#   e2e_tests         `#[test]` count in tests/end_to_end.rs
#   parser_tests      `#[test]` count in tests/parser.rs
#   lexer_tests       `#[test]` count in tests/lexer.rs
#   lsp_methods       fn count inside `impl LanguageServer for Backend`
#                     (crates/n3v3-lsp/src/backend.rs)
#   lsp_notifications notification handlers implemented by that impl block
#   lsp_requests      lsp_methods - lsp_notifications
#   error_codes       unique Exxxx codes in crates/n3v3-diagnostic/src/codes.rs
#   keywords          canonical keyword tokens = reserved keywords minus the legacy
#                     declaration/path spellings (struct, enum, super, crate)
#   stream_apis       distinct `streamXxx` builtins registered by n3v3-std and
#                     cross-checked by n3v3-typeck
#   lean_modules      *.lean files under formal/
#   skills            skill entry files under .claude/skills
#   crates            directories under crates/
#   examples          *.n3v3 files under examples/
#
# Fact variables are read indirectly through `${!key}`, which shellcheck cannot
# trace statically.
# 事实变量通过 `${!key}` 间接读取，shellcheck 无法静态追踪。
# shellcheck disable=SC2034
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

die() { printf 'counts.sh: %s\n' "$*" >&2; exit 1; }
require() { [ -e "$1" ] || die "missing required source: $1"; }
count_matches() { # file pattern -> count (0 when absent)
  local n
  n=$(grep -c -E "$2" "$1" 2>/dev/null || true)
  printf '%s' "${n:-0}"
}

require Cargo.toml
require tests/end_to_end.rs
require tests/parser.rs
require tests/lexer.rs
require crates/n3v3-lsp/src/backend.rs
require crates/n3v3-diagnostic/src/codes.rs
require crates/n3v3-lexer/src/token.rs
require crates/n3v3-std/src
require crates/n3v3-typeck/src
require .claude/skills

version=$(awk '
  /^\[workspace.package\]/ { inside = 1; next }
  /^\[/ { inside = 0 }
  inside && /^version[[:space:]]*=/ { gsub(/[^0-9.]/, "", $0); print; exit }
' Cargo.toml)
[ -n "$version" ] || die "cannot derive version from [workspace.package] in Cargo.toml"

e2e_tests=$(count_matches tests/end_to_end.rs '#\[test\]')
parser_tests=$(count_matches tests/parser.rs '#\[test\]')
lexer_tests=$(count_matches tests/lexer.rs '#\[test\]')

lsp_block=$(awk '
  /^impl LanguageServer for Backend/ { inside = 1 }
  inside { print }
  inside && /^\}$/ { exit }
' crates/n3v3-lsp/src/backend.rs)
[ -n "$lsp_block" ] || die "cannot locate impl LanguageServer for Backend"
lsp_methods=$(printf '%s\n' "$lsp_block" | grep -c -E '^[[:space:]]+(async )?fn ' || true)
# Handler names are extracted once and matched without a pipeline: piping this
# ~1k-line block into `grep -q` lets the writer die of EPIPE, which `pipefail`
# turns into a failed pipeline (see .claude/hooks/verify-skills.sh).
# 处理器名只提取一次，匹配时不走管道：把这块 ~1k 行文本管道给 `grep -q` 会让写入方
# 因 EPIPE 失败，而 `pipefail` 会把整条管道判为失败（见 .claude/hooks/verify-skills.sh）。
lsp_declared=$(printf '%s\n' "$lsp_block" |
  sed -n -E 's/^[[:space:]]+(async[[:space:]]+)?fn[[:space:]]+([A-Za-z0-9_]+).*/\2/p')
lsp_notifications=0
for handler in initialized did_open did_change did_save did_close \
  did_change_configuration did_change_watched_files; do
  if grep -qxF "$handler" <<<"$lsp_declared"; then
    lsp_notifications=$((lsp_notifications + 1))
  fi
done
lsp_requests=$((lsp_methods - lsp_notifications))

error_codes=$(grep -o -E 'E[0-9]{4}' crates/n3v3-diagnostic/src/codes.rs | sort -u | wc -l)

legacy_keywords="struct enum super crate"
keywords=0
while IFS= read -r kw; do
  [ -n "$kw" ] || continue
  case " $legacy_keywords " in
    *" $kw "*) ;;
    *) keywords=$((keywords + 1)) ;;
  esac
done < <(awk '
  /pub fn keyword_from_str/ { inside = 1 }
  inside { print }
  inside && /^    \}$/ { exit }
' crates/n3v3-lexer/src/token.rs | grep -o -E '"[a-z]+"' | tr -d '"' | sort -u)

stream_apis=$(
  {
    grep -o -h -E '"(io\.)?stream[A-Z][A-Za-z]*"' -r crates/n3v3-std/src || true
    grep -o -h -E '"(io\.)?stream[A-Z][A-Za-z]*"' -r crates/n3v3-typeck/src || true
  } | sed -E 's/^"(io\.)?//; s/"$//' | sort -u | wc -l
)

lean_modules=$(find formal -name '*.lean' | wc -l)
skills=$(
  {
    find .claude/skills -maxdepth 1 -name '*.md' ! -name 'README.md'
    find .claude/skills -mindepth 2 -name 'SKILL.md'
  } | wc -l
)
crates=$(find crates -mindepth 1 -maxdepth 1 -type d | wc -l)
examples=$(find examples -name '*.n3v3' | wc -l)

keys=(version e2e_tests parser_tests lexer_tests lsp_methods lsp_notifications
  lsp_requests error_codes keywords stream_apis lean_modules skills crates examples)

known_key() { # key -> 0/1
  local candidate=$1 key
  for key in "${keys[@]}"; do
    [ "$key" = "$candidate" ] && return 0
  done
  return 1
}

case "${1:-}" in
  "")
    for key in $(printf '%s\n' "${keys[@]}" | sort); do
      printf '%s=%s\n' "$key" "${!key}"
    done
    ;;
  --json)
    printf '{\n'
    for i in "${!keys[@]}"; do
      key=${keys[$i]}
      value=${!key}
      comma=','
      [ "$i" -eq $((${#keys[@]} - 1)) ] && comma=''
      # `version` is a string; every other key is a derived count.
      # `version` 为字符串，其余键都是从代码派生的计数（输出为 JSON 数字）。
      if [ "$key" = version ]; then
        printf '  "%s": "%s"%s\n' "$key" "$value" "$comma"
      else
        printf '  "%s": %s%s\n' "$key" "$value" "$comma"
      fi
    done
    printf '}\n'
    ;;
  *)
    known_key "$1" || die "unknown key '$1' (see 'scripts/counts.sh' for the key list)"
    printf '%s=%s\n' "$1" "${!1}"
    ;;
esac
