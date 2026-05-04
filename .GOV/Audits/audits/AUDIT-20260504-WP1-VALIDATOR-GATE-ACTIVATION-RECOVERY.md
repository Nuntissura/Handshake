# WP-1 Validator Gate Activation Recovery Audit

- AUDIT_ID: `AUDIT-20260504-WP1-VALIDATOR-GATE-ACTIVATION-RECOVERY`
- STATUS: APPLIED
- DATE: 2026-05-04
- SCOPE: repo governance recovery during `WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1` activation
- LANE: `ORCHESTRATOR_MANAGED`

## Trigger

The Activation Manager produced refinement content but stalled before normal completion, and the Orchestrator had to keep the autonomous run moving without another Operator handoff after signature approval.

## Findings

- `record-refinement` rejected canonical force-multiplier pillar names containing commas, including `Task board (product, not repo)`.
- `record-refinement` rejected the template-shaped `Tooltip: text` UI control rows.
- The refinement template used `Prompt-to-Spec` while the checker canonicalized the pillar as `Spec to prompt`.
- `orchestrator-prepare-and-packet` created a WP worktree from stale local `main`, which inherited an obsolete artifact target path and failed artifact hygiene.
- The packet checkpoint committed local symlinks to `AGENTS.md`, `.claude`, and `.github` on `gov_kernel`; those root control surfaces belong to `main`, not the governance kernel branch.
- `gov-check` launched subprocess checks from `handshake_main`, causing relative `.GOV/task_packets` reads to target stale backup governance instead of the live kernel.
- The first `WP_VALIDATOR` Claude startup auto-backgrounded the long `validator-startup` command and then launched duplicate startup/gov-check chains while trying to wait for completion.
- The repaired `WP_VALIDATOR` startup avoided duplicate reruns, but the model ended its START_SESSION turn after saying it would wait while the backgrounded bootstrap task was still active.

## Repairs

- Hardened refinement checking so known canonical values containing commas parse correctly in force-multiplier rows.
- Aligned the refinement template and checker behavior for `Spec to prompt`.
- Relaxed UI control tooltip detection to accept the documented `Tooltip: text` shape.
- Hardened `worktree-add` to fetch `origin/main`, fast-forward local `main` only when safe, and then create or reuse WP worktrees from current canonical state.
- Removed `AGENTS.md`, `.claude`, and `.github` from the `gov_kernel` index while retaining local symlinks for operator context.
- Hardened `gov-check` so child checks run from the live kernel worktree while `HANDSHAKE_ACTIVE_REPO_ROOT` still points at the canonical product worktree.
- Hardened generated START_SESSION prompts so governed CLI models use long tool timeouts for bootstrap commands and monitor backgrounded tasks instead of relaunching duplicate startup commands.
- Added a generated START_SESSION active-wait gate that forbids ending the turn, saying "I'll wait", or reporting final startup state while a bootstrap background task remains active.

## Verification

- `node --check .GOV/roles_shared/checks/refinement-check.mjs`
- `just record-refinement WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1`
- `just record-signature WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1 ilja040520260128 ORCHESTRATOR_MANAGED Coder-A`
- `just record-role-model-profiles WP-1-Software-Delivery-Validator-Gate-Closeout-Posture-v1 OPENAI_GPT_5_5_XHIGH OPENAI_GPT_5_5_XHIGH CLAUDE_CODE_OPUS_4_7_THINKING_XHIGH OPENAI_GPT_5_5_XHIGH OPENAI_GPT_5_5_XHIGH`
- `node --check .GOV/roles_shared/scripts/topology/worktree-add.mjs`
- `node --check .GOV/roles_shared/checks/gov-check.mjs`
- `node --check .GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `just gov-check`
