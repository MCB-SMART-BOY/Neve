<div align="center">

<img src="../assets/logo.svg" width="120" alt="Neve logo">

<h1>Neve Documentation Hub</h1>

<p>
  <strong><a href="../README.md">Home</a></strong> ·
  <strong><a href="./">Docs</a></strong> ·
  <strong><a href="user/quickstart.md">Quickstart</a></strong> ·
  <strong><a href="reference/spec.md">Spec</a></strong> ·
  <strong><a href="reference/api.md">API</a></strong>
</p>

</div>

**Current version: v4.0.4**. 12 canonical keywords, 55 diagnostic codes, 541 E2E tests (all pass), 21 LSP methods. Install via `cargo install n3v3`.

Welcome to the Neve documentation hub. For **developer reference** (per-crate APIs,
architecture, integration points), see `.claude/skills/` — one skill per crate,
kept in sync with the actual code.

---

## Start Here

- New user: [user/install.md](user/install.md), [user/quickstart.md](user/quickstart.md), [user/tutorial.md](user/tutorial.md)
- Language reference: [reference/spec.md](reference/spec.md), [reference/api.md](reference/api.md), [reference/diagnostics.md](reference/diagnostics.md)
- Contributor: [contributor/contributing.md](contributor/contributing.md), [contributor/onboarding.md](contributor/onboarding.md), [contributor/architecture.md](contributor/architecture.md)
- Project status: [project/feature-matrix.md](project/feature-matrix.md), [project/changelog.md](project/changelog.md)
- Stability: [reference/stability.md](reference/stability.md)
- **Developer skills**: [`.claude/skills/`](../.claude/skills/) — per-crate architecture, APIs, key files

---

## By Audience

### I want to use Neve

- [user/install.md](user/install.md): installation, platform notes, binary cache setup
- [user/quickstart.md](user/quickstart.md): quickest path to first expression and file
- [user/tutorial.md](user/tutorial.md): learn the language surface systematically

### I want exact language truth

- [reference/spec.md](reference/spec.md): syntax and semantic rules
- [reference/api.md](reference/api.md): standard library reference
- [reference/diagnostics.md](reference/diagnostics.md): diagnostic code index

### I want to understand project reality

- [project/feature-matrix.md](project/feature-matrix.md): real support matrix
- [project/ecosystem-design.md](project/ecosystem-design.md): flake, store, builder, registry
- [project/changelog.md](project/changelog.md): released changes only
- [../.claude/forward-plan.md](../.claude/forward-plan.md): language completion roadmap

### I want to contribute

- [contributor/contributing.md](contributor/contributing.md): setup, workflow, style
- [contributor/onboarding.md](contributor/onboarding.md): codebase reading order
- [contributor/architecture.md](contributor/architecture.md): crate responsibilities and pipeline

---

## CLI Docs

```bash
neve doc index
neve doc quickstart
neve doc spec
neve doc api
neve doc contributing
neve doc feature-matrix
```

For command behavior, use `neve --help`.

---

<div align="center">

**[Main README](../README.md)** · **[License: MPL-2.0](../LICENSE)**

</div>
