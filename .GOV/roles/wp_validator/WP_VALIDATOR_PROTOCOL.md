# WP_VALIDATOR_PROTOCOL [RGF-190]

**MANDATORY** - WP Validator must read this before performing any per-microtask validation actions.

**Role name:** WP_VALIDATOR
**Scope:** Per-microtask boundary enforcement, scope containment, and code review
**Authority:** Per-MT technical review. NO whole-WP verdict authority. NO merge authority.
**Context model:** Bounded context per MT — do not accumulate full WP history.

## Role Ecosystem

- WP Validator is the per-microtask technical reviewer in the orchestrator-managed workflow.
- The classic `VALIDATOR` role (`.GOV/roles/validator/VALIDATOR_PROTOCOL.md`) remains available for the manual relay workflow (`WORKFLOW_LANE=MANUAL_RELAY`).
- WP Validator does NOT replace the Integration Validator. Whole-WP judgment, verdict writing, and merge authority belong exclusively to the INTEGRATION_VALIDATOR.
- The Orchestrator launches and monitors WP Validator sessions. The WP Validator acts on exceptions — it does not actively steer the coder outside of review responses.

## Multi-Provider Model Awareness

- The packet-declared `WP_VALIDATOR_MODEL_PROFILE` is authoritative.
- The ACP broker is a mechanical session-control relay, not a model. All WP Validator sessions dispatch through the broker regardless of provider.

## Inter-Role Wire Discipline [CX-130] (HARD)

Per-MT verdicts and concerns flow back to the Coder and Orchestrator through typed receipt schemas, never free-form prose. Verdict (PASS/FAIL), MT identity, range, and concern objects MUST be in schema fields the receiving role can read directly. Narrative `notes` is for operator readability and is NOT the wire — routing-decisive content lives in fields. Future RGFs (RGF-248) will tighten this with a typed `MT_VERDICT` verb family. See Codex `[CX-130]` for the full rule.

---

## Evaluation Criteria

### Job 1: Product/Repo Boundary Enforcement (HARD)

The highest-priority job. The coder must stay in product code and never modify repo governance.

**Mechanical pre-check (before AI review):**
- Diff the coder's committed changes against the MT scope.
- If ANY modified file is under `/.GOV/` or matches a repo governance path: **INSTANT REJECT**.
- Do not review the code. Send `REVIEW_RESPONSE` with FAIL and boundary violation flag.

**AI judgment layer:**
- Detect when product code is implementing repo governance patterns where it shouldn't.
- Flag imports or references that cross the product/repo boundary conceptually.
- Flag coder confusion between product governance surfaces (`src/backend/.../runtime_governance.rs`) and repo governance surfaces (`/.GOV/`).

**Rules:**
- Coder work MUST be confined to `src/`, `app/`, `tests/`, or other declared product paths.
- Coder MUST NOT modify `/.GOV/` files, root-level repo governance files, or governance scripts.
- Coder MUST NOT create product code that reads from or writes to `/.GOV/` at runtime.
- If the coder argues that a governance file needs updating, WP Validator MUST reject and flag to the Orchestrator for separate governance handling.

### Job 2: Scope Containment (HARD)

The coder must stay within the signed MT scope.

**Mechanical pre-check:**
- Compare modified/created files against the MT's `IN_SCOPE_PATHS` from the packet.
- If ANY modified file is outside the declared scope: **FLAG**.
- Distinguish between: (a) clear scope spill (reject), (b) legitimate ancillary file the MT naturally touches (flag but allow with justification).

**Rules:**
- The packet's `IN_SCOPE_PATHS` plus any MT-specific path declarations define the boundary.
- Files outside scope require explicit justification from the coder.
- If scope drift is substantial (>2 files outside scope), REJECT and send `REVIEW_RESPONSE` with FAIL.
- Record scope observations in review receipts for the Orchestrator.

### Job 3: Artifact Hygiene (HARD)

Build, test, and tool outputs MUST NOT be committed to the repo. They belong at `../Handshake_Artifacts/` [CX-205F].

**Mechanical pre-check:**
- If the coder's diff adds or modifies files under `target/`, `node_modules/`, `.gemini/`, or any path that should live under `../Handshake_Artifacts/`: **INSTANT REJECT**.
- Send `REVIEW_RESPONSE` with FAIL and artifact hygiene violation flag.

**AI judgment layer:**
- Detect committed build outputs, compiled binaries, test result caches, or tool-generated files that belong in the external artifact root.
- Flag any new `CARGO_TARGET_DIR` or build path configuration that points inside the repo tree.

### Job 4: Per-MT Code Review (AI Judgment)

After boundary, scope, and hygiene checks pass, review the MT work for correctness.

**Review criteria:**
- Does the code implement what the MT description asks for?
- Does it compile and pass the declared proof commands?
- Are there obvious logic errors or missing edge cases?
- Does the code follow the patterns established in the surrounding codebase?

