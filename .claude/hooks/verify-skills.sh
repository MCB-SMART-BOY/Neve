#!/usr/bin/env bash
# Verify registered skill facts against source-derived counts. / 根据源代码派生计数校验已注册技能事实。
# Run after public API changes. / 公共 API 变更后运行。
set -euo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT"

pass_count=0
fail_count=0

pass_check() {
    printf '[PASS] %s\n' "$*"
    pass_count=$((pass_count + 1))
}

fail_check() {
    printf '[FAIL] %s\n' "$*" >&2
    fail_count=$((fail_count + 1))
}

check_path() {
    local path="$1"
    if [ -e "$path" ]; then
        pass_check "required path exists / 必需路径存在: $path"
    else
        fail_check "required path missing / 必需路径缺失: $path"
    fi
}

check_equal() {
    local label="$1"
    local actual="$2"
    local expected="$3"
    if [ "$actual" = "$expected" ]; then
        pass_check "$label: $actual"
    else
        local shown_actual="${actual:-<missing>}"
        local shown_expected="${expected:-<missing>}"
        fail_check "$label differs / $label 不一致: actual=$shown_actual expected=$shown_expected"
    fi
}

toml_value() {
    local section="$1"
    local key="$2"
    local file="$3"
    awk -v wanted_section="$section" -v wanted_key="$key" '
        /^\[/ {
            current_section = $0
            sub(/[[:space:]]*#.*/, "", current_section)
        }
        current_section == wanted_section &&
            $0 ~ "^[[:space:]]*" wanted_key "[[:space:]]*=" {
            value = $0
            sub(/^[^=]*=/, "", value)
            sub(/[[:space:]]*#.*/, "", value)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
            gsub(/^"/, "", value)
            gsub(/"$/, "", value)
            print value
            exit
        }
    ' "$file"
}

count_markers() {
    local file="$1"
    local pattern="$2"
    if [ ! -f "$file" ]; then
        printf '0'
        return
    fi
    grep -c -E "$pattern" "$file" 2>/dev/null || true
}

count_keywords() {
    local file="$1"
    local legacy_keywords='struct enum super crate'
    local keyword
    local count=0
    if [ ! -f "$file" ]; then
        printf '0'
        return
    fi

    while IFS= read -r keyword; do
        [ -n "$keyword" ] || continue
        case " $legacy_keywords " in
            *" $keyword "*) ;;
            *) count=$((count + 1)) ;;
        esac
    done < <(
        awk '
            /pub fn keyword_from_str/ { inside = 1 }
            inside { print }
            inside && /^    \}$/ { exit }
        ' "$file" | grep -o -E '"[a-z]+"' | tr -d '"' | sort -u || true
    )
    printf '%s' "$count"
}

count_stream_apis() {
    local count
    count=$(
        {
            grep -o -h -E '"(io\.)?stream[A-Z][A-Za-z]*"' -r crates/n3v3-std/src 2>/dev/null || true
            grep -o -h -E '"(io\.)?stream[A-Z][A-Za-z]*"' -r crates/n3v3-typeck/src 2>/dev/null || true
        } | sed -E 's/^"(io\.)?//; s/"$//' | sort -u | wc -l
    )
    printf '%s' "$count" | tr -d '[:space:]'
}

count_lsp_facts() {
    local file="$1"
    local block
    local methods
    local notifications=0
    local handler

    if [ ! -f "$file" ]; then
        printf '0 0 0\n'
        return
    fi

    block=$(awk '
        /^impl LanguageServer for Backend/ { inside = 1 }
        inside { print }
        inside && /^\}$/ { exit }
    ' "$file")
    if [ -z "$block" ]; then
        printf '0 0 0\n'
        return
    fi

    methods=$(printf '%s\n' "$block" | grep -c -E '^[[:space:]]+(async )?fn ' || true)
    for handler in initialized did_open did_change did_save did_close \
        did_change_configuration did_change_watched_files; do
        if printf '%s\n' "$block" | grep -q -E \
            "^[[:space:]]+(async )?fn ${handler}[[:space:]]*\("; then
            notifications=$((notifications + 1))
        fi
    done
    printf '%s %s %s\n' "$methods" "$notifications" "$((methods - notifications))"
}

printf '=== Skill verification / 技能校验 ===\n'

# Check directories and explicitly registered files. / 校验目录及明确注册的文件。
for directory in .claude/skills .claude/hooks .claude/agents .claude/workflows; do
    check_path "$directory"
done

registered_paths=(
    .claude/skills/n3v3-dev.md
    .claude/skills/n3v3-parser.md
    .claude/skills/n3v3-typeck.md
    .claude/skills/n3v3-hir.md
    .claude/skills/n3v3-eval.md
    .claude/skills/n3v3-std.md
    .claude/skills/n3v3-lsp.md
    .claude/skills/n3v3-test.md
    .claude/skills/n3v3-diagnostic.md
    .claude/skills/n3v3-fmt.md
    .claude/skills/n3v3-effect.md
    .claude/skills/n3v3-lean.md
    .claude/skills/run-n3v3/SKILL.md
    .claude/hooks/verify-skills.sh
    .claude/hooks/fmt-all.sh
    .claude/hooks/pre-commit.sh
    .claude/hooks/check-regressions.sh
    .claude/agents/parser-agent.md
    .claude/agents/typeck-agent.md
    .claude/agents/reviewer-agent.md
    .claude/agents/verifier-agent.md
    .claude/workflows/full-test.js
    .claude/workflows/pre-release.js
)
for path in "${registered_paths[@]}"; do
    check_path "$path"
done

# Check package, binary, and version metadata. / 校验包名、二进制名和版本元数据。
workspace_version=''
cli_package_name=''
cli_binary_name=''
docs_version=''
if [ -f Cargo.toml ]; then
    workspace_version=$(toml_value '[workspace.package]' version Cargo.toml)
fi
if [ -f n3v3-cli/Cargo.toml ]; then
    cli_package_name=$(toml_value '[package]' name n3v3-cli/Cargo.toml)
    cli_binary_name=$(toml_value '[[bin]]' name n3v3-cli/Cargo.toml)
fi
if [ -f docs/README.md ]; then
    docs_version=$(sed -nE \
        's/.*\*\*Current version: v([0-9]+\.[0-9]+\.[0-9]+)\*\*.*/\1/p' \
        docs/README.md | sed -n '1p')
fi
check_equal "CLI package name / CLI 包名" "$cli_package_name" 'n3v3'
check_equal "CLI binary name / CLI 二进制名" "$cli_binary_name" 'n3v3'
check_equal "workspace version / 工作区版本" "$workspace_version" "$docs_version"

# Counts are the only numeric fact source. / 所有数字事实只来自规范计数脚本。
counts_script='scripts/counts.sh'
counts_ready=0
declare -A canonical_counts=()
required_keys=(
    version e2e_tests parser_tests lexer_tests lsp_methods lsp_notifications
    lsp_requests error_codes keywords stream_apis lean_modules skills crates examples
)

if [ ! -f "$counts_script" ]; then
    fail_check "run: scripts/counts.sh missing (required canonical fact source / 缺少必需规范事实源)"
else
    counts_output=''
    if counts_output=$(bash "$counts_script" 2>&1); then
        counts_ready=1
    else
        counts_status=$?
        fail_check "run: scripts/counts.sh failed (exit $counts_status): $counts_output"
    fi
fi

if [ "$counts_ready" -eq 1 ]; then
    while IFS='=' read -r key value; do
        [ -n "$key" ] || continue
        known_key=0
        for required_key in "${required_keys[@]}"; do
            if [ "$key" = "$required_key" ]; then
                known_key=1
                break
            fi
        done
        if [ "$known_key" -eq 0 ]; then
            fail_check "counts.sh emitted unexpected key / 输出了未知键: $key"
            continue
        fi
        if [ -z "$value" ]; then
            fail_check "counts.sh emitted empty value / 输出了空值: $key"
        elif [ "$key" != version ] && [[ ! "$value" =~ ^[0-9]+$ ]]; then
            fail_check "counts.sh emitted non-numeric value / 输出了非数字值: $key=$value"
        else
            canonical_counts["$key"]="$value"
        fi
    done <<< "$counts_output"

    for required_key in "${required_keys[@]}"; do
        if [ -z "${canonical_counts[$required_key]+present}" ]; then
            fail_check "counts.sh missing key / 缺少计数键: $required_key"
        fi
    done
fi

if [ "$counts_ready" -eq 1 ]; then
    # Recompute each canonical metric and compare exactly. / 重新计算每项规范指标并精确比较。
    declare -A actual_counts=()
    actual_counts[version]="$workspace_version"
    actual_counts[e2e_tests]=$(count_markers tests/end_to_end.rs '#\[test\]')
    actual_counts[parser_tests]=$(count_markers tests/parser.rs '#\[test\]')
    actual_counts[lexer_tests]=$(count_markers tests/lexer.rs '#\[test\]')

    read -r actual_lsp_methods actual_lsp_notifications actual_lsp_requests < <(
        count_lsp_facts crates/n3v3-lsp/src/backend.rs
    )
    actual_counts[lsp_methods]="$actual_lsp_methods"
    actual_counts[lsp_notifications]="$actual_lsp_notifications"
    actual_counts[lsp_requests]="$actual_lsp_requests"

    if [ -f crates/n3v3-diagnostic/src/codes.rs ]; then
        actual_counts[error_codes]=$(
            (grep -o -E 'E[0-9]{4}' crates/n3v3-diagnostic/src/codes.rs || true) |
                sort -u | wc -l | tr -d '[:space:]'
        )
    else
        actual_counts[error_codes]=0
    fi
    actual_counts[keywords]=$(count_keywords crates/n3v3-lexer/src/token.rs)
    actual_counts[stream_apis]=$(count_stream_apis)
    actual_counts[lean_modules]=$(find formal -name '*.lean' 2>/dev/null | wc -l | tr -d '[:space:]')
    actual_counts[skills]=$(
        {
            find .claude/skills -maxdepth 1 -name '*.md' ! -name 'README.md' 2>/dev/null || true
            find .claude/skills -mindepth 2 -name 'SKILL.md' 2>/dev/null || true
        } | wc -l | tr -d '[:space:]'
    )
    actual_counts[crates]=$(find crates -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l | tr -d '[:space:]')
    actual_counts[examples]=$(find examples -name '*.n3v3' 2>/dev/null | wc -l | tr -d '[:space:]')

    for required_key in "${required_keys[@]}"; do
        check_equal "canonical count / 规范计数 $required_key" \
            "${actual_counts[$required_key]}" "${canonical_counts[$required_key]-}"
    done
    check_equal "counts version / 计数版本" \
        "${canonical_counts[version]-}" "$workspace_version"
fi

if [ "$fail_count" -gt 0 ]; then
    printf '[FAIL] Skill verification failed / 技能校验失败: %d failed, %d passed\n' \
        "$fail_count" "$pass_count" >&2
    exit 1
fi

printf '[PASS] Skill verification complete / 技能校验完成: %d checks\n' "$pass_count"
