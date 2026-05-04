# Audit: Coder Handoff Microtask Commit Range

- AUDIT_ID: AUDIT-20260504-CODER-HANDOFF-MICROTASK-COMMIT-RANGE
- STATUS: APPLIED
- DATE: 2026-05-04
- SCOPE: Repo Governance
- DRIVER: Active orchestrator-managed WP coder handoff was blocked after a valid product commit because committed handoff preflight validated the packet branch range instead of the coder-declared microtask handoff commit.
- RELATED_WP: WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- RELATED_RGF: RGF-276

## Findings

1. `wp-coder-handoff` accepted structured microtask JSON with a product commit, but the committed handoff gate did not use that commit to derive the validation range.
2. The fallback packet range could pull unrelated historical branch work into `post-work-check`, producing false manifest and evidence failures for files outside the current microtask.
3. The handoff range should be accepted only when the supplied full SHA resolves to the prepared WP worktree `HEAD`; otherwise the gate must fail closed.
4. The Windows/Just wrapper path can fail to deliver `microtask_json` as a parsed named option when wrapper quotes survive argument forwarding, so the split preflight must also recover the canonical handoff commit from summary text.
5. Once the handoff commit became first-class in `microtask_contract`, the receipt/runtime schemas needed to allow and validate that field instead of rejecting the otherwise valid handoff receipt.

## Changes Applied

- Added `resolveMicrotaskCommittedHandoffRange()` to derive `<parent>..<commit>` from `microtask_json.commit`, `commit_sha`, or `head_commit`.
- Required the supplied commit to be a full 40-character SHA and match current prepared worktree `HEAD`.
- Passed the microtask-derived range into committed `CODER_HANDOFF` preflight before falling back to packet-level committed handoff range logic.
- Hardened `wp-review-exchange` named-option parsing against wrapper quotes and added `CODER_HANDOFF` summary commit inference as a fallback for the split preflight path.
- Added `commit`, `commit_sha`, and `head_commit` as nullable 40-character SHA fields in microtask contracts for receipt/runtime validation and schemas.
- Added a regression test using a temporary git repository to prove the helper returns the `HEAD^..HEAD` range for the handoff commit.
- Added review-exchange regressions for wrapper-quoted metadata and summary-derived handoff commit contracts.
- Added communication-schema regression coverage for accepted and malformed microtask handoff commit fields.

## Verification

- `node --check .GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
- `node --check .GOV/roles_shared/scripts/wp/wp-review-exchange.mjs`
- `node --check .GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
- `node --test .GOV/roles_shared/tests/wp-receipt-append.test.mjs`
- `node --test .GOV/roles_shared/tests/wp-review-exchange.test.mjs`
- `node --test .GOV/roles_shared/tests/wp-communications-lib.test.mjs`
