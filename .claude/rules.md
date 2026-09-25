# .claude/ Rules

The `.claude/` directory is the **agent-native source of truth** for development.
Rules here are enforced by scripts and hooks, not memory: `scripts/validate.sh`
(the gate entry point), `scripts/check-docs.sh` (documentation standard),
`scripts/counts.sh` (canonical facts), and `.claude/hooks/*.sh`.

## Rule 1: Code Is Truth

When a skill describes an API, type, signature, or behavior, it must match the
**actual code**. If the code changes, the skill must be updated. Never trust a
skill over the code — if there's a conflict, the code wins and the skill is stale.

### Verification

After every code change that touches a public API, run:

```bash
.claude/hooks/verify-skills.sh   # declared paths, package/binary names, version, counts
```

It checks that every declared skill/hook/agent/workflow path exists and that the
version and count claims match `scripts/counts.sh`. Type and signature claims are
not machine-checked; keep them in code or tests instead of prose.

## Rule 2: .claude/ Updates After Every Code Change

After completing any non-trivial code change (new feature, API change, deprecation,
crate reorg), update the relevant skill files:

| Code change in… | Update… |
|-----------------|---------|
| `crates/n3v3-parser/`, `crates/n3v3-lexer/`, `crates/n3v3-syntax/` | `skills/n3v3-parser.md` |
| `crates/n3v3-hir/` | `skills/n3v3-hir.md` |
| `crates/n3v3-typeck/` | `skills/n3v3-typeck.md` |
| `crates/n3v3-eval/` | `skills/n3v3-eval.md` |
| `crates/n3v3-std/` | `skills/n3v3-std.md` |
| `crates/n3v3-lsp/` | `skills/n3v3-lsp.md` |
| `formal/` | `skills/n3v3-lean.md` |
| `tests/` (test count, coverage) | `skills/n3v3-test.md` |
| `docs/reference/spec.md` | `skills/README.md` (status) |
| Any effect-related code | `skills/n3v3-effect.md` |
| Crate additions/removals | `skills/README.md` (crate map), `skills/n3v3-dev.md` |
| New decision gates resolved | `CLAUDE.md`, `skills/README.md` |

## Rule 3: Skills Are Agent Instructions, Docs Are Human References

- **`.claude/skills/`** = agent instructions. SHORT. Architecture diagrams, crate
  APIs, decision records, gotchas. What the agent needs to work on this crate.
- **`docs/`** = human references. Language spec, tutorials, user guides, changelog.
  What humans read to learn or use n3v3.

Skills **reference** docs with file paths — they don't duplicate them. If a skill
is repeating content from `docs/`, it should be a one-line link, not a copy.

## Rule 4: Skills Must Be Verifiable

Every claim in a skill file must be checkable against the codebase. Numbers are
never hand-written: they come from `scripts/counts.sh`, which derives them from
code, and `.claude/hooks/verify-skills.sh` compares the skill claims against it.

- "Key file: `crates/n3v3-typeck/src/check/mod.rs`" → the file must exist
- "26 LSP methods" → `scripts/counts.sh lsp_methods`
- "556 E2E tests" → `scripts/counts.sh e2e_tests`
- "12 canonical keywords", "13 Stream<T> APIs", "21 Lean modules", "55 diagnostic
  codes" → the matching `counts.sh` key

Claims that can't be verified are speculation. Remove them.

## Rule 5: Driver Is Canonical

The `skills/run-n3v3/driver.sh` is the canonical smoke test for agent-driven
verification. If the driver doesn't pass, something is broken. Run it after any
change that touches the CLI or the language pipeline.

```bash
.claude/skills/run-n3v3/driver.sh
```

## Rule 6: One Quality-Gate Entry Point

`scripts/validate.sh` is the only place that defines the gate set. Hooks, agent
workflows, CI, and release checks call it — they never re-list commands:

|Consumer|Invocation|
|---|---|
|`.claude/hooks/pre-commit.sh` (pre-commit)|`scripts/validate.sh --quick`|
|`.claude/workflows/full-test.js`|`scripts/validate.sh`|
|`.claude/workflows/pre-release.js`|`scripts/validate.sh --release`|
|`.github/workflows/ci.yml`|`scripts/validate.sh --ci`|
|`.github/workflows/release.yml` (release gates)|`scripts/validate.sh --release --strict`|

Gate additions and command changes happen in `scripts/validate.sh` only.
`--list` prints the gate set; `--strict` turns a missing external tool
(`gitleaks`, `trivy`, `cargo-audit`, `cargo-deny`) from `[SKIP]` into a failure.
Pinned tool versions live in `scripts/install-gate-tools.sh`, which both
workflows call (a tag push does not trigger `ci.yml`, so release gates are the
only proof for a release commit).

## Rule 7: Documentation Standard

Markdown standards live in `docs/contributor/contributing.md`
(§ Documentation Standards) and are enforced by `scripts/check-docs.sh`:
machine-checkable counts and version claims, hub index coverage, page header
blocks, resolvable relative links, `n3v3-check` snippets and `examples/**/*.n3v3`
that pass `n3v3 check`, `n3v3 doc` registry consistency, and changelog structure.
The standard's bilingual, status-wording and change-flow rules are review
conventions, not script checks.

Facts are owned by code: the product version by `Cargo.toml`, every count by
`scripts/counts.sh`, API signatures by `crates/**`, behavior by `tests/**`.
Documents state those facts; they never become a second source of truth.
