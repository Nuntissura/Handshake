# Governance Maintenance Workflow

Use this workflow for repo-governance maintenance that does not require a Work Packet.

## Use This Workflow Only When

- the planned diff is strictly limited to governance surfaces:
  - `/.GOV/**`
  - `/.github/**`
  - `/justfile`
  - `/AGENTS.md`
  - `/.GOV/codex/Handshake_Codex_v1.4.md`
- no Handshake product code is touched under `src/`, `app/`, or `tests/`
- no Master Spec main-body edit is required

If the planned change touches product code or the Master Spec, stop and use the normal refinement plus WP path instead.

## Authoritative Records

- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - open, sequence, and close governance-maintenance items here
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
  - record applied governance changes here
- `.GOV/Audits/**`
  - store the evidence driver here
- `.GOV/roles_shared/records/TASK_BOARD.md`
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
- `.GOV/roles_shared/records/BUILD_ORDER.md`
  - touch these only when the governance change actually changes product-WP projections

## Stable Linkage Rules

- Every governance-maintenance item should cite the evidence that opened it.
- Use `AUDIT_ID` for all audits.
- Use `SMOKETEST_REVIEW_ID` for smoketest or workflow-proof reviews.
- Use `CHANGESET_ID` in the changelog for every applied governance change.
- Prefer stable IDs over prose references when linking the task board, changelog, and audits.

## Templates

- `.GOV/templates/REPO_GOVERNANCE_TASK_ITEM_TEMPLATE.md`
- `.GOV/templates/REPO_GOVERNANCE_CHANGELOG_TEMPLATE.md`
- `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
- `.GOV/templates/AUDIT_TEMPLATE.md`
- `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`

## Workflow

1. Classify the work.
   - Confirm it is governance-only and does not require a WP, signature, or refinement.
2. Open or update the evidence document.
   - Give the audit or review a stable `AUDIT_ID`.
   - If it is a smoketest review, also assign a stable `SMOKETEST_REVIEW_ID`.
   - If it follows an earlier smoketest review, include a short explicit subsection named `What Improved vs Previous Smoketest` so recovery and closeout passes stay directly comparable.
   - Every smoketest review, closeout review, workflow-proof review, or workflow comparison audit must include the required `Post-Smoketest Improvement Rubric` section using `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`.
   - Every such review must also include the required `Silent Failures, Command Surface Misuse, and Ambiguity Scan` section. Treat repeated governance-document rereads, repeated command discovery, repeated wrong-tool usage, and repeated path/source-of-truth checks as explicit evidence of ambiguity and token-cost waste.
3. Open or update the governance item.
   - Add or update the row in `REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`.
   - Record dependencies, evidence IDs, primary surfaces, and the exit signal.
4. Apply the governance change.
   - Edit the relevant governance docs, checks, scripts, or records.
   - If `AGENTS.md` or the canonical root `justfile` must change, do that work from `handshake_main` on local `main`; do not author those files from `wt-gov-kernel` or a WP worktree.
5. Record the applied changes.
   - Add a changelog entry in `REPO_GOVERNANCE_CHANGELOG.md` with a stable `CHANGESET_ID`.
6. Sync any affected projections.
   - If `TASK_BOARD.md` or `WP_TRACEABILITY_REGISTRY.md` changed, run `just build-order-sync`.
7. Verify.
   - Minimum required verification: `just gov-check`.

## Role Expectations

- Orchestrator:
  - owns governance-maintenance sequencing and the shared governance task board
  - does not create a WP for repo-governance maintenance
- Validator:
  - may audit or remediate governance surfaces without a WP
  - must use the same task-board and changelog records so governance fixes stay visible
- Coder or generic Codex session:
  - may implement governance-only changes when explicitly assigned
  - must still follow this recordkeeping flow and may not drift into product code

## Non-Negotiables

- No Work Packet, no USER_SIGNATURE, and no refinement for pure repo-governance maintenance.
- No product-code edits under this workflow.
- No Master Spec edits under this workflow.
- `just gov-check` is mandatory before claiming completion.
