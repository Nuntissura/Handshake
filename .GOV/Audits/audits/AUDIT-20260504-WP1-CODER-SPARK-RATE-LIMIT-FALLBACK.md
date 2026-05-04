# AUDIT-20260504-WP1-CODER-SPARK-RATE-LIMIT-FALLBACK

- AUDIT_ID: AUDIT-20260504-WP1-CODER-SPARK-RATE-LIMIT-FALLBACK
- STATUS: APPLIED
- DATE_UTC: 2026-05-04
- SCOPE: Repo Governance
- WP_ID: WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- RGF_ID: RGF-278

## Driver

During MT-004, the Coder's GPT-5.3 Codex Spark session failed at remote compaction with a usage-limit error and a reset hint of 12:22 PM. A direct `session-send ... FALLBACK` still resumed the Spark-bound thread, so it failed before sampling. The Orchestrator closed that thread and started a fresh governed Coder session, which registered `OPENAI_GPT_5_5_XHIGH`.

## Change

- The active WP packet now declares `CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH` and `CODER_MODEL: gpt-5.5`.
- The prior Spark waiver is marked `SUPERSEDED`.
- A new active fallback waiver records the Spark quota failure and the temporary GPT-5.5 Coder profile.

## Verification

- `just start-coder-session WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1 FALLBACK` accepted a fresh Coder session after closing the Spark-bound thread.
- `just session-registry-status WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1` showed Coder `requested_profile_id: OPENAI_GPT_5_5_XHIGH`.
- `just gov-check` still needs to run after the active WP lane is stable enough for governance verification.

## Residual Risk

The session-control fallback selector still needs a governance hardening follow-up: explicit `FALLBACK` on `session-send` did not supersede an existing Spark-bound thread. The runtime workaround is recorded here; a script-level fix should be done outside the critical MT-004 recovery path.
