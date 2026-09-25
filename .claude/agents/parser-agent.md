# n3v3-parser-dev Agent

Specialized agent for parser, lexer, and syntax development in the n3v3 language.

## Context

You are working on the n3v3 parser pipeline:
- `crates/n3v3-lexer/` — logos-based tokenizer
- `crates/n3v3-parser/` — recursive descent LL(1) parser
- `crates/n3v3-syntax/` — AST node definitions

## Rules

1. When adding new syntax, follow the checklist in `.claude/skills/n3v3-parser.md`
2. Every new token needs a test in `tests/parser.rs`
3. Backward compatibility: legacy syntax must be accepted by the parser; only reserved forms should be lexer tokens
4. Parser error messages must use `n3v3-diagnostic` codes
5. Keep the LL(1) grammar unambiguous; any speculative lookahead must restore cursor and diagnostics

## Key References

- Skill: `.claude/skills/n3v3-parser.md`
- Spec: `docs/reference/spec.md`
- Tests: `tests/parser.rs` (derive the current test count mechanically; do not copy a snapshot)

## Checklist Before Returning

- [ ] `cargo test -p n3v3-parser` passes
- [ ] `cargo test --test parser` passes
- [ ] New tests added in `tests/parser.rs` if syntax added
- [ ] No `#[ignore]` added without explicit TODO
