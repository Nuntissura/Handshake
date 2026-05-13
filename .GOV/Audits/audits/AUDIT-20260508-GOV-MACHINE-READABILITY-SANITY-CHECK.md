# AUDIT-20260508-GOV-MACHINE-READABILITY-SANITY-CHECK

## Metadata

- AUDIT_ID: `AUDIT-20260508-GOV-MACHINE-READABILITY-SANITY-CHECK`
- SMOKETEST_REVIEW_ID: `N/A`
- DATE_UTC: `2026-05-08T17:54:35Z`
- AUDITOR: `Codex`
- SCOPE: Repo Governance machine-readable artifact sanity check
- RELATED_GOVERNANCE_ITEMS:
  - `RGF-301`
  - `RGF-302`
  - `RGF-303`
  - `RGF-304`
  - `RGF-305`
  - `RGF-306`
- OUT_OF_SCOPE:
  - Handshake product code
  - Product work packets
  - Branch or worktree cleanup

## Result

- REVIEW_RESULT: `FAIL_FOR_DETERMINISTIC_INGEST_READINESS`
- SUMMARY: The machine-readable governance refactor is directionally correct, but several checks still prove file presence, projection hash parity, or shallow shape validity rather than full deterministic contract semantics for ACP or future software consumers.

## Findings Recorded

1. Topology discovery omits `.GOV/roles_shared/schemas/` and `.GOV/roles_shared/workflow_contracts/`, so contract authority files can be absent from `GOVERNANCE_TOPOLOGY.json` while topology checks pass. Follow-on: `RGF-301`.
2. `SESSION_CONTROL_REQUEST.schema.json` permits fewer roles than the live session-control runtime supports, creating schema/runtime divergence for ACP consumers. Follow-on: `RGF-302`.
3. Workflow contract validation accepts invalid transitions, unknown roles, and role/command-kind mismatches because current validation is mostly shape-oriented. Follow-on: `RGF-303`.
4. Packet/refinement/MT projection validation proves hash/header parity but does not validate primary contract schema or semantic completeness. Follow-on: `RGF-304`.
5. Residual writer inventory can classify risky authority writers as generic or future-tracked without failing the check, creating false confidence around remaining Markdown-first mutation paths. Follow-on: `RGF-305`.
6. Phase-bundle failure dossier fields can be present but non-evidentiary because empty topology rows and arbitrary memory-capture status strings are accepted. Follow-on: `RGF-306`.

## Verification Notes

- Focused checks observed passing before this record was created: `workflow-contract-check`, `public-surface-consolidation-check`, `packet-contract-projection-check`, `residual-artifact-writer-inventory-check`, `governance-topology-check`, and `session-control-runtime-check`.
- Negative probes showed acceptance of invalid workflow transitions, unknown-role workflow envelopes, bogus contract schema ids when projection hashes matched, and empty failure-dossier topology rows.
- Full `just gov-check` was attempted during the initial sanity check but did not complete in that command window; after recording the queued follow-ons, `just gov-check --sync-topology` and plain `just gov-check` both passed.

## Decision

- Do not mark existing non-`DONE` task-board items as `FAIL`.
- Keep existing `IN_PROGRESS` rows active.
- Record distinct hardening work as queued follow-ons with this audit as stable evidence.
