# AUDIT-20260504-WP1-COMMITTED-TARGET-CLOSEOUT-REPAIR

- AUDIT_ID: AUDIT-20260504-WP1-COMMITTED-TARGET-CLOSEOUT-REPAIR
- STATUS: APPLIED
- DATE_UTC: 2026-05-04
- SCOPE: Repo Governance
- WP_ID: WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1
- RGF_ID: RGF-279

## Driver

Integration Validator source review for WP-1 reached PASS on accepted product commit `eddcf18ba08898dcf2b4a99e5b901ad80dba8aaa`, but final-lane progression was blocked by governed handoff/closeout proof that still preferred stale live-worktree and branch-range artifacts. The WP worktree also had preserved out-of-scope dirty product files that must not be cleaned or hidden by Orchestrator. Under active host load, cargo-based proof remained waived by Operator instruction.

## Change

- Validator handoff evidence now builds an explicit committed-target scope proof from the requested validation range, changed file list, signed packet scope, file budget, broad-tool allowlist, and `git diff --check`.
- Preserved out-of-scope dirty files and stale post-work manifest failures can be recorded as non-blocking only when the committed target scope proof passes for the explicit range under review.
- Committed validation evidence persists the committed scope proof fields so later closeout readers do not have to rediscover the target from live worktree state.
- Signed-scope validation now treats `Historical Target File` declarations as optional containment-only declarations, which prevents historical packet context from acting as a false required patch surface.
- Signed-scope candidate validation now accepts the durable `committed_validation_target` from validator evidence. Contained-in-main closeout therefore keeps validating the accepted range `040197df72b590f35034f3ec282dc4fb43515adc..eddcf18ba08898dcf2b4a99e5b901ad80dba8aaa` after local `main` has already fast-forwarded to `eddcf18ba08898dcf2b4a99e5b901ad80dba8aaa`.
- The `phase-check CLOSEOUT` RGF-183 guard now compares the closeout sync CWD to the actual live governance kernel root, not to the phase-check CWD, so a valid `handshake_main` run with `HANDSHAKE_GOV_ROOT` pointed at `wt-gov-kernel/.GOV` is not misclassified as a kernel-root closeout.
- `closeout-repair` can import the accepted candidate commit into `handshake_main`, detect default PASS verdict formatting, and generate/link a packet-local `signed-scope.patch` when the packet has commit artifacts but no patch artifact.

## Verification

- `node --test .GOV/roles_shared/tests/phase-check.test.mjs`
- `node --test .GOV/roles/validator/tests/validator-governance-lib.test.mjs`
- `node --test .GOV/roles/validator/tests/committed-validation-evidence-lib.test.mjs`
- `node --test .GOV/roles_shared/tests/signed-scope-surface-lib.test.mjs`
- `node --test .GOV/roles_shared/tests/signed-scope-surface-lib.test.mjs .GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
- `node --test .GOV/roles/orchestrator/tests/closeout-repair.test.mjs`
- `just phase-check HANDOFF WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1 WP_VALIDATOR "" --range 040197df72b590f35034f3ec282dc4fb43515adc..eddcf18ba08898dcf2b4a99e5b901ad80dba8aaa`

## Result

The committed handoff gate passed for `040197df72b590f35034f3ec282dc4fb43515adc..eddcf18ba08898dcf2b4a99e5b901ad80dba8aaa` with `committed_target_status=PASS` and `committed_scope_status=PASS`. `closeout-repair` imported the accepted target into `../handshake_main` and generated `.GOV/task_packets/WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1/signed-scope.patch`. The closeout sync CWD guard no longer rejects a product-main closeout run solely because phase-check itself also starts in the product worktree. Post-merge closeout can now validate the signed scope from durable committed evidence even when local `main` already equals the target commit.

## Residual Risk

This repair intentionally keeps cargo/clippy/full-build proof waived under the 2026-05-04 host-load instruction. The final merge and `CONTAINED_IN_MAIN` publication remain Integration Validator authority and must run from `../handshake_main` while preserving the existing dirty `AGENTS.md`.
