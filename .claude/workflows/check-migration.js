export const meta = {
  name: 'check-migration',
  description: 'Check AST→HIR migration status across codebase',
  phases: [
    { title: 'Scan', detail: 'Scan for AST compat usage patterns' },
    { title: 'Analyze', detail: 'Categorize migration readiness' },
  ],
}

phase('Scan')
const scan = await parallel([
  () => agent(
    'Search for all uses of `AstEvaluator`, `AstEnv`, `neve_eval::compat` across the codebase (excluding `crates/neve-eval/src/`). Count per file.',
    { label: 'scan-ast-usage' }
  ),
  () => agent(
    'Search for `#[ignore]` tests in `tests/parser.rs`. List each one with its TODO comment and line number.',
    { label: 'scan-ignored' }
  ),
  () => agent(
    'Search `docs/reference/spec.md` for the "Known Implementation Gaps" table. What gaps remain?',
    { label: 'scan-gaps' }
  ),
])

phase('Analyze')
const analysis = await agent(
  `Based on the scan results, categorize migration readiness:
  1. Count files still using AST compat
  2. Count remaining #[ignore] tests
  3. Count remaining spec gaps
  4. Estimate % migration complete`,
  { label: 'analyze' }
)

log(analysis)
return { scan: scan.filter(Boolean), analysis }
