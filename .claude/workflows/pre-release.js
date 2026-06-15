export const meta = {
  name: 'pre-release',
  description: 'Pre-release checklist: tests, clippy, fmt, changelog, version bump',
  phases: [
    { title: 'Verify', detail: 'cargo test + clippy + fmt' },
    { title: 'Changelog', detail: 'Check changelog is up to date' },
    { title: 'Deprecation', detail: 'Count deprecation warnings' },
    { title: 'Report', detail: 'Generate release readiness report' },
  ],
}

phase('Verify')
const verify = await parallel([
  () => agent('Run `cargo test --workspace` and report pass/fail count.', { label: 'test' }),
  () => agent('Run `cargo clippy --workspace --all-targets -- -D warnings` and report.', { label: 'clippy' }),
  () => agent('Run `cargo fmt --all -- --check` and report.', { label: 'fmt' }),
])

phase('Changelog')
const changelogCheck = await agent(
  'Check `docs/project/changelog.md`: is there an entry for the version being released? Does it list all recent changes (deprecations, fixes, features)?',
  { label: 'changelog' }
)

phase('Deprecation')
const deprecationCheck = await agent(
  'Search the entire codebase (`crates/`, `tests/`, `neve-cli/`) for `#[deprecated]` usage and count them. Also count `#![allow(deprecated)]` suppresses. Report totals.',
  { label: 'deprecation' }
)

phase('Report')
log(`Release readiness report:
  - Tests: ${verify[0] || '?'}
  - Clippy: ${verify[1] || '?'}
  - Format: ${verify[2] || '?'}
  - Changelog: ${changelogCheck || '?'}
  - Deprecations: ${deprecationCheck || '?'}
`)

return {
  verify: verify.filter(Boolean),
  changelog: changelogCheck,
  deprecations: deprecationCheck,
}
