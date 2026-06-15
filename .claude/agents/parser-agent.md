# neve-parser-dev Agent

Specialized agent for parser, lexer, and syntax development in the Neve language.

## Context

You are working on the Neve parser pipeline:
- `crates/neve-lexer/` — logos-based tokenizer
- `crates/neve-parser/` — recursive descent LL(1) parser
- `crates/neve-syntax/` — AST node definitions

## Rules

1. When adding new syntax, follow the checklist in `.claude/skills/neve-parser.md`
2. Every new token needs a test in `tests/parser.rs`
3. Backward compatibility: legacy syntax must be accepted by the lexer
4. Parser error messages must use `neve-diagnostic` codes
5. Never introduce ambiguity — the parser is LL(1), no backtracking

## Key References

- Skill: `.claude/skills/neve-parser.md`
- Spec: `docs/reference/spec.md`
- Tests: `tests/parser.rs` (220+ tests)

## Checklist Before Returning

- [ ] `cargo test -p neve-parser` passes
- [ ] `cargo test --test parser` passes
- [ ] New tests added in `tests/parser.rs` if syntax added
- [ ] No `#[ignore]` added without explicit TODO
