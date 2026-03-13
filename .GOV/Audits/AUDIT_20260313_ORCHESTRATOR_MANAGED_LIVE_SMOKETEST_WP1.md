# Audit: Orchestrator-Managed Live Smoke Test for WP-1

## METADATA
- AUDIT_ID: AUDIT-20260313-ORCH-MANAGED-LIVE-SMOKETEST-WP1
- DATE_UTC: 2026-03-13
- AUDITOR: Codex (governance review after live execution)
- SCOPE: End-to-end live smoke test of the new orchestrator-managed ACP workflow for `WP-1-Structured-Collaboration-Artifact-Family-v1`
- WP_ID: WP-1-Structured-Collaboration-Artifact-Family-v1
- RESULT: PASS WITH MATERIAL GOVERNANCE AND WORKFLOW FINDINGS
- IMPLEMENTATION_SOURCE_HEAD_VALIDATED: `84d37247e9e8e6ff6350fb109e5faaea821af9b9`
- FEATURE_BRANCH_CLOSEOUT_HEAD: `f9e89285a17dc2f7a2a19f08c09fd57c89e89d2b`
- MAIN_MERGE_COMMIT: `5367d86e960888ff1ccd04308bbc847e87442d7a`

---

## 1. EXECUTIVE SUMMARY

The new workflow did complete a real end-to-end WP run:

- governed Coder session implemented the WP
- governed WP Validator reached advisory `PASS`
- governed Integration Validator completed final validation and merged to `main`

That proves the ACP-first orchestrator-managed workflow is viable in practice.

It also exposed multiple real failures that would have caused loss of confidence, unnecessary babysitting, or unsafe operator assumptions if left unfixed:

- resume-state misclassification in coder governance
- ACP run timeout too short for real work
- stale broker reuse after governance changes
- historical control ledger invalidated by broker build bumps
- Windows line-ending false positives in topology validation
- stale validator bootstrap mirrors from `main`
- weak operator visibility around which artifact copy is actually authoritative

This was not a clean run. It was a successful run with meaningful defects discovered and patched under live pressure.

Overall assessment:

- Architecture direction: correct
- Governance implementation quality before smoke test: not yet operator-trustworthy
- Governance implementation quality after smoke test patches: materially stronger, but still not "fire and forget"

---

## 2. WHAT WAS PROVEN

### 2.1 Proven Capabilities

- The Orchestrator can start, steer, and resume governed Coder, WP Validator, and Integration Validator sessions through the ACP broker.
- A full WP can move from implementation to advisory validation to final merge without direct product-code editing from the Orchestrator.
- The broker/session ledger model is strong enough to preserve a usable audit trail across retries, cancels, and role handoffs.
- Final merge-to-main authority can be exercised by the Integration Validator while preserving the feature branch as backup truth.

### 2.2 Proven Commits

| Stage | Commit | Notes |
|------|--------|-------|
| First substantive implementation commit | `3274f16` | product implementation landed on feature branch |
| Validator-ready implementation source head | `84d3724` | validator PASS source head |
| Feature branch governance closeout head | `f9e8928` | validator gate / closeout on feature branch |
| Canonical merge to `main` | `5367d86` | final merge commit on `main` |

---

## 3. TECHNICAL FAILURES FOUND AND PATCHED

| # | Severity | Failure | Effect During Run | Resolution |
|---|----------|---------|-------------------|------------|
| 1 | HIGH | `coder-next` misclassified a claimed WP as `BOOTSTRAP` | Orchestrator got false resume guidance and could have restarted the WP incorrectly | Fixed in `coder-next.mjs` so resume depends on missing bootstrap claim, not stale packet/model markers |
| 2 | HIGH | ACP governed run timeout was only `900s` | real implementation run timed out while still making progress | Raised broker run timeout to `5400s` |
| 3 | HIGH | Broker build identity was not refreshed when governance behavior changed | stale in-memory broker kept old semantics after governance patch | Broker build id was bumped so stale brokers are rejected and restarted |
| 4 | HIGH | Historical settled result rows were treated as invalid after broker build change | `gov-check` failed immediately after legitimate broker upgrade | Validation changed so historical `broker_build_id` remains audit metadata rather than a same-build requirement |
| 5 | MEDIUM | `topology-registry-check` compared raw bytes instead of normalized text | clean Windows validation checkouts reported false stale-topology failures | EOL normalization added to topology registry check |
| 6 | MEDIUM | Initial launcher/bridge semantics overstated liveness earlier in the smoke test | terminal dispatch could be misread as live CLI/session proof | workflow was corrected toward ACP-backed thread identity and governed steering |
| 7 | MEDIUM | Integration-validator startup from `main` saw stale packet/runtime mirrors | first integration-validator message was "wait state" instead of actionable review | Orchestrator had to steer explicitly to the validated feature-branch head as source of truth |

