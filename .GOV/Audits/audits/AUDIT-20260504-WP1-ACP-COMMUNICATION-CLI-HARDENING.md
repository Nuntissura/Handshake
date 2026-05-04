# WP-1 ACP Communication CLI Hardening Audit

- AUDIT_ID: `AUDIT-20260504-WP1-ACP-COMMUNICATION-CLI-HARDENING`
- STATUS: APPLIED
- DATE: 2026-05-04
- SCOPE: repo governance communication helper hardening during `WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1`
- LANE: `ORCHESTRATOR_MANAGED`

## Trigger

The WP Validator kickoff moved the lane forward, but its optional metadata shifted across fields after named `spec_anchor=...`, `packet_row_ref=...`, and `microtask_json=...` arguments crossed the Just/Windows invocation boundary. The same kickoff also exposed `../handshake_main` paths in review metadata even though this WP's assigned worktree is `../wtc-closeout-posture-v1`.

## Findings

- Sparse optional arguments in direct-review helpers could be dropped by the shell and shift later fields left.
- Models may author optional receipt metadata as `key=value` even when a Just recipe declares positional optional arguments.
- Just wrappers could forward those values as nested `key=key=value` tokens after recipe-level stabilization.
- Coder and WP Validator startup prompts did not explicitly say that product navigation and proof commands must resolve from the assigned WP worktree when stale `handshake_main` paths appear in communication metadata.

## Repairs

- Added key=value optional metadata parsing to `wp-review-exchange.mjs` and `wp-thread-append.mjs`.
- Added nested key=value unwrapping so Just-forwarded values like `spec_anchor=spec_anchor=...` still land in the intended field.
- Updated direct-review and thread Just recipes to forward optional metadata as named tokens instead of positional empty placeholders.
- Added startup prompt routing law for Coder and WP Validator sessions so stale `../handshake_main` product paths are treated as navigation noise for active WP work.
- Added regression tests for direct-review and thread metadata parsing.

## Verification

- `node --check .GOV/roles_shared/scripts/wp/wp-review-exchange.mjs`
- `node --check .GOV/roles_shared/scripts/wp/wp-thread-append.mjs`
- `node --check .GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `node --test .GOV/roles_shared/tests/wp-review-exchange.test.mjs .GOV/roles_shared/tests/wp-thread-append.test.mjs`
- `just gov-check`
