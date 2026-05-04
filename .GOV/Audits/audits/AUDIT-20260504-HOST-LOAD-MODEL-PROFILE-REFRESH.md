# Audit: Host-Load Waiver and Model-Profile Refresh

- AUDIT_ID: AUDIT-20260504-HOST-LOAD-MODEL-PROFILE-REFRESH
- STATUS: APPLIED
- DATE: 2026-05-04
- SCOPE: Repo Governance
- DRIVER: Operator restarted orchestrator-managed WP sessions after a coder launched cargo tests while the host was under sustained operator-owned download-script load; Operator also changed WP Validator from Claude to GPT-5.5 extra-high reasoning due Claude rate limits.
- RELATED_WP: WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- RELATED_RGF: RGF-275

## Findings

1. The active packet can record a host-load TEST/ENVIRONMENT waiver, but generated startup prompts and role protocols needed explicit language so roles do not infer that heavy tests must still run immediately.
2. Existing session-registry records could preserve an old validator launch profile across a deliberate session close/restart, which made packet-declared model changes harder to apply mechanically.
3. Future packet authors needed a deterministic waiver example for host-load test deferral, not prose-only chat context.

## Changes Applied

- Added host-load stance to generated governed role startup prompts: operator-owned downloads and external processes are out of scope; active host-load waivers convert covered heavy commands to `NOT_RUN_WAIVED` or deferred evidence.
- Updated Coder, Validator, and WP Validator protocols to honor active TEST/ENVIRONMENT host-load waivers and forbid touching operator-owned processes.
- Updated the task-packet template with a host-load/test-deferral waiver example.
- Updated session-registry restart behavior so closed/failed governed sessions can refresh requested launch model/profile fields from the active packet when restarted.
- Updated the active WP packet to set `WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH` and record the operator's active host-load and WP-validator model waivers.

## Verification

- `node --check .GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `node --check .GOV/roles_shared/scripts/session/session-registry-lib.mjs`
- `node --test .GOV/roles_shared/tests/session-registry-lib.test.mjs`
- `just gov-check`
