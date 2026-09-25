---
name: Pull request
about: Propose a change to n3v3
title: ""
labels: ""
assignees: ""
---

<!--
Canonical source: [`.claude/templates/pull_request.md`](../.claude/templates/pull_request.md).
This GitHub mirror must stay aligned with the canonical template; edit the source first.
-->

## Description
<!-- What does this PR do? Which crates are affected? -->

## Type
- [ ] feat: new feature
- [ ] fix: bug fix
- [ ] docs: documentation
- [ ] refactor: code restructuring
- [ ] style: formatting
- [ ] test: test addition/update
- [ ] release: version bump
- [ ] chore: CI/build/tooling

## Validation
<!-- Gate definitions: `scripts/validate.sh --list`. -->

### Local validation
- [ ] Before requesting review, run `scripts/validate.sh --quick` (format, lint, build, skills, and docs).
- [ ] For CI-equivalent local validation, run `scripts/validate.sh`; missing external tools may report `[SKIP]`.
- [ ] Use `scripts/validate.sh --strict` when skipped gates must fail.
- [ ] For release-facing changes, run `scripts/validate.sh --release`.

### CI validation
- [ ] The required CI job passes via `.github/workflows/ci.yml` → `scripts/validate.sh --ci` (CI enables strict mode).

## Change Hygiene
- [ ] New tests are added in the appropriate file when behavior changes.
- [ ] `docs/project/feature-matrix.md` is updated when capability status changes.
- [ ] `docs/project/changelog.md` is updated for user-facing changes.
- [ ] `CLAUDE.md` is updated when project-level guidance changes.
- [ ] All evaluation goes through the canonical HIR pipeline.

## Affected Crates
<!-- List the crates modified by this PR -->

## Related Issues
<!-- Link to related issues -->

## Screenshots / Terminal Output
<!-- If CLI or LSP behavior changed, include output -->
