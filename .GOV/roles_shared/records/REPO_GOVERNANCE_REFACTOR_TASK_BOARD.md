# Repo Governance Refactor Task Board

**Status:** Governance refactor complete; smoketest follow-on remediation open
**Scope:** Governance-only refactor tracking for `/.GOV/`  
**Authority:** `.GOV/roles_shared/docs/REPO_GOVERNANCE_REFACTOR_ROADMAP.md`

**Closeout record:** `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_CLOSEOUT.md`
**Changelog:** `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

## Notes

- This board tracks governance refactor items only.
- This board does not replace `.GOV/roles_shared/records/TASK_BOARD.md`.
- This board does not create or imply Work Packets.
- Status here is planning truth for governance refactor sequencing, not product execution truth.

## Status Keys

- `PLANNED` = defined but not yet implementation-ready
- `READY` = sequenced and safe to begin
- `IN_PROGRESS` = active refactor item
- `BLOCKED` = waiting on upstream item
- `DONE` = implemented and verified
- `HOLD` = intentionally deferred

## Board

| ID | Status | Workstream | Depends On | Primary Surfaces | Exit Signal |
|---|---|---|---|---|---|
| RGR-01 | DONE | Workflow Truth and Startup Gate | - | `roles_shared/checks`, `roles/orchestrator/scripts`, runtime ledgers | startup blocks split truth and false-ready state |
| RGR-02 | DONE | Transactional Prepare and Status Sync | RGR-01 | `roles/orchestrator/scripts`, `roles_shared/scripts/lib`, records/runtime | activation writes become coherent or fail cleanly |
| RGR-03 | DONE | Direct Review Boundary Enforcement | RGR-01, RGR-02 | communication helpers, receipt routing, communication checks | required coder-validator review becomes boundary-blocking |
| RGR-04 | DONE | Scope and Tool Spill Enforcement | RGR-01 | coder/validator checks, packet template, shared schemas | broad spill becomes visible and blockable |
| RGR-05 | DONE | Computed Policy and Evidence Gate | RGR-01, RGR-02, RGR-03 | shared records, schemas, policy checks, validator checks | final closure becomes computed rather than narrated |
| RGR-06 | DONE | Prevention Ladder, Legacy Cleanup, and Audit Capture | RGR-03, RGR-05 | deprecation surfaces, audit generators, session/runtime helpers | repeated escapes gain prevention assets and audit capture hardens |

## Audit Linkage Convention

- Governance maintenance items and changelog entries must link by stable audit IDs, not by Work Packet IDs.
- Current smoke-review drivers:
  - `AUDIT_ID: AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4`
  - `AUDIT_ID: AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260325-CONTRACT-HARDENING-V1`
  - `AUDIT_ID: AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
  - `SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW`
  - `AUDIT_ID: AUDIT-20260329-WORKFLOW-PROJECTION-CORRELATION-V1-SMOKETEST-PROOF-RUN-REVIEW`
  - `SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260329-WORKFLOW-PROJECTION-CORRELATION-V1`
- Historical comparison driver:
  - `AUDIT_ID: AUDIT-20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT`

## Post-Refactor Follow-On Board

