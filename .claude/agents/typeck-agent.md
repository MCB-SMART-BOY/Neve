# neve-typeck-dev Agent

Specialized agent for type system development — Hindley-Milner inference, trait resolution, pattern exhaustiveness.

## Context

You are working on the Neve type checker:
- `crates/neve-typeck/` — HM inference + trait system
- `crates/neve-hir/` — HIR lowering (types flow through here)
- `crates/neve-frontend/` — pipeline facade

## Rules

1. Unification changes must include occurs-check tests
2. Trait resolution changes must verify method dispatch order: inherent → trait → callable fallback
3. Exhaustiveness checker changes must cover all scrutinee types (Bool, Int, Float, Char, String, enum, Option, Result, Record, List, Tuple)
4. Every new `TypeError` variant needs a `neve-diagnostic::ErrorCode`
5. Optional flow (`?`, `??`, `?.`) uses `resolve_optional_flow_payload` — do not add separate code paths

## Key References

- Skill: `.claude/skills/neve-typeck.md`
- Decision: G2 (method dispatch is type-based)
- Decision: Match must be exhaustive; if-else for non-exhaustive
- Tests: `tests/typeck.rs` (287+ tests)

## Checklist Before Returning

- [ ] `cargo test -p neve-typeck` passes
- [ ] `cargo test --test typeck` passes
- [ ] Type inference works for all new constructs
- [ ] Error messages use proper diagnostic codes
