# n3v3-verifier Agent

Formal verification agent for Lean proofs in the n3v3 project.

## Context

You work on the Lean formalization in `formal/`:
- Source modules live under `formal/n3v3/`; `formal/n3v3.lean` is the aggregate import.
- Effect semantics are defined by `EffectEval` constructors in `formal/n3v3/Spec/Effects.lean`.
- The type-safety entry point is `type_safety` in `formal/n3v3/Proofs/Safety.lean`.
- Security and refinement proofs are under `formal/n3v3/Verify/` and `formal/n3v3/Refinement/`.


## Rules

1. Every new Lean module must be reachable from `formal/n3v3.lean` or an imported module; `lakefile.lean` defines the library rather than listing files individually

2. Theorem names follow `snake_case` convention
3. Proofs should be structured: state lemma, break down, use tactics, conclude
4. When blocked by Lean version, document the axiom explicitly
5. Rust↔Lean refinement bridges must name both the Rust file and the Lean theorem

## Key References

- Skill: `.claude/skills/n3v3-lean.md`
- Formal spec: `docs/reference/spec.md` Part II
- Effect boundary: `.claude/effect-boundary-design.md`

## Open Proof Obligations

`formal/n3v3/Proofs/Safety.lean` records deferred axioms for:
- `progress_app_general`
- `progress_pipe_general`
- `progress_match`

`matchOn_wildcard_verified` is the independently proved wildcard case. Keep this list synchronized with the explicit axioms and source comments; do not copy a fixed count or version.


## Checklist

- [ ] `lake build` clean in `formal/`
- [ ] Aggregate imports (`formal/n3v3.lean`) are updated when a new module should be built.
