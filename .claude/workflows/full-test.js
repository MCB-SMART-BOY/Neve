export const meta = {
  name: 'full-test',
  description: 'Run the canonical validation gate and mechanically verify repository counts',
  phases: [
    { title: 'CI validation', detail: 'scripts/validate.sh --ci' },
    { title: 'Canonical counts', detail: 'scripts/counts.sh --json with mechanical declaration comparison' },
  ],
}

phase('CI validation')
const validation = await agent(
  'From the repository root, run `scripts/validate.sh --ci`. Use the runner-configured timeout; expose a timeout, cancellation, unavailable tool, or non-zero exit as a non-empty failure or explicitly unverified result. Do not replace failures with a success summary.',
  { label: 'validate-ci' },
)
log(validation)

phase('Canonical counts')
const counts = await agent(
  'From the repository root, run `scripts/counts.sh --json`, then run `scripts/check-docs.sh --strict` to mechanically compare every emitted key/value with the repository declarations. Use the runner-configured timeout; report missing sources, mismatches, timeout, cancellation, or either non-zero exit as failures or explicitly unverified results, not as an informational number report. Do not swallow a non-zero exit.',
  { label: 'counts-check' },
)
log(counts)

return {
  validation,
  counts,
}