---

## 4. SYSTEMIC FAILURES AND PROCESS GAPS

### 4.1 Mirror Drift Is Still Too Easy

The workflow still allows branch-local packet/task-board/runtime mirrors to diverge enough that:

- the Coder can believe the WP is handed off
- the WP Validator can see `FAIL`
- the Orchestrator viewport can still show stale authority fragments

This happened in practice:

- packet said `Done`
- task board still showed `[READY_FOR_DEV]`
- runtime state still showed coder-facing / not-ready-for-validation posture

This is a systemic problem, not a one-off typo. The workflow still depends on explicit sync discipline between:

- feature branch packet
- feature branch task board / WP communications
- validator read path
- orchestrator viewport

### 4.2 Session Success Does Not Automatically Mean Workflow Success

The governed Coder session successfully completed prompts, but that did not guarantee:

- the right commit was the one validators used as source of truth
- the committed branch state matched the narrative handoff
- the governance mirrors were validator-ready

This is a classic "vibe coding" risk in workflow form:

- the role can produce a coherent success narrative
- but the branch/packet/task-board/runtime surfaces may still be inconsistent

The validator caught this, which is good. But the workflow should make that mismatch harder to create.

### 4.3 Validation Worktrees Start From a Misleading Local Reality

Validator worktrees created from `main` are structurally correct, but operationally misleading for active WPs because they begin with stale packet/runtime copies. That means:

- the validator's first truthful view is often wrong for the active WP
- the Orchestrator must explicitly override the default local mirror and point the validator at the real feature-branch handoff state

That is workable, but it is not intuitive.

### 4.4 Safety Helpers Are Correct but Friction-Heavy

`backup-push` protected the safety rule correctly, but it was awkward in a multi-worktree live run because:

- it required a clean worktree
- the clean worktree used did not have the target local branch ref
- the owning feature worktree had the branch ref but was dirty

The safety intent was right. The operator ergonomics were poor.

---

## 5. AMBIGUOUS RULES, INSTRUCTION GAPS, AND CONFLICTS

### 5.1 "Packet Wins" vs "Use the WP Worktree as Source of Truth"

Both are defensible rules, but they are easy to misread together.

Actual working rule observed in the smoke test:

- the packet is the authority
- but for an active WP, the authoritative packet is the feature-branch/WP-worktree copy, not a stale mirror in another worktree

That distinction is not obvious enough operationally.

### 5.2 Validator Startup and Resume Wording Is Ambiguous for Active WPs

The validator protocols correctly emphasize startup and local worktree checks, but in a real run that can suggest the validator should trust its own startup worktree copy first. In practice, the validator often must validate:

- the explicit feature branch head
- the WP-assigned worktree
- or a clean detached checkout of the handoff commit

That should be stated much more bluntly.

### 5.3 Operator Approval Boundaries Are Clearer Than Tooling Behavior

The repo rules clearly require safety before destructive git actions. But helper behavior and live topology did not line up neatly:

- the operator had authorized the full end-to-end smoke test
- the workflow still had to work around helper constraints to safely push the feature branch backup before merge

This is not a law conflict, but it is a tooling-policy friction point.

### 5.4 The TUI Is a Viewport, but Its Inputs Still Need Better Truth Signaling

The TUI rule is correct: viewport only. The problem is not authority. The problem is operator confidence when multiple surfaces disagree.

The smoke test showed that the operator still needs better visibility into:

- which worktree/branch copy is being treated as active truth
- whether the displayed packet/runtime view is stale relative to the validated feature head
- whether validator state is local-mirror state or feature-head validation state

---

## 6. WORKFLOW BOTTLENECKS

### 6.1 Long-Running Product Work vs Broker Timeouts

