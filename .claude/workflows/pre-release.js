export const meta = {
  name: 'pre-release',
  description: 'Run the release validation gate, canonical count comparison, and release metadata checks',
  phases: [
    { title: 'Release validation', detail: 'scripts/validate.sh --release' },
    { title: 'Canonical counts', detail: 'scripts/counts.sh --json with mechanical declaration comparison' },
    {
      title: 'Release metadata',
      detail: 'Cargo.toml ↔ docs declarations ↔ latest changelog release title and complete entry',
    },
  ],
}

phase('Release validation')
const validation = await agent(
  'From the repository root, run `scripts/validate.sh --release`. Use the runner-configured timeout; this single release gate must verify Cargo.toml workspace version consistency with docs declarations and the latest release title in `docs/project/changelog.md`, and must check changelog completeness. Expose a timeout, cancellation, unavailable tool, or non-zero exit as a non-empty failure or explicitly unverified result; do not replace failures with a success summary.',
  { label: 'validate-release' },
)
log(validation)

phase('Canonical counts')
const counts = await agent(
  'From the repository root, run `scripts/counts.sh --json`, then run `scripts/check-docs.sh --strict` to mechanically compare every emitted key/value with the repository declarations. Use the runner-configured timeout; report missing sources, mismatches, timeout, cancellation, or either non-zero exit as failures or explicitly unverified results, not as an informational number report. Do not swallow a non-zero exit.',
  { label: 'counts-check' },
)
log(counts)

phase('Release metadata')
const releaseMetadata = await agent(
  'From the repository root, run `scripts/check-docs.sh --release --strict`. Use the runner-configured timeout; mechanically verify Cargo.toml workspace version ↔ docs version declarations ↔ the latest `## [version]` heading in `docs/project/changelog.md`, and verify the latest changelog entry is complete with its required sections and content. Expose any missing check, mismatch, timeout, cancellation, unavailable tool, or non-zero exit as a non-empty failure or explicitly unverified result; do not summarize it as an informational report.',
  { label: 'release-metadata' },
)
log(releaseMetadata)

return {
  validation,
  counts,
  releaseMetadata,
}