| ID | Status | Workstream | Depends On | Evidence | Primary Surfaces | Exit Signal |
|---|---|---|---|---|---|---|
| RGF-01 | DONE | Chat-Visible Refinement Proof | - | `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` / `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4` | `roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`, `roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`, `roles/orchestrator/checks/orchestrator_gates.mjs` | signature cannot be requested until the full refinement block has been emitted as assistant-authored chat text |
| RGF-02 | DONE | Orchestrator Helper-Agent Product-Code Boundary | RGF-01 | `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` / `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4` | `roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`, `roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`, `roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`, `roles/orchestrator/checks/orchestrator_gates.mjs` | helper agents cannot write product code unless explicit operator approval is recorded in packet fields |
| RGF-03 | DONE | Merge Progression Truth and Main Containment Gate | - | `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` + `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW` | final review gates, closeout scripts, `TASK_BOARD.md`, runtime status surfaces | a validated PASS cannot close while the approved commit is absent from `main` unless status explicitly remains awaiting integration |
| RGF-04 | DONE | Integration-Validator Topology Preflight and Atomic Closeout | RGF-03 | `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` + `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW` | ACP broker control/status, final review scripts, session registry reconciliation | final review either finishes coherently or fails before partial closeout truth is written |
| RGF-05 | DONE | Session-Control Self-Settlement and Orphan Prevention | RGF-04 | `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` + `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW` | `SESSION_CONTROL_REQUESTS.jsonl`, `SESSION_CONTROL_RESULTS.jsonl`, broker outputs, repair helpers | every control request lands exactly one terminal result and orphaned rejected prompts stop requiring manual truth repair |
| RGF-06 | DONE | Historical Failure vs Live Smoketest Modeling | RGF-03 | `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT` + `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW` | `TASK_BOARD.md`, `WP_TRACEABILITY_REGISTRY.md`, `roles_shared/checks/historical-smoketest-lineage-check.mjs`, smoke-review naming, changelog linkage | failed historical closure and active smoketest lineage can coexist without split truth |
| RGF-07 | DONE | Operator-Facing Scope Split Discipline | - | `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW` | `roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`, `roles/coder/CODER_PROTOCOL.md`, `roles/validator/VALIDATOR_PROTOCOL.md`, `roles_shared/docs/START_HERE.md`, `roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`, `../handshake_main/AGENTS.md` | operator-facing answers now explicitly split `Handshake (Product)` from `Repo Governance`, and governance-themed product code is no longer mislabeled as repo governance |
| RGF-08 | DONE | Minimal Live Read Set and Startup Prompt Anti-Rediscovery Discipline | RGF-07 | `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW` | `roles_shared/scripts/session/session-control-lib.mjs`, `roles_shared/checks/protocol-alignment-check.mjs`, `roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`, role protocols, `SMOKETEST_REVIEW_TEMPLATE.md` | governed startup prompts now carry a minimal live read set, repeated protocol rereads/command rediscovery are treated as ambiguity signals, and protocol-alignment checks enforce the prompt contract |
| RGF-09 | DONE | Orchestrator-Managed Invalidity Rules | RGF-06 | `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW` | lifecycle gates, receipt/runtime truth, closeout/status rules, command surface law | workflow-invalid orchestrator-managed conditions now record `WORKFLOW_INVALIDITY`, block verdict/closure truth, and reject checkpoint-relapse helper commands mechanically |
| RGF-10 | DONE | Declared Topology Enforcement and Auxiliary Worktree Rejection | RGF-04 | `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW` | worktree helpers, topology checks, cleanup surfaces, worktree budget enforcement | declared WP topology is now machine-checked, undeclared auxiliary worktrees fail topology/closeout checks, and a direct per-WP topology inspection command exists |
| RGF-11 | DONE | Orchestrator-Managed Mid-Run Approval Relapse Guard | RGF-09 | `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW` | startup prompts, orchestrator-next flow, approval/signature gates, lifecycle checks | startup prompts now forbid routine post-signature Operator interruption on orchestrator-managed lanes, `just orchestrator-next` emits machine-visible `BLOCKER_CLASS` state, and only explicit blocker classes remain legal escalation paths |
| RGF-12 | DONE | Signed-Scope Compatibility Preflight and Governed Packet-Widening Policy | RGF-04 | `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW` | final review preflight, packet scope metadata, integration-validator flow, scope-governance helpers | final-lane closeout now requires recorded current-`main` compatibility truth, stale compatibility baselines fail preflight, and adjacent shared-surface widening must route to a follow-on or superseding packet instead of ad hoc scope drift |
| RGF-13 | DONE | Operator Rule Restatement Invalidity and Lane Reset | RGF-09, RGF-11 | `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW` | workflow-invalidity helpers, orchestrator monitor surfaces, lifecycle reset law | if the Operator has to restate a core orchestrator-managed lane rule, the run records a dedicated invalidity/reset condition instead of continuing as normal |
| RGF-14 | DONE | Terminal Closeout Projection Sync | RGF-03, RGF-04, RGF-05, RGF-12 | `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW` | `task-board-set`, runtime projection helpers, merge-progression truth checks | terminal Task Board transitions now reject packet-truth mismatches and sync packet/runtime closeout projections immediately so final truth lags less |
| RGF-15 | DONE | Command Family Simplification and Wrong-Tool Rejection | RGF-08, RGF-11 | `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW` | command surface, helper wrappers, wrong-lane rejection rules, startup prompts | the live command surface narrows further and obviously wrong helper families fail earlier |
| RGF-16 | DONE | Final-Lane Authority Context Bundle | RGF-10, RGF-12 | `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW` | final-lane brief/resume helpers, authority context surfaces, source-of-truth summaries | integration/final review can open one canonical context bundle instead of repeating path and authority inspection |
| RGF-17 | DONE | Integration-Validator Merge Execution and Orphaned WP Prevention | RGF-04, RGF-12, RGF-14, RGF-16 | `AUDIT-20260329-WORKFLOW-PROJECTION-CORRELATION-V1-SMOKETEST-PROOF-RUN-REVIEW` / `SMOKETEST-REVIEW-20260329-WORKFLOW-PROJECTION-CORRELATION-V1` | `roles/validator/VALIDATOR_PROTOCOL.md`, `roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`, `roles/validator/checks/integration-validator-closeout-check.mjs`, `roles/validator/scripts/integration-validator-closeout-sync.mjs`, `roles_shared/scripts/lib/packet-runtime-projection-lib.mjs` | orchestrator-managed WPs with final PASS-grade proof must end in one machine-visible terminal authority outcome: contained in `main`, explicit validator `FAIL` or `OUTDATED_ONLY`, or explicit abandon/discard truth with packet/runtime/task-board state synchronized |
| RGF-18 | DONE | Accurate WP Token Accounting and Drift Detection | RGF-05 | `AUDIT-20260329-WORKFLOW-PROJECTION-CORRELATION-V1-SMOKETEST-PROOF-RUN-REVIEW` / `SMOKETEST-REVIEW-20260329-WORKFLOW-PROJECTION-CORRELATION-V1` | `roles_shared/scripts/session/wp-token-usage-lib.mjs`, `roles_shared/scripts/session/wp-token-usage-report.mjs`, `roles_shared/tests/wp-token-usage-lib.test.mjs`, `roles/orchestrator/scripts/session-registry-status.mjs` | the governed WP token ledger matches raw `turn.completed` usage across all role session outputs closely enough to be trusted, and material drift becomes machine-visible FAIL state |
| RGF-19 | DONE | Compact Gate Output and Artifact-First Overflow Discipline | RGF-08, RGF-15 | `AUDIT-20260329-WORKFLOW-PROJECTION-CORRELATION-V1-SMOKETEST-PROOF-RUN-REVIEW` / `SMOKETEST-REVIEW-20260329-WORKFLOW-PROJECTION-CORRELATION-V1` | gate wrappers, `pre-work` / `post-work` output shaping, session-control summaries, command-surface docs | large gate/protocol/packet/status outputs default to compact summaries plus artifact pointers, and full verbose dumps stop entering model context unless explicitly requested |
| RGF-20 | DONE | Context-Brief Command Parity and No-Rediscovery Enforcement | RGF-15, RGF-16 | `AUDIT-20260329-WORKFLOW-PROJECTION-CORRELATION-V1-SMOKETEST-PROOF-RUN-REVIEW` / `SMOKETEST-REVIEW-20260329-WORKFLOW-PROJECTION-CORRELATION-V1` | `justfile`, `roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`, `roles/validator/VALIDATOR_PROTOCOL.md`, final-lane brief helpers/tests | every documented role context-brief helper exists and is callable through the sanctioned command surface, and missing helpers fail closed instead of sending the role back into protocol rediscovery |
| RGF-21 | DONE | Dirty Worktree and Shared-Junction Noise Compression | RGF-10, RGF-19 | `AUDIT-20260329-WORKFLOW-PROJECTION-CORRELATION-V1-SMOKETEST-PROOF-RUN-REVIEW` / `SMOKETEST-REVIEW-20260329-WORKFLOW-PROJECTION-CORRELATION-V1` | `pre-work`, `validator-handoff-check`, worktree topology helpers, session summaries | shared `.GOV` junction drift and large unrelated dirty-worktree surfaces are reported as counts plus bounded samples, not as giant enumerations that inflate model context cost |
| RGF-22 | DONE | Turn and Token Budget Enforcement for ACP Lanes | RGF-18, RGF-19, RGF-20 | `AUDIT-20260329-WORKFLOW-PROJECTION-CORRELATION-V1-SMOKETEST-PROOF-RUN-REVIEW` / `SMOKETEST-REVIEW-20260329-WORKFLOW-PROJECTION-CORRELATION-V1` | session policy, orchestrator monitor surfaces, invalidity helpers, session registry reporting | orchestrator-managed lanes expose per-role turn/token budgets, and large ambiguity-driven overruns become machine-visible blocker or invalidity conditions instead of silent spend |

## Refactor Sequence (Historical)

1. `RGR-01`
2. `RGR-02`
3. `RGR-03`
4. `RGR-04`
5. `RGR-05`
6. `RGR-06`

## Follow-On Sequence

1. `RGF-09`
2. `RGF-10`
3. `RGF-11`
4. `RGF-12`
5. `RGF-13`
6. `RGF-14`
7. `RGF-15`
8. `RGF-16`
9. `RGF-17`
10. `RGF-18`
11. `RGF-19`
12. `RGF-20`
13. `RGF-21`
14. `RGF-22`

## Explicit Holds

- Product-code remediation now runs through product Work Packets only; this board must not create or imply governance WPs.
- New Master Spec edits: `HOLD` for this refactor track.
- Work Packet creation for this governance roadmap: `HOLD` unless the operating model changes later.
