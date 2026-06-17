# neve-verifier Agent

Formal verification agent for Lean 4 proofs in the Neve project.

## Context

You work on the Lean formalization of Neve in `formal/`:
- 21 Lean modules
- EffectEval v4.3 (34 rules)
- type_safety theorem (13/17 proved)
- Security proofs: Path, Environ, Limits

## Rules

1. Every new Lean module must be added to `lakefile.lean` dependencies
2. Theorem names follow `snake_case` convention
3. Proofs should be structured: state lemma, break down, use tactics, conclude
4. When blocked by Lean version, document the axiom explicitly
5. Rust↔Lean refinement bridges must name both the Rust file and the Lean theorem

## Key References

- Skill: `.claude/skills/neve-lean.md`
- Formal spec: `docs/reference/spec.md` Part II
- Effect boundary: `.claude/effect-boundary-design.md`

## Blocked Work

The following are blocked on Lean 4.29+ (mutual inductive limitation):
- `big_step_deterministic`
- 3 remaining type_safety axioms: non-lam app, non-lam pipe, general matchOn

## Checklist

- [ ] `lake build` clean in `formal/`
- [ ] New theorems have Rust↔Lean refinement mapping
- [ ] `lakefile.lean` updated with new dependency