A serious WP implementation prompt exceeded the original broker timeout. That means the original timeout was not sized for the real workload.

### 6.2 Governance Sync as a Repeated Manual Repair Step

The live run required repeated recovery/sync work around:

- packet/task-board/runtime alignment
- validator view alignment
- integration-validator source-of-truth alignment

This is too much manual orchestration for a workflow that claims semi-autonomous operation.

### 6.3 Validator Review Needed a Clean Detached Validation Path

The feature worktree contained unrelated dirt outside the WP scope. The validator had to reason around that by validating committed branch state instead of trusting the live worktree.

That is the right technical choice, but it is a bottleneck and a contamination risk.

### 6.4 Multi-Worktree Branch Ownership Is Operationally Sharp

The branch that mattered lived in one worktree, while the clean worktrees available for safety helpers did not always carry the same local branch refs. That slowed backup push and merge-prep operations.

---

## 7. RISK REGISTER

| Risk | Severity | Why It Matters | Smoke Test Verdict |
|------|----------|----------------|--------------------|
| Repo poison / dirty-worktree contamination | HIGH | unrelated modified files can pollute validation context and mislead branch-state assumptions | real risk observed; mitigated by commit-based validation, not eliminated |
| Vibe-coding style narrative drift | HIGH | role says "done" while committed branch/governance state is not actually validator-ready | real risk observed; validator caught it |
| False-positive governance failures | HIGH | operators lose trust and sessions loop on non-actionable failures | real risk observed twice (`broker_build_id`, topology registry EOL) |
| Stale mirror authority confusion | HIGH | wrong packet/runtime copy can be treated as current truth | real risk observed in validator and integration-validator startup |
| Deletion / cleanup risk | MEDIUM | complex multi-worktree state creates temptation for unsafe cleanup | not triggered, but risk remains materially real |
| Merge safety erosion under pressure | MEDIUM | under live pressure, operators may bypass backup safety to keep momentum | avoided this run, but tooling friction increases temptation |
| ACP opacity during long runs | MEDIUM | without careful output inspection, long commands can look hung or dead | mitigated by ledgers/output logs; still operationally sharp |

---

## 8. WHAT WORKED WELL

- The ACP broker and governed ledgers were good enough to support real session steering, retry, cancel, and audit.
- The WP Validator correctly rejected a handoff that was not yet governance-clean.
- The Integration Validator correctly owned the final merge-to-main authority.
- Commit-based validation was the right defense against dirty worktree contamination.
- Governance-only fixes could be applied in place without touching Handshake product runtime outside the WP's product scope.

---

## 9. RECOMMENDED NEXT ACTIONS

### Immediate

1. Make source-of-truth rules more explicit in validator and integration-validator startup guidance:
   - local startup worktree may be stale
   - active feature-branch handoff state wins for an in-flight WP
2. Keep the longer governed run timeout unless evidence shows it is too permissive.
3. Preserve the historical-result build-id rule as patched; do not re-tighten it.
4. Keep line-ending normalization in deterministic file validators where Git can materialize `CRLF`.

### Near-Term

1. Improve operator visibility of stale mirror vs active feature-head truth in the TUI.
2. Reduce manual mirror-sync burden between packet, task board, and runtime status at coder handoff.
3. Revisit backup-push ergonomics for multi-worktree branch ownership without weakening safety.

### Strategic

1. Treat "clean detached validation of committed handoff state" as the primary validation model, not an exception.
2. Reduce the gap between "session completed" and "workflow state is truly validator-ready."
3. Keep pressure against vibe-coded workflow closure by requiring validators to validate committed branch state, not narrative summaries.

---

## 10. FINAL JUDGMENT

This smoke test should be considered a success, but not a clean success.

The new orchestrator-managed ACP workflow is now credible because it survived a real WP from implementation through merge to `main`. But it only became credible after multiple live governance/tooling failures were exposed and patched.

The most important lesson is not "the workflow worked." The most important lesson is:

- the workflow worked only because the validator lanes were strong enough to reject false closure
- and because the Orchestrator could patch governance in place while preserving the role boundaries

That is a good foundation. It is not yet evidence that the workflow is effortless or foolproof.

---

*Audit complete. This document records the live smoke test outcome and the defects exposed during the run. It is intentionally candid because the goal is operational trust, not narrative polish.*
