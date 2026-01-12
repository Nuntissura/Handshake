# 04) Small-Context Handoff (Kernel)

This kernel is designed so that **work survives model swaps**:
- small-context local models
- large-context cloud models
- human handoffs

The mechanism is not “remembering chat”. It is **artifact-first continuity**.

## 1. Principle: chat is not state

Kernel rule:
- Any decision that affects scope, requirements, safety, or acceptance MUST be written into a governance artifact (packet/refinement/board/audit log).

Rationale:
- Small models cannot hold long chat context.
- Chat logs are not reliably searchable/structured for mechanical auditing.

## 2. Deterministic “minimum context bundle” per role

A fresh agent should be able to start by opening a small, stable set of files.

Recommended minimum set:
- Operator: `docs/TASK_BOARD.md` + the active packet(s) being approved.
- Orchestrator: `docs/SPEC_CURRENT.md`, `docs/TASK_BOARD.md`, `docs/WP_TRACEABILITY_REGISTRY.md`, refinement + packet templates, and the target WP refinement/packet.
- Coder: the activated packet + the referenced refinement + the in-scope code paths.
- Validator: activated packet + refinement + spec target resolved + changed files + CI/test outputs + validator gate state.

## 3. Packetization as context compression

The task packet is the primary context compressor. To support small-context models, the packet MUST include:
- `FILES_TO_OPEN`: the exact files the model must read (ordered).
- `SEARCH_TERMS`: stable grep terms to find key anchors in code.
- `RUN_COMMANDS`: exact commands (or “none”).
- `RISK_MAP`: “risk → impact” mapping to guide cautious behavior.
- `DONE_MEANS` + `TEST_PLAN`: to prevent “looks good” completion claims.

Kernel intent:
- A coder should never need to re-open the entire Master Spec to start.
- A validator should never need to infer scope from commits.

## 4. Refinement anchors as “spec sharding”

Large specs do not fit in small contexts. The refinement solves this by:
- Binding the WP to a specific spec version (`SPEC_TARGET_SHA1`).
- Providing one or more anchors with:
  - start/end line window
  - context token that must exist in-window
  - excerpt captured as ASCII

Effect:
- A small model can prove it is reading the right part of the spec without ingesting the whole document.

## 5. Decomposing large work into internal sub-tasks (recommended method)

Yes: complex tasks should be decomposed, but the decomposition must be **artifact-backed**.

Kernel method:
1. Orchestrator creates a single WP refinement + packet with an explicit “internal milestones” list inside `## SKELETON` or `## IMPLEMENTATION` as a checklist.
2. Each milestone has:
   - explicit in-scope files
   - local acceptance criteria
   - the evidence that must be produced (logs, manifests, screenshots, etc.)
3. After each milestone, the coder updates `## STATUS_HANDOFF` with:
   - current milestone
   - what changed (file list)
   - next command to run
   - any hazards discovered
4. Validator reviews milestone-by-milestone, appending official notes in `## VALIDATION_REPORTS`.

Alternative (when milestones exceed packet size or become independent):
- Split into multiple WPs. Each WP must still be independently refinable, signable, and gate-checkable.

## 6. Context continuity across model swaps (mechanical)

When swapping models/roles mid-flight, the outgoing agent must leave:
- Updated `docs/TASK_BOARD.md` status (if the project uses it as SSoT).
- Updated packet `## STATUS_HANDOFF` (single place to read “where we are”).
- Updated `## EVIDENCE` (copy/paste logs; avoid “ran tests locally, trust me”).
- Completed manifests in `## VALIDATION` for any changed files.

Incoming agent procedure (deterministic):
1. Open the packet and read `## STATUS_HANDOFF`.
2. Open the refinement and confirm:
   - correct WP_ID
   - signature present
   - anchors exist and match the current spec target
3. Run the pre-work gate before doing anything (or verify it was run and the inputs are unchanged).

## 7. Why “heavy thinking” is not the primary control surface

The kernel assumes model capability varies. Therefore:
- correctness is enforced by gates + explicit artifacts, not by model “memory”
- reasoning strength is captured as a declared field (e.g., `CODER_REASONING_STRENGTH`) for risk management, not as a substitute for evidence

Practical guidance:
- For strict governance with large specs, a standard “heavy reasoning” model is usually sufficient because artifacts bound scope and anchors shard context.
- Extra-heavy reasoning helps during refinement and cross-artifact drift detection, but it should not replace mechanical verification.