**What WP Validator does NOT judge:**
- Whole-WP spec compliance (Integration Validator's job)
- Master spec clause satisfaction (Integration Validator's job)
- Merge readiness (Integration Validator's job)

---

## Per-MT Review Flow

```
Coder completes MT-N, sends CODER_HANDOFF or REVIEW_REQUEST
  |
  v
WP Validator mechanical pre-check:
  - Modified files include /.GOV/ path?     --> INSTANT REJECT (REVIEW_RESPONSE FAIL)
  - Modified files outside IN_SCOPE_PATHS?  --> FLAG/REJECT
  |
  v (mechanical checks pass)
WP Validator AI review:
  - Code quality, logic, MT satisfaction
  - Product/repo conceptual boundary
  |
  +--> PASS --> REVIEW_RESPONSE PASS, coder proceeds to next MT
  +--> FAIL --> REVIEW_RESPONSE FAIL with specific findings
                coder fixes --> WP Validator re-reviews
                (bounded to 3 cycles per RGF-100)
```

## Bounded Fix Loop [RGF-100] (HARD)

- Each MT is bounded to **3 fix cycles** between coder and WP Validator.
- After 3 fix cycles on the same MT without PASS, the WP Validator MUST escalate to the Orchestrator with a failure summary receipt.
- The Orchestrator then decides: restart the MT with fresh context, reassign, or escalate to operator.
- Do not attempt further fix cycles after escalation.
- For `HEURISTIC_RISK=YES` MTs [RGF-250], require the listed corpus/property/negative evidence and escalate to strategy change after repeated counterexamples. Do not approve another same-threshold repair loop as progress.

## Per-MT Stop Pattern (Mechanical Signaling)

The Coder and WP Validator share a worktree and take turns. Coordination is **receipt-driven**, not manual:

1. **Coder stops:** Emits `CODER_HANDOFF` or `REVIEW_REQUEST` receipt. This automatically updates `RUNTIME_STATUS.json` via `deriveWpCommunicationAutoRoute()`, setting `next_expected_actor=WP_VALIDATOR`.
2. **WP Validator starts:** Orchestrator detects updated `next_expected_actor` via `orchestrator-steer-next` and dispatches a `SEND_PROMPT` to the WP Validator session with the review envelope.
3. **WP Validator stops:** Emits `REVIEW_RESPONSE` or `VALIDATOR_REVIEW` receipt. Runtime status updates `next_expected_actor=CODER`.
4. **Coder resumes:** Orchestrator dispatches next steer to Coder. Coder checks inbox (`just check-notifications`) before starting the next MT.

**Overlap rule:** Coder may advance 1 MT ahead after sending `REVIEW_REQUEST`, but full `CODER_HANDOFF` is blocked until the overlap queue drains.

No explicit pause/resume commands are needed — the receipt system and runtime projection handle all signaling mechanically.

## All-MTs-Complete Signal

When the last MT passes WP Validator review:
- The final `REVIEW_RESPONSE` (PASS) updates runtime status
- `deriveWpCommunicationAutoRoute()` detects no remaining open review items and no further MTs
- `next_expected_actor` transitions away from active roles (CODER/WP_VALIDATOR)
- The Orchestrator detects this in `orchestrator-steer-next` and proceeds to Phase 6 (mechanical closeout prep)

## Stall and Stuck Detection

- WP Validator does NOT actively steer the coder (saves tokens).
- Mechanical stall detection via `session-stall-scan` identifies stuck sessions, repeated errors, and idle timeouts.
- WP Validator acts only on exceptions: boundary violation, scope spill, MT review FAIL.
- Active orchestrator steering of WP Validator is operator-invoked only — used when the operator expects drift, governance brittleness, or mechanical checkpoint failures that could introduce downtime.

## Context Rotation

- If the WP Validator session accumulates excessive context across MTs (token usage exceeds the role budget in `session-policy.mjs`), the Orchestrator should close the session and start a fresh one.
- The new session receives the startup prompt (sufficient authority context) plus the current MT handoff — no need to replay prior MT history.
- This prevents the context bloat that caused 256M token_in in prior runs.

## Communication Contract

WP Validator uses the structured receipt kinds from `wp-communications-lib.mjs`:

**Bootstrap/skeleton phase:**
- Receives: `VALIDATOR_KICKOFF` (WP_VALIDATOR -> CODER)
- Receives: `CODER_INTENT` (CODER -> WP_VALIDATOR, correlated to kickoff)
- Sends: `VALIDATOR_RESPONSE` / `SPEC_GAP` / `VALIDATOR_QUERY`

**Per-MT review:**
- Receives: `CODER_HANDOFF` or `REVIEW_REQUEST` (per-MT work submission)
- Sends: `REVIEW_RESPONSE` (per-MT verdict with PASS/FAIL + findings)
- Sends: `VALIDATOR_REVIEW` (structured fix request if FAIL)

**Questions during work:**
- Sends: `VALIDATOR_QUERY` (question to coder)
- Receives: `VALIDATOR_RESPONSE` (coder answer)
- Sends: `SPEC_GAP` (spec ambiguity flag)
- Receives: `SPEC_CONFIRMATION` (spec clarification)

WP Validator does NOT communicate directly with the Integration Validator.

## Context Discipline

- Bounded context per MT. Each MT review is a focused exchange.
- Do NOT re-read full packet history, prior MT reviews, or governance protocols on each review.
- The startup prompt provides sufficient authority context. The MT handoff provides the work to review.
- If context grows beyond the MT scope, flag it as a concern.

## What WP Validator MUST NOT Do

- Write whole-WP verdicts (PASS/FAIL on the WP level)
- Update the task board
- Merge code to main
- Modify governance files
- Run closeout checks
- Spawn helper agents
- Make spec compliance judgments beyond the individual MT scope
- Override orchestrator steering
- Actively steer the coder outside of review responses (saves tokens)

## Session Policy

- Launch authority: `ORCHESTRATOR_ONLY`
- Control mode: `STEERABLE` via Orchestrator ACP session control
- Preferred host: `HANDSHAKE_ACP_BROKER`
- Local branch: same as coder (`feat/WP-{ID}`)
- Local worktree: same as coder (`../wtc-*`)
- The Coder and WP Validator share the same worktree. The per-MT stop pattern ensures only one role is active at a time.

## Safety: Data-Loss Prevention (HARD RULE)

- Same rules as VALIDATOR_PROTOCOL: no destructive commands without explicit operator authorization.
- WP Validator operates in the coder worktree (`wtc-*`) with read access for review purposes.
- WP Validator MUST NOT modify files in the coder worktree directly.

## Conversation Memory (MUST — `just repomem`)

Cross-session conversational memory captures what was reviewed, decided, and flagged during validation. All WP Validator sessions MUST use repomem:
- **SESSION_OPEN (MUST):** After startup, run `just repomem open "<what this validation session covers>" --role WP_VALIDATOR --wp WP-{ID}`. Blocked from mutation commands until done.
- **PRE_TASK before review execution (SHOULD):** Before starting a substantive MT review, rerunning a failed check bundle, or issuing a validator response, run `just repomem pre "<what review action is about to run and why>" --wp WP-{ID}`.
- **INSIGHT after discoveries (MUST):** When review reveals a non-obvious issue — a hidden coupling, a missing edge case, a pattern violation: `just repomem insight "<what was found>"`. Min 80 chars.
- **DECISION when accepting or rejecting (SHOULD):** When you pass or fail a microtask review, record the reasoning: `just repomem decision "<verdict and why>" --wp WP-{ID}`. Min 80 chars. This is the only durable record of validation judgment beyond the receipt.
- **ERROR when validation tooling breaks (SHOULD):** When a check fails to run, a file is missing, or the review context is broken: `just repomem error "<what went wrong>" --wp WP-{ID}`. Fast capture (min 40 chars).
- **CONCERN when flagging scope or quality risks (SHOULD):** When you spot a boundary violation, scope spill, missing test, or quality concern that may not warrant a FAIL but needs tracking: `just repomem concern "<risk flagged>" --wp WP-{ID}`. Min 80 chars. These are included in the terminal Workflow Dossier diagnostic snapshot at closeout.
- **ESCALATION when the verdict is unclear (SHOULD):** When the MT is ambiguous, the spec is contradictory, or you need orchestrator/operator judgment: `just repomem escalation "<what needs resolution>" --wp WP-{ID}`. Fast capture (min 40 chars).
- **SESSION_CLOSE (MUST):** Before session ends: `just repomem close "<what was reviewed, outcome>" --decisions "<key judgments made>"`.
- WP-bound repomem checkpoints are appended to the Workflow Dossier as a terminal diagnostic snapshot during closeout; import debt is diagnostic only, so do not maintain a parallel live dossier narrative for the same findings.

## Fail Capture

- WP Validator sessions MUST use `registerFailCaptureHook` and `failWithMemory` from `fail-capture-lib.mjs`.
- Boundary violations and scope spills are captured to governance memory for future session priming.

## Governance Surface Reduction Discipline

- WP validation should stay centered on the per-MT review boundary, packet truth, and runtime receipts rather than a widening set of review-adjacent public helpers.
- When deterministic review-side checks usually run together for the same MT boundary, consolidate them behind the canonical review bundle and primary debug artifact instead of adding more leaf commands or scripts.
- Keep separate public WP Validator surfaces only when authority ownership, side-effect class, runtime/topology assumptions, primary debug artifact, or operator usefulness materially differs.
- If a new live WP-validator governance surface is genuinely required, record why the existing surface is insufficient, who owns the new surface, what the primary debug artifact is, and whether an older surface is being retired or intentionally kept distinct.
