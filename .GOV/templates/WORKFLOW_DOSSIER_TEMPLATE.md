# Workflow Dossier Template

Use this template for the canonical live run artifact created at WP activation and maintained through closeout.

## Purpose

The Workflow Dossier is:

- a live execution dossier opened at activation time
- seeded mechanically from ACP/session-control and WP runtime artifacts
- maintained during the run by the Orchestrator
- appended by roles through the live findings surfaces
- finalized with closeout judgment, drift assessment, and rubric scoring

## Migration Rule

During the migration window, the full scaffold remains compatible with `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`.

Use that full section structure with these semantic rules:

- `DOCUMENT_KIND` should be `LIVE_WORKFLOW_DOSSIER`
- `WORKFLOW_DOSSIER_ID` is the canonical artifact id for new runs
- `SMOKETEST_REVIEW_ID` remains as a compatibility id until downstream lineage and memory tooling finish migrating
- the live dossier should be created at WP activation, not reconstructed at closeout
- the ACP/session-control snapshot should appear before the final review/opinion sections
- the final judgment section should be treated as a closeout layer inside the dossier, not the dossier itself

## Required Live Sections

The dossier must retain these append-only live sections:

- `LIVE_EXECUTION_LOG`
- `LIVE_IDLE_LEDGER`
- `LIVE_GOVERNANCE_CHANGE_LOG`
- `LIVE_CONCERNS_LOG`
- `LIVE_FINDINGS_LOG`

Formatting rule for `LIVE_EXECUTION_LOG`:

- prefer compact append-only bullet lines over wide tables
- prefer lane-style mechanical paths such as `ORCHESTRATOR -> ACP -> CODER` or `CODER -> ACP -> ORCHESTRATOR` when logging ACP movement
- ACP/session-control live entries should read as short stage records, for example `run.started`, `process.spawned`, `thread.started`, `result`, `terminal.reclaimed`
- keep the timestamp, role surface, stage, and only the few fields needed to diagnose stalls or routing drift
- include the latest per-lane ACP activity summary when available, for example a recent `file_change`, `web_search`, or `command_execution`, so idle ledgers can be compared against actual lane progress
- prefer grouped mechanical ledgers such as `counts{...} | route{...} | settlement{...} | repomem{...} | tokens{...} | host{...}` over one long undifferentiated field list
- include token-cost telemetry as grouped diagnostics: policy, enforcement mode, budget status, ledger health, gross/fresh/cached input, output, turns, and commands
- assume host load is heavy; shell timeout observations belong under `host{...}` or findings, not as standalone workflow truth

Formatting rule for `LIVE_IDLE_LEDGER`:

- keep it mechanical and compact; prefer one append-only line per sync
- report latency and drift as short ledgers, not prose
- include request-to-response timing, validator-pass-to-coder timing, current/max idle gaps, wall-clock attribution buckets (active build, validator wait, route wait, dependency wait, human wait, repair overhead), queue-pressure counts, and drift markers such as duplicate receipts or unresolved control rows
- group idle output into stable blocks such as `latency{...} | idle{...} | wall_clock{...} | current_wait{...} | queue{...} | drift{...}` so raw data stays readable

## Required Mechanical Evidence Sections

The dossier must expose at least:

- ACP/session-control state
- broker state and active-run projection
- request/result counts
- receipt and notification counts
- runtime status and next-actor projection
- microtask seed rows or explicit `MICROTASKS_NOT_USED`
- token-cost diagnostics with gross/fresh/cached usage, budget status, ledger health, and a `HEAVY_ASSUMED` host-load stance

The Workflow Dossier is diagnostic evidence only. By itself it must not block product outcome; only artifacts that define or judge product correctness may do that.

## Required Closeout Sections

At closeout, the dossier must still include:

- structured failure ledger
- role review
- communication audit
- cost attribution
- positive controls
- ambiguity and silent-failure scan
- the canonical rubric from `.GOV/roles_shared/docs/WORKFLOW_DOSSIER_RUBRIC.md`

## Compatibility

If a tool or document still asks for the smoketest review template, use the same scaffold but treat `Workflow Dossier` as the canonical concept.
