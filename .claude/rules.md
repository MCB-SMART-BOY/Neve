# .claude/ Rules

The `.claude/` directory is the **agent-native source of truth** for development.
Every rule here is enforced by hooks, not memory.

## Rule 1: Code Is Truth

When a skill describes an API, type, signature, or behavior, it must match the
**actual code**. If the code changes, the skill must be updated. Never trust a
skill over the code — if there's a conflict, the code wins and the skill is stale.

### Verification

After every code change that touches a public API, run:

```bash
.claude/hooks/verify-skills.sh
```

This checks that skill-documented types, files, and signatures actually exist.

## Rule 2: .claude/ Updates After Every Code Change

After completing any non-trivial code change (new feature, API change, deprecation,
crate reorg), update the relevant skill files:

| Code change in… | Update… |
|-----------------|---------|
| `crates/neve-parser/`, `crates/neve-lexer/`, `crates/neve-syntax/` | `skills/neve-parser.md` |
| `crates/neve-hir/` | `skills/neve-hir.md` |
| `crates/neve-typeck/` | `skills/neve-typeck.md` |
| `crates/neve-eval/` | `skills/neve-eval.md` |
| `crates/neve-std/` | `skills/neve-std.md` |
| `crates/neve-lsp/` | `skills/neve-lsp.md` |
| `formal/` | `skills/neve-lean.md` |
| `tests/` (test count, coverage) | `skills/neve-test.md` |
| `docs/reference/spec.md` | `skills/README.md` (status) |
| Any effect-related code | `skills/neve-effect.md` |
| Crate additions/removals | `skills/README.md` (crate map), `skills/neve-dev.md` |
| New decision gates resolved | `AGENTS.md`, `skills/README.md` |

## Rule 3: Skills Are Agent Instructions, Docs Are Human References

- **`.claude/skills/`** = agent instructions. SHORT. Architecture diagrams, crate
  APIs, decision records, gotchas. What the agent needs to work on this crate.
- **`docs/`** = human references. Language spec, tutorials, user guides, changelog.
  What humans read to learn or use Neve.

Skills **reference** docs with file paths — they don't duplicate them. If a skill
is repeating content from `docs/`, it should be a one-line link, not a copy.

## Rule 4: Skills Must Be Verifiable

Every claim in a skill file must be checkable against the codebase:

- "Key file: `typeck/src/infer.rs`" → the file must exist
- "20 LSP methods" → grep for handler functions must return ~20
- "EffectEval v4.3 (34 rules)" → the Lean file must declare 34 rules
- "541 E2E tests" → grep `#[test]` in `tests/end_to_end.rs` must return 537

Claims that can't be verified are speculation. Remove them.

## Rule 5: Driver Is Canonical

The `skills/run-neve/driver.sh` is the canonical smoke test for agent-driven
verification. If the driver doesn't pass, something is broken. Run it after any
change that touches the CLI or the language pipeline.

```bash
.claude/skills/run-neve/driver.sh
```
