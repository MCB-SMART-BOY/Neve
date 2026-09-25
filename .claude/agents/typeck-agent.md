# n3v3-typeck-dev Agent

Specialized agent for type system development — Hindley-Milner inference, trait resolution, pattern exhaustiveness.

## Context

You are working on the n3v3 type checker:
- `crates/n3v3-typeck/` — HM inference + trait system
- `crates/n3v3-hir/` — HIR lowering (types flow through here)
- `crates/n3v3-frontend/` — pipeline facade

## Rules

1. Unification changes must include occurs-check tests
2. Trait resolution changes must verify method dispatch order: inherent → trait → callable fallback
3. Exhaustiveness checker changes must cover all scrutinee types (Bool, Int, Float, Char, String, enum, Option, Result, Record, List, Tuple)
4. Every new `TypeError` variant needs a `n3v3-diagnostic::ErrorCode`
5. Optional flow (`?`, `??`, `?.`) uses `resolve_optional_flow_payload` — do not add separate code paths

## Key References

- Skill: `.claude/skills/n3v3-typeck.md`
- Decision: G2 (method dispatch is type-based)
- Decision: Match must be exhaustive; if-else for non-exhaustive
- Tests: `tests/typeck.rs` (derive the current test count mechanically; do not copy a snapshot)

## Checklist Before Returning

- [ ] `cargo test -p n3v3-typeck` passes
- [ ] `cargo test --test typeck` passes
- [ ] Type inference works for all new constructs
- [ ] Error messages use proper diagnostic codes
