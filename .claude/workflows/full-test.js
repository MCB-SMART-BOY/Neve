export const meta = {
  name: 'full-test',
  description: 'Run full test suite: unit + integration + E2E + clippy + fmt + differential',
  phases: [
    { title: 'Unit + Integration', detail: 'cargo test --workspace' },
    { title: 'E2E', detail: 'cargo test --test end_to_end -- --nocapture' },
    { title: 'Clippy', detail: 'cargo clippy --workspace --all-targets -- -D warnings' },
    { title: 'Format', detail: 'cargo fmt --all -- --check' },
    { title: 'Differential', detail: 'AST vs HIR parity tests' },
  ],
}

phase('Unit + Integration')
const unitResult = await agent('Run `cargo test --workspace` and report any failures.', {
  label: 'unit-test'
})
log(unitResult || 'Unit tests: no output captured')

phase('E2E')
const e2eResult = await agent('Run `cargo test --test end_to_end -- --nocapture` and count passed/failed/ignored.', {
  label: 'e2e-test'
})
log(e2eResult || 'E2E tests: no output captured')

phase('Clippy')
const clippyResult = await agent('Run `cargo clippy --workspace --all-targets -- -D warnings` and report any warnings or errors.', {
  label: 'clippy'
})
log(clippyResult || 'Clippy: clean')

phase('Format')
const fmtResult = await agent('Run `cargo fmt --all -- --check` and report if any files would be reformatted.', {
  label: 'fmt'
})
log(fmtResult || 'Format: clean')

phase('Differential')
const diffResult = await agent('Count the number of `#[ignore]` tests and `#[deprecated]` usages across `tests/` and `crates/`. Report findings.', {
  label: 'differential'
})
log(diffResult || 'Differential: clean')

return {
  unit: unitResult,
  e2e: e2eResult,
  clippy: clippyResult,
  fmt: fmtResult,
  differential: diffResult,
}
