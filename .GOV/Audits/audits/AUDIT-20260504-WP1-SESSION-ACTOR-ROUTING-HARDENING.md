# AUDIT-20260504-WP1-SESSION-ACTOR-ROUTING-HARDENING

- AUDIT_ID: `AUDIT-20260504-WP1-SESSION-ACTOR-ROUTING-HARDENING`
- STATUS: APPLIED
- DATE: 2026-05-04
- SCOPE: repo governance ACP/session routing during `WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1`
- DRIVER: MT-001/MT-002 review traffic exposed mismatched broker `session_key` and receipt `actor_session` strings (`CODER:WP-...`, `coder:wp-...`, and `CODER:CODER:20260504-070818`), causing an initial validator response rejection and brittle manual review prompts.

## Finding

`send-mt` and generated role prompts presented synthesized broker session keys as if they were receipt actor sessions. Direct-review helpers require exact reversed role/session continuity, so a validator response can be rejected even when the role and correlation are correct if the response targets a synthesized session string instead of the open review item's `actor_session`.

## Change

- `send-mt-prompt.mjs` now distinguishes broker `session_key` from receipt `actor_session` and uses live runtime/registry context to fill manual `wp-review-request` actor and target sessions.
- Generated Coder/WP Validator prompt guidance now tells roles to use the exact open review item session strings for responses.
- Role and orchestration docs now state that `actor_session`/`target_session` are exact routing values and must not be reconstructed from broker keys.

## Verification

- `node --check .GOV/roles_shared/scripts/session/send-mt-prompt.mjs`
- `node --check .GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `just gov-check`

## Outcome

Future microtask dispatch prompts should reduce same-role session alias drift, keep direct-review ack matching deterministic, and avoid forcing Orchestrator to repair routine validator responses by hand.
